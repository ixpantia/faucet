use super::LoadBalancingStrategy;
use crate::client::{worker::WorkerConfig, Client};
use std::{
    net::IpAddr,
    sync::atomic::{AtomicI32, AtomicU64, AtomicUsize},
};

#[derive(Default)]
struct AtomicF64(AtomicU64);

impl AtomicF64 {
    fn new(value: f64) -> Self {
        Self(AtomicU64::new(value.to_bits()))
    }
    fn load(&self, order: std::sync::atomic::Ordering) -> f64 {
        f64::from_bits(self.0.load(order))
    }
    fn store(&self, value: f64, order: std::sync::atomic::Ordering) {
        self.0.store(value.to_bits(), order);
    }
    fn fetch_add(&self, value: f64, order: std::sync::atomic::Ordering) -> f64 {
        let old = self.load(order);
        let new = old + value;
        self.store(new, order);
        old
    }
    fn swap(&self, value: f64, order: std::sync::atomic::Ordering) -> f64 {
        let old = self.load(order);
        self.store(value, order);
        old
    }
}

#[derive(Default)]
struct RequestCounter {
    current_window: AtomicF64,
    previous_window: AtomicF64,
}

const WINDOW_SIZE: f64 = 0.5;

impl RequestCounter {
    fn add(&self, count: f64) {
        self.current_window
            .fetch_add(count, std::sync::atomic::Ordering::Relaxed);
    }
    fn set_new_window(&self) -> f64 {
        // This will reset the current window to 0 and set the previous window to the current value
        let requests = self
            .current_window
            .swap(0.0, std::sync::atomic::Ordering::SeqCst);
        self.previous_window
            .swap(requests, std::sync::atomic::Ordering::SeqCst)
    }
    fn rps(&self, window: f64) -> f64 {
        self.previous_window
            .load(std::sync::atomic::Ordering::SeqCst)
            / window
    }
}

struct Targets {
    targets: &'static [Client],
    index: AtomicUsize,
    request_counter: &'static [RequestCounter],
    request_counter_calculator_handle: tokio::task::JoinHandle<()>,
}

// 500us is the time it takes for the round robin to move to the next target
// in the unlikely event that the target is offline
const WAIT_TIME_UNTIL_RETRY: std::time::Duration = std::time::Duration::from_millis(500);

impl Targets {
    fn new(configs: &[&'static WorkerConfig]) -> Self {
        let mut targets = Vec::new();
        let mut request_last_5_seconds = Vec::new();
        for state in configs {
            let client = Client::new(state);
            targets.push(client);
            request_last_5_seconds.push(RequestCounter::default());
        }
        let targets = Box::leak(targets.into_boxed_slice()) as &'static [Client];
        let request_counter =
            Box::leak(request_last_5_seconds.into_boxed_slice()) as &'static [RequestCounter];
        let request_per_second_calculator_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(
                    (WINDOW_SIZE * 1000.0) as u64,
                ))
                .await;
                for i in 0..targets.len() {
                    if request_counter[i].set_new_window() / WINDOW_SIZE > 3.0 {
                        log::debug!(
                            target: "faucet",
                            "Target {} is overloaded, spawning worker task",
                            targets[i].config.addr
                        );
                        if let Some(next_target) = targets.get(i + 1) {
                            next_target.config.spawn_worker_task().await;
                        }
                    }
                }
            }
        });
        Targets {
            targets,
            request_counter,
            request_counter_calculator_handle: request_per_second_calculator_handle,
            index: AtomicUsize::new(0),
        }
    }
    fn get(&self, index: usize) -> (Client, &'static RequestCounter) {
        (
            self.targets[index % self.targets.len()].clone(),
            &self.request_counter[index % self.targets.len()],
        )
    }
}

pub struct RoundRobinRps {
    targets: Targets,
}

impl RoundRobinRps {
    pub(crate) async fn new(configs: &[&'static WorkerConfig]) -> Self {
        Self {
            targets: Targets::new(configs),
        }
    }
}

impl LoadBalancingStrategy for RoundRobinRps {
    type Input = IpAddr;
    async fn entry(&self, _ip: IpAddr) -> Client {
        let mut index = self
            .targets
            .index
            .load(std::sync::atomic::Ordering::Relaxed);
        let mut use_next_online_target = false;
        let mut is_first_round = true;

        loop {
            let (client, request_last_5_seconds) = self.targets.get(index);

            if request_last_5_seconds.rps(WINDOW_SIZE) > 3.0 && !use_next_online_target {
                // If the target is overloaded, skip it
                index += 1;

                if index >= self.targets.targets.len() {
                    index = 0;
                    self.targets
                        .index
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    use_next_online_target = true;
                }

                continue;
            }

            if client.is_online() {
                request_last_5_seconds.add(1.0);
                return client;
            }

            if !is_first_round {
                // Only when we have tried all targets and none are online
                // we will spawn a worker task manually
                client.config.spawn_worker_task().await;
                for _ in 0..1000 {
                    // Wait for the target to come online
                    tokio::time::sleep(WAIT_TIME_UNTIL_RETRY).await;
                    if client.is_online() {
                        request_last_5_seconds.add(1.0);
                        return client;
                    }
                }
            }

            if index >= self.targets.targets.len() {
                // If we have tried all targets, we can return the first one
                // that is online
                index = 0;
                is_first_round = false;
                continue;
            }

            index += 1;
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_new_targets() {
        let configs = (0..3)
            .map(|i| WorkerConfig::dummy("test", &format!("127.0.0.1:900{i}"), true))
            .collect::<Vec<_>>();

        let _ = Targets::new(&configs).expect("failed to create targets");
    }

    #[test]
    fn test_new_round_robin() {
        let configs = (0..3)
            .map(|i| WorkerConfig::dummy("test", &format!("127.0.0.1:900{i}"), true))
            .collect::<Vec<_>>();

        let _ = RoundRobinRps::new(&configs).expect("failed to create round robin");
    }

    #[tokio::test]
    async fn test_round_robin_entry() {
        use crate::client::ExtractSocketAddr;

        let configs = (0..3)
            .map(|i| WorkerConfig::dummy("test", &format!("127.0.0.1:900{i}"), true))
            .collect::<Vec<_>>();

        let rr = RoundRobinRps::new(&configs).expect("failed to create round robin");

        let ip = "0.0.0.0".parse().expect("failed to parse ip");

        assert_eq!(rr.entry(ip).await.socket_addr(), configs[0].addr);
        assert_eq!(rr.entry(ip).await.socket_addr(), configs[1].addr);
        assert_eq!(rr.entry(ip).await.socket_addr(), configs[2].addr);
        assert_eq!(rr.entry(ip).await.socket_addr(), configs[0].addr);
        assert_eq!(rr.entry(ip).await.socket_addr(), configs[1].addr);
        assert_eq!(rr.entry(ip).await.socket_addr(), configs[2].addr);
    }

    #[tokio::test]
    async fn test_round_robin_entry_with_offline_target() {
        use crate::client::ExtractSocketAddr;

        let configs = [
            WorkerConfig::dummy("test", "127.0.0.1:9000", false),
            WorkerConfig::dummy("test", "127.0.0.1:9001", false),
            WorkerConfig::dummy("test", "127.0.0.1:9002", true),
        ];

        let rr = RoundRobinRps::new(&configs).expect("failed to create round robin");

        let ip = "0.0.0.0".parse().expect("failed to parse ip");

        assert_eq!(rr.entry(ip).await.socket_addr(), configs[2].addr);
    }
}
