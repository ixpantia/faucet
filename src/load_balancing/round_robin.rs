use async_trait::async_trait;

use super::LoadBalancingStrategy;
use crate::client::Client;
use crate::error::FaucetResult;
use crate::worker::WorkerState;
use std::net::IpAddr;
use std::sync::atomic::AtomicUsize;

struct Targets {
    targets: &'static [Client],
    index: AtomicUsize,
}

// 500us is the time it takes for the round robin to move to the next target
// in the unlikely event that the target is offline
const WAIT_TIME_UNTIL_RETRY: std::time::Duration = std::time::Duration::from_micros(500);

impl Targets {
    fn new(workers_state: &[WorkerState]) -> FaucetResult<Self> {
        let mut targets = Vec::new();
        for state in workers_state {
            let client = Client::builder(state.clone()).build()?;
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
    pub(crate) fn new(targets: &[WorkerState]) -> FaucetResult<Self> {
        Ok(Self {
            targets: Targets::new(targets)?,
        })
    }
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobin {
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

    use std::sync::{atomic::AtomicBool, Arc};

    use super::*;

    #[test]
    fn test_new_targets() {
        let mut workers_state = Vec::new();
        for i in 0..3 {
            let is_online = Arc::new(AtomicBool::new(true));
            let socket_addr = format!("127.0.0.1:900{}", i)
                .parse()
                .expect("failed to parse socket addr");
            let state = WorkerState::new("test", is_online, socket_addr);
            workers_state.push(state);
        }

        let _ = Targets::new(&workers_state).expect("failed to create targets");
    }

    #[test]
    fn test_new_round_robin() {
        let mut workers_state = Vec::new();
        for i in 0..3 {
            let is_online = Arc::new(AtomicBool::new(true));
            let socket_addr = format!("127.0.0.1:900{}", i)
                .parse()
                .expect("failed to parse socket addr");
            let state = WorkerState::new("test", is_online, socket_addr);
            workers_state.push(state);
        }

        let _ = RoundRobin::new(&workers_state).expect("failed to create round robin");
    }

    #[tokio::test]
    async fn test_round_robin_entry() {
        use crate::client::ExtractSocketAddr;

        let mut workers_state = Vec::new();
        for i in 0..3 {
            let is_online = Arc::new(AtomicBool::new(true));
            let socket_addr = format!("127.0.0.1:900{}", i)
                .parse()
                .expect("failed to parse socket addr");
            let state = WorkerState::new("test", is_online, socket_addr);
            workers_state.push(state);
        }

        let rr = RoundRobin::new(&workers_state).expect("failed to create round robin");

        let ip = "0.0.0.0".parse().expect("failed to parse ip");

        assert_eq!(
            rr.entry(ip).await.socket_addr(),
            workers_state[0].socket_addr()
        );
        assert_eq!(
            rr.entry(ip).await.socket_addr(),
            workers_state[1].socket_addr()
        );
        assert_eq!(
            rr.entry(ip).await.socket_addr(),
            workers_state[2].socket_addr()
        );
        assert_eq!(
            rr.entry(ip).await.socket_addr(),
            workers_state[0].socket_addr()
        );
        assert_eq!(
            rr.entry(ip).await.socket_addr(),
            workers_state[1].socket_addr()
        );
        assert_eq!(
            rr.entry(ip).await.socket_addr(),
            workers_state[2].socket_addr()
        );
    }

    #[tokio::test]
    async fn test_round_robin_entry_with_offline_target() {
        use crate::client::ExtractSocketAddr;

        let workers_state = [
            WorkerState::new(
                "test",
                Arc::new(AtomicBool::new(false)),
                "127.0.0.1:9000"
                    .parse()
                    .expect("failed to parse socket addr"),
            ),
            WorkerState::new(
                "test",
                Arc::new(AtomicBool::new(false)),
                "127.0.0.1:9001"
                    .parse()
                    .expect("failed to parse socket addr"),
            ),
            WorkerState::new(
                "test",
                Arc::new(AtomicBool::new(true)),
                "127.0.0.1:9002"
                    .parse()
                    .expect("failed to parse socket addr"),
            ),
        ];

        let rr = RoundRobin::new(&workers_state).expect("failed to create round robin");

        let ip = "0.0.0.0".parse().expect("failed to parse ip");

        assert_eq!(
            rr.entry(ip).await.socket_addr(),
            workers_state[2].socket_addr()
        );
    }
}
