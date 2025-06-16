use rand::Rng;
use tokio::sync::Mutex;

use super::LoadBalancingStrategy;
use crate::client::{worker::WorkerConfig, Client};
use std::net::IpAddr;

struct RequestCounter {
    last_reset: std::time::Instant,
    current_window: f64,
    previous_window_rps: f64,
    big_reset_counter: f64,
    pub max_rps: f64,
}

const WINDOW_SIZE: f64 = 10.0; // seconds
const BIG_RESET_WINDOW_SIZE: f64 = 30.0; // seconds

impl RequestCounter {
    fn new(max_rps: f64) -> Self {
        RequestCounter {
            last_reset: std::time::Instant::now(),
            current_window: 0.0,
            previous_window_rps: 0.0,
            big_reset_counter: 0.0,
            max_rps,
        }
    }
    fn add(&mut self, count: f64) {
        self.current_window += count;
        self.big_reset_counter += count;
    }
    fn set_new_window(&mut self) -> f64 {
        let elapsed = self.last_reset.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        let previous_window_rps = if elapsed_secs > 0.0 {
            self.current_window / elapsed_secs
        } else {
            // Avoid division by zero if elapsed time is extremely small
            // Treat as very high RPS if there were any requests
            if self.current_window > 0.0 {
                f64::MAX
            } else {
                0.0
            }
        };

        self.previous_window_rps = previous_window_rps;
        self.last_reset = std::time::Instant::now();
        self.current_window = 0.0;
        previous_window_rps
    }
    fn rps(&mut self) -> f64 {
        let elapsed_secs = self.last_reset.elapsed().as_secs_f64();
        if elapsed_secs > 0.0 {
            self.current_window / elapsed_secs
        } else {
            match self.current_window > 0.0 {
                true => f64::MAX,
                false => 0.0,
            }
        }
    }
    fn total_requests_since_big_reset(&mut self) -> f64 {
        self.big_reset_counter
            + self.previous_window_rps * self.last_reset.elapsed().as_secs_f64()
            + self.current_window
    }
    fn reset_big(&mut self) {
        self.big_reset_counter = 0.0;
    }
}

struct Targets {
    targets: &'static [Client],
    request_counter: &'static [Mutex<RequestCounter>],
    _request_counter_calculator_handle: tokio::task::JoinHandle<()>,
}

const WAIT_TIME_UNTIL_RETRY: std::time::Duration = std::time::Duration::from_millis(500);

impl Targets {
    fn new(configs: &[&'static WorkerConfig], max_rps: f64) -> Self {
        let mut targets_vec = Vec::new();
        let mut request_counters_vec = Vec::new();
        for config in configs {
            let client = Client::new(config);
            targets_vec.push(client);
            request_counters_vec.push(Mutex::new(RequestCounter::new(max_rps)));
        }
        let targets = Box::leak(targets_vec.into_boxed_slice()) as &'static [Client];
        let request_counter_static_slice = Box::leak(request_counters_vec.into_boxed_slice())
            as &'static [Mutex<RequestCounter>];

        let request_per_second_calculator_handle = tokio::spawn(async move {
            let mut last_big_reset_time = std::time::Instant::now();
            loop {
                tokio::time::sleep(std::time::Duration::from_secs_f64(WINDOW_SIZE)).await;

                let is_big_reset_due =
                    last_big_reset_time.elapsed().as_secs_f64() >= BIG_RESET_WINDOW_SIZE;

                if is_big_reset_due {
                    last_big_reset_time = std::time::Instant::now();
                }

                for i in 0..targets.len() {
                    let mut rc_guard = request_counter_static_slice[i].lock().await;
                    let calculated_rps = rc_guard.set_new_window();

                    if calculated_rps > rc_guard.max_rps {
                        log::debug!(
                            target: "faucet",
                            "Target {} ({}) is overloaded ({} RPS), attempting to spawn worker for next target",
                            i, targets[i].config.target, calculated_rps
                        );
                        match targets.get(i + 1) {
                            Some(next_target_client) => {
                                log::info!(
                                    target: "faucet",
                                    "Spawning worker task for adjacent target {} due to overload on target {}",
                                    next_target_client.config.target, targets[i].config.target
                                );
                                next_target_client.config.spawn_worker_task().await;
                            }
                            _ if targets.len() == 1 => {
                                log::warn!(
                                    target: "faucet",
                                    "Target {} is overloaded but it's the only target. No autoscaling action possible for spawning.",
                                    targets[i].config.target
                                );
                            }
                            _ => (),
                        }
                    }

                    if is_big_reset_due {
                        let total_requests = rc_guard.total_requests_since_big_reset();

                        if total_requests == 0.0 {
                            // Check if the worker is actually running before trying to stop it.
                            // For dummy workers, handle might be None if never "spawned".
                            // If handle is Some, and not finished, then it's "running".
                            let is_running = targets[i]
                                .config
                                .handle
                                .lock()
                                .await
                                .as_ref()
                                .map_or_else(|| false, |h| !h.is_finished());
                            if is_running || targets[i].is_online() {
                                // is_online for initial state before handle is set
                                log::info!(
                                    target: "faucet",
                                    "Target {} ({}) has no requests in the last ~{} seconds, notifying idle stop.",
                                    i, targets[i].config.target, BIG_RESET_WINDOW_SIZE
                                );
                                targets[i].config.idle_stop.notify_waiters();
                            }
                        }
                        rc_guard.reset_big();
                    }
                }
            }
        });
        Targets {
            targets,
            request_counter: request_counter_static_slice,
            _request_counter_calculator_handle: request_per_second_calculator_handle,
        }
    }
    fn get(&self, index: usize) -> (Client, &'static Mutex<RequestCounter>) {
        (
            self.targets[index % self.targets.len()].clone(),
            &self.request_counter[index % self.targets.len()],
        )
    }
}

pub struct RpsAutoscale {
    targets: Targets,
}

impl RpsAutoscale {
    pub(crate) async fn new(configs: &[&'static WorkerConfig], max_rps: f64) -> Self {
        // Spawn initial worker tasks as per configs
        for config in configs {
            if config.is_online.load(std::sync::atomic::Ordering::SeqCst) {
                // If configured to be initially online
                config.spawn_worker_task().await;
            }
        }
        Self {
            targets: Targets::new(configs, max_rps),
        }
    }
}

impl LoadBalancingStrategy for RpsAutoscale {
    type Input = IpAddr;
    async fn entry(&self, _ip: IpAddr) -> Client {
        let len = self.targets.targets.len();
        if len == 0 {
            panic!("RpsAutoscale called with no targets!");
        }

        let mut passes = 0;
        let mut current_index; // Start at a random target

        loop {
            current_index = rand::rng().random_range(0..len);
            passes += 1;

            let (client, request_counter_mutex) = self.targets.get(current_index);

            let is_online = client.is_online();

            let mut rc_guard = match request_counter_mutex.try_lock() {
                Ok(rc) => rc,
                Err(_) => {
                    continue;
                }
            };

            if is_online && (rc_guard.rps() <= rc_guard.max_rps || passes > len) {
                rc_guard.add(1.0);
                return client;
            }

            if (passes > len * 2) && is_online {
                return client; // If we tried all once and this one is online, return it
            }

            if (passes > len * 5) && !is_online {
                log::warn!(target: "faucet", "Looped {} times, still no suitable target. Trying to spawn for target 0 if offline.", 5);
                client.config.spawn_worker_task().await;
                // Wait a bit for it to potentially come online
                for _ in 0..1000 {
                    // Try for up to 10 * WAIT_TIME_UNTIL_RETRY
                    tokio::time::sleep(WAIT_TIME_UNTIL_RETRY).await;
                    if client.is_online() {
                        rc_guard.add(1.0);
                        return client;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::worker::WorkerConfig; // WorkerType needed for dummy
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Notify; // Notify used in WorkerConfig::dummy

    // Helper to create &'static WorkerConfig using WorkerConfig::dummy
    fn create_leaked_dummy_config(
        id_prefix: &str,
        index: usize,
        initial_online: bool,
    ) -> &'static WorkerConfig {
        let target_name =
            Box::leak(format!("{}-{}", id_prefix, index).into_boxed_str()) as &'static str;
        let addr_str = format!("127.0.0.1:{}", 9500 + index); // Ensure unique ports for tests

        &*Box::leak(Box::new(WorkerConfig::dummy(
            target_name,
            &addr_str,
            initial_online,
        )))
    }

    fn dummy_ip() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)) // A typical private IP
    }

    #[tokio::test]
    async fn test_new_rps_autoscale() {
        let config1 = create_leaked_dummy_config("new", 0, true);
        let config2 = create_leaked_dummy_config("new", 1, true);
        let autoscale = RpsAutoscale::new(&[config1, config2], 10.0).await;
        assert_eq!(autoscale.targets.targets.len(), 2);
        // Drop the autoscale to allow its background task to be cleaned up if possible
        drop(autoscale);
    }

    #[tokio::test]
    async fn test_load_balancing_strategy_basic_entry() {
        let config1 = create_leaked_dummy_config("basic", 0, true);
        let autoscale = RpsAutoscale::new(&[config1], 10.0).await;
        let client = autoscale.entry(dummy_ip()).await;
        assert_eq!(client.config.target, config1.target);
        assert!(client.is_online());
        drop(autoscale);
    }

    #[tokio::test]
    async fn test_load_balancing_strategy_offline_target() {
        let config_offline = create_leaked_dummy_config("offline", 0, false);
        let config_online = create_leaked_dummy_config("offline", 1, true);
        let autoscale = RpsAutoscale::new(&[config_offline, config_online], 10.0).await;

        for _ in 0..5 {
            let client = autoscale.entry(dummy_ip()).await;
            assert_eq!(
                client.config.target, config_online.target,
                "Should pick the online target"
            );
            assert!(client.is_online());
        }
        drop(autoscale);
    }

    #[tokio::test]
    async fn test_load_balancing_overloaded_target_skipped_by_entry() {
        let config1 = create_leaked_dummy_config("overload", 0, true);
        let config2 = create_leaked_dummy_config("overload", 1, true);
        let autoscale = RpsAutoscale::new(&[config1, config2], 10.0).await;

        {
            let (_client1, rc1_mutex) = autoscale.targets.get(0);
            let mut rc1_guard = rc1_mutex.lock().await;

            rc1_guard.current_window = rc1_guard.max_rps * 5.0;
        }

        tokio::time::sleep(Duration::from_millis(10)).await; // Ensure a tiny bit of time has passed for rc1.last_reset

        let mut picked_config2 = false;
        for _ in 0..5 {
            let client = autoscale.entry(dummy_ip()).await;
            if client.config.target == config2.target {
                picked_config2 = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            picked_config2,
            "Load balancer should skip overloaded target config1 and pick config2"
        );

        drop(autoscale);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_autoscale_spawn_worker_on_overload_background_task() {
        let config0 = create_leaked_dummy_config("autospawn", 0, true); // Target to be overloaded
        let config1 = create_leaked_dummy_config("autospawn", 1, true); // Target whose worker should be "spawned"

        assert!(
            config1.handle.lock().await.is_none(),
            "Config1 handle should be None initially"
        );

        let autoscale = RpsAutoscale::new(&[config0, config1], 10.0).await;

        {
            let rc0_mutex = &autoscale.targets.request_counter[0];
            let mut rc0_guard = rc0_mutex.lock().await;
            rc0_guard.current_window = (rc0_guard.max_rps + 1.0) * WINDOW_SIZE;
        }

        let wait_duration = Duration::from_secs_f64(WINDOW_SIZE + 2.0);
        tokio::time::sleep(wait_duration).await;

        let config1_handle_lock = config1.handle.lock().await;
        assert!(config1_handle_lock.is_some(), "Worker handle for config1 should be set after simulated overload of config0 and background task execution.");

        drop(autoscale);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_autoscale_shutdown_idle_worker_background_task() {
        let config0 = create_leaked_dummy_config("autoshutdown", 0, true);
        // We need to ensure spawn_worker_task was called for config0 so it's considered "running"
        // RpsAutoscale::new calls spawn_worker_task for initially online workers.

        let autoscale = RpsAutoscale::new(&[config0], 10.0).await;

        // Wait for config0's handle to be set by RpsAutoscale::new
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            config0.handle.lock().await.is_some(),
            "Config0 handle should be set after RpsAutoscale::new"
        );

        let idle_stop_notification = Arc::new(Notify::new());
        let notification_clone = idle_stop_notification.clone();

        // Spawn a task to listen for the idle_stop notification from the config
        tokio::spawn(async move {
            config0.idle_stop.notified().await;
            notification_clone.notify_one();
        });

        let wait_duration = Duration::from_secs_f64(BIG_RESET_WINDOW_SIZE + WINDOW_SIZE + 5.0); // e.g., 30s + 10s + 5s = 45s

        log::debug!(target: "faucet_test", "Waiting for {:?} for idle shutdown test on target {}", wait_duration, config0.target);

        match tokio::time::timeout(wait_duration, idle_stop_notification.notified()).await {
            Ok(_) => {
                log::info!(target: "faucet_test", "Idle stop notification received for target {}", config0.target);
            }
            Err(_) => {
                panic!("Idle stop notification timed out for target {}. Worker was not shut down as expected.", config0.target);
            }
        }
        drop(autoscale);
    }
}
