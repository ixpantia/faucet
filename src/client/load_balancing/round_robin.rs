use super::LoadBalancingStrategy;
use crate::{
    client::{worker::WorkerConfig, Client},
    error::FaucetResult,
};
use std::{net::IpAddr, sync::atomic::AtomicUsize};

struct Targets {
    targets: &'static [Client],
    index: AtomicUsize,
}

// 500us is the time it takes for the round robin to move to the next target
// in the unlikely event that the target is offline
const WAIT_TIME_UNTIL_RETRY: std::time::Duration = std::time::Duration::from_micros(500);

impl Targets {
    fn new(configs: &[WorkerConfig]) -> FaucetResult<Self> {
        let mut targets = Vec::new();
        for state in configs {
            let client = Client::builder(*state).build()?;
            targets.push(client);
        }
        let targets = Box::leak(targets.into_boxed_slice());
        Ok(Targets {
            targets,
            index: AtomicUsize::new(0),
        })
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
    pub(crate) fn new(targets: &[WorkerConfig]) -> FaucetResult<Self> {
        Ok(Self {
            targets: Targets::new(targets)?,
        })
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

        let _ = RoundRobin::new(&configs).expect("failed to create round robin");
    }

    #[tokio::test]
    async fn test_round_robin_entry() {
        use crate::client::ExtractSocketAddr;

        let configs = (0..3)
            .map(|i| WorkerConfig::dummy("test", &format!("127.0.0.1:900{i}"), true))
            .collect::<Vec<_>>();

        let rr = RoundRobin::new(&configs).expect("failed to create round robin");

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

        let rr = RoundRobin::new(&configs).expect("failed to create round robin");

        let ip = "0.0.0.0".parse().expect("failed to parse ip");

        assert_eq!(rr.entry(ip).await.socket_addr(), configs[2].addr);
    }
}
