use super::LoadBalancingStrategy;
use crate::client::Client;
use crate::error::FaucetResult;
use async_trait::async_trait;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::AtomicUsize;

struct Targets {
    targets: &'static [Client],
    index: AtomicUsize,
}

impl Targets {
    fn new(targets: impl AsRef<[SocketAddr]>) -> FaucetResult<Self> {
        let targets = targets
            .as_ref()
            .iter()
            .map(|addr| Client::builder(*addr).build())
            .collect::<FaucetResult<Box<[Client]>>>()?;
        let targets = Box::leak(targets);
        Ok(Targets {
            targets,
            index: AtomicUsize::new(0),
        })
    }
    fn next(&self) -> Client {
        let index = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.targets[index % self.targets.len()].clone()
    }
}

pub struct RoundRobin {
    targets: Targets,
}

impl RoundRobin {
    pub(crate) fn new(targets: impl AsRef<[SocketAddr]>) -> FaucetResult<Self> {
        Ok(Self {
            targets: Targets::new(targets)?,
        })
    }
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobin {
    async fn entry(&self, _ip: IpAddr) -> Client {
        self.targets.next()
    }
}
