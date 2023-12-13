use super::LoadBalancingStrategy;
use crate::client::Client;
use crate::error::FaucetResult;
use crate::worker::WorkerState;
use async_trait::async_trait;
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
    fn new(targets: &[WorkerState]) -> FaucetResult<Self> {
        let targets = targets
            .as_ref()
            .iter()
            .map(|state| Client::builder(state.clone()).build())
            .collect::<FaucetResult<Box<[Client]>>>()?;
        let targets = Box::leak(targets);
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
