use super::LoadBalancingStrategy;
use crate::client::{worker::WorkerConfig, Client};
use std::{net::IpAddr, sync::atomic::AtomicUsize};

struct Targets {
    targets: &'static [Client],
    index: AtomicUsize,
}

// 500us is the time it takes for the round robin to move to the next target
// in the unlikely event that the target is offline
const WAIT_TIME_UNTIL_RETRY: std::time::Duration = std::time::Duration::from_micros(500);

impl Targets {
    fn new(configs: &[&'static WorkerConfig]) -> Self {
        let mut targets = Vec::new();
        for state in configs {
            let client = Client::new(state);
            targets.push(client);
        }
        let targets = Box::leak(targets.into_boxed_slice());
        Targets {
            targets,
            index: AtomicUsize::new(0),
        }
    }
    fn next(&self) -> Client {
        let index = self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.targets[index % self.targets.len()].clone()
    }
}

pub struct RoundRobin {
    targets: Targets,
}

impl RoundRobin {
    pub(crate) async fn new(configs: &[&'static WorkerConfig]) -> Self {
        // Start the process of each config
        for config in configs {
            config.spawn_worker_task().await;
        }
        Self {
            targets: Targets::new(configs),
        }
    }
}

impl LoadBalancingStrategy for RoundRobin {
    type Input = IpAddr;
    async fn entry(&self, _ip: IpAddr) -> Client {
        let mut client = self.targets.next();
        loop {
            if client.is_online() {
                break client;
            }
            tokio::time::sleep(WAIT_TIME_UNTIL_RETRY).await;
            client = self.targets.next();
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_new_targets() {
        let configs_static_refs: Vec<&'static WorkerConfig> = (0..3)
            .map(|i| {
                &*Box::leak(Box::new(WorkerConfig::dummy(
                    "test",
                    &format!("127.0.0.1:900{i}"),
                    true,
                )))
            })
            .collect();

        let _ = Targets::new(&configs_static_refs);
    }

    #[tokio::test]
    async fn test_new_round_robin() {
        let configs_static_refs: Vec<&'static WorkerConfig> = (0..3)
            .map(|i| {
                &*Box::leak(Box::new(WorkerConfig::dummy(
                    "test",
                    &format!("127.0.0.1:900{i}"),
                    true,
                )))
            })
            .collect();

        let _ = RoundRobin::new(&configs_static_refs).await;

        for config in configs_static_refs.iter() {
            config.wait_until_done().await;
        }
    }

    #[tokio::test]
    async fn test_round_robin_entry() {
        use crate::client::ExtractSocketAddr;

        let original_addrs: Vec<std::net::SocketAddr> = (0..3)
            .map(|i| {
                format!("127.0.0.1:900{i}")
                    .parse()
                    .expect("Failed to parse addr")
            })
            .collect();

        let configs_static_refs: Vec<&'static WorkerConfig> = (0..3)
            .map(|i| {
                &*Box::leak(Box::new(WorkerConfig::dummy(
                    "test",
                    &format!("127.0.0.1:900{i}"),
                    true,
                )))
            })
            .collect();

        let rr = RoundRobin::new(&configs_static_refs).await;

        let ip = "0.0.0.0".parse().expect("failed to parse ip");

        assert_eq!(rr.entry(ip).await.socket_addr(), original_addrs[0]);
        assert_eq!(rr.entry(ip).await.socket_addr(), original_addrs[1]);
        assert_eq!(rr.entry(ip).await.socket_addr(), original_addrs[2]);
        assert_eq!(rr.entry(ip).await.socket_addr(), original_addrs[0]);
        assert_eq!(rr.entry(ip).await.socket_addr(), original_addrs[1]);
        assert_eq!(rr.entry(ip).await.socket_addr(), original_addrs[2]);

        for config in configs_static_refs.iter() {
            config.wait_until_done().await;
        }
    }

    #[tokio::test]
    async fn test_round_robin_entry_with_offline_target() {
        use crate::client::ExtractSocketAddr;

        // Storing the target address for assertion, as the original WorkerConfig array is no longer directly used.
        let target_online_addr: std::net::SocketAddr = "127.0.0.1:9002".parse().unwrap();

        let configs_static_refs: [&'static WorkerConfig; 3] = [
            &*Box::leak(Box::new(WorkerConfig::dummy(
                "test",
                "127.0.0.1:9000",
                false,
            ))),
            &*Box::leak(Box::new(WorkerConfig::dummy(
                "test",
                "127.0.0.1:9001",
                false,
            ))),
            &*Box::leak(Box::new(WorkerConfig::dummy(
                "test",
                "127.0.0.1:9002",
                true,
            ))),
        ];

        let rr = RoundRobin::new(&configs_static_refs).await;

        let ip = "0.0.0.0".parse().expect("failed to parse ip");

        assert_eq!(rr.entry(ip).await.socket_addr(), target_online_addr);

        for config in configs_static_refs.iter() {
            config.wait_until_done().await;
        }
    }
}
