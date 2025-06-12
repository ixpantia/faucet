use rand::Rng;
use tokio::sync::Mutex;

use super::LoadBalancingStrategy;
use crate::client::{worker::WorkerConfig, Client};
use std::{net::IpAddr, sync::atomic::AtomicUsize};

struct RequestCounter {
    last_reset: std::time::Instant,
    current_window: f64,
    previous_window_rps: f64,
    big_reset_counter: f64,
}

impl Default for RequestCounter {
    fn default() -> Self {
        RequestCounter {
            last_reset: std::time::Instant::now(),
            current_window: 0.0,
            previous_window_rps: 0.0,
            big_reset_counter: 0.0,
        }
    }
}

const WINDOW_SIZE: f64 = 10.0;
const BIG_RESET_WINDOW_SIZE: f64 = 30.0;
const MAX_REQUESTS_PER_SECOND: f64 = 10.0;

impl RequestCounter {
    fn add(&mut self, count: f64) {
        self.current_window += count;
        self.big_reset_counter += count;
    }
    fn set_new_window(&mut self) -> f64 {
        let elapsed = self.last_reset.elapsed();
        let previous_window_rps = self.current_window / elapsed.as_secs_f64();
        log::debug!(
            target: "faucet",
            "Setting new window: {} requests per second in the last {} seconds",
            previous_window_rps,
            elapsed.as_secs_f64()
        );
        self.previous_window_rps = previous_window_rps;
        self.last_reset = std::time::Instant::now();
        self.current_window = 0.0;
        previous_window_rps
    }
    fn rps(&mut self) -> f64 {
        self.current_window / self.last_reset.elapsed().as_secs_f64()
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
    index: AtomicUsize,
    _request_counter_calculator_handle: tokio::task::JoinHandle<()>,
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
            request_last_5_seconds.push(Mutex::new(RequestCounter::default()));
        }
        let targets = Box::leak(targets.into_boxed_slice()) as &'static [Client];
        let request_counter = Box::leak(request_last_5_seconds.into_boxed_slice())
            as &'static [Mutex<RequestCounter>];
        let request_per_second_calculator_handle = tokio::spawn(async move {
            let mut last_big_reset = std::time::Instant::now();
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(WINDOW_SIZE as u64)).await;

                let is_big_reset = last_big_reset.elapsed().as_secs_f64() > BIG_RESET_WINDOW_SIZE;

                if is_big_reset {
                    last_big_reset = std::time::Instant::now();
                }

                for i in 0..targets.len() {
                    let mut request_counter = request_counter[i].lock().await;
                    if request_counter.set_new_window() > MAX_REQUESTS_PER_SECOND {
                        log::debug!(
                            target: "faucet",
                            "Target {} is overloaded, spawning worker task",
                            targets[i].config.addr
                        );
                        if let Some(next_target) = targets.get(i + 1) {
                            next_target.config.spawn_worker_task().await;
                        }
                    }
                    if is_big_reset {
                        let total_requests = request_counter.total_requests_since_big_reset();

                        log::debug!(
                            target: "faucet",
                            "Target {} has {} requests in the last {BIG_RESET_WINDOW_SIZE} seconds",
                            targets[i].config.target,
                            total_requests
                        );

                        if total_requests == 0.0 {
                            if let Some(handle) = targets[i].config.handle.lock().await.as_ref() {
                                if !handle.is_finished() {
                                    log::debug!(
                                        target: "faucet",
                                        "Target {} has no requests in the last {BIG_RESET_WINDOW_SIZE} seconds, shutting down",
                                        targets[i].config.target
                                    );
                                    targets[i].config.idle_stop.notify_waiters();
                                }
                            }
                        }
                        request_counter.reset_big();
                    }
                }
            }
        });
        Targets {
            targets,
            request_counter,
            index: AtomicUsize::new(0),
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
    pub(crate) async fn new(configs: &[&'static WorkerConfig]) -> Self {
        Self {
            targets: Targets::new(configs),
        }
    }
}

impl LoadBalancingStrategy for RpsAutoscale {
    type Input = IpAddr;
    async fn entry(&self, _ip: IpAddr) -> Client {
        let len = self.targets.targets.len();
        let mut round = 0;
        let mut index = 0;
        let mut use_next_online_target = false;
        let mut is_third_round_plus = false;
        let mut biggest_online_index = 0;

        loop {
            let (client, request_counter) = self.targets.get(index);

            let is_online = client.is_online();

            if is_online {
                biggest_online_index = biggest_online_index.max(index);
            }

            let mut request_counter = match request_counter.try_lock() {
                Ok(rc) => rc,
                Err(_) => {
                    index += 1;
                    continue;
                }
            };

            if request_counter.rps() > MAX_REQUESTS_PER_SECOND && !use_next_online_target {
                // If the target is overloaded, skip it
                index += 1;

                if index >= len {
                    index = rand::thread_rng().gen_range(0..biggest_online_index);
                    use_next_online_target = true;
                }

                continue;
            }

            if is_online {
                request_counter.add(1.0);
                return client;
            }

            if is_third_round_plus {
                // Only when we have tried all targets and none are online
                // we will spawn a worker task manually
                client.config.spawn_worker_task().await;
                for _ in 0..1000 {
                    // Wait for the target to come online
                    tokio::time::sleep(WAIT_TIME_UNTIL_RETRY).await;
                    if client.is_online() {
                        request_counter.add(1.0);
                        return client;
                    }
                }
            }

            if index >= len {
                // If we have tried all targets, we can return the first one
                // that is online
                index = 0;

                round += 1;

                if round > 3 {
                    is_third_round_plus = true;
                }

                continue;
            }

            index += 1;
        }
    }
}
