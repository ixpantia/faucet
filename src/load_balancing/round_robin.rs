use super::LoadBalancingStrategy;
use crate::client::Client;
use crate::error::FaucetResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::AtomicUsize;
use tokio::sync::Mutex;

struct Targets {
    targets: Box<[Client]>,
    index: AtomicUsize,
}

impl Targets {
    fn new(targets: impl AsRef<[SocketAddr]>) -> FaucetResult<Self> {
        let targets = targets
            .as_ref()
            .iter()
            .map(|addr| Client::builder(*addr).build())
            .collect::<FaucetResult<Box<[Client]>>>()?;
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

pub struct RoundRobinSimple {
    targets: Targets,
}

impl RoundRobinSimple {
    pub(crate) fn new(targets: impl AsRef<[SocketAddr]>) -> FaucetResult<Self> {
        Ok(Self {
            targets: Targets::new(targets)?,
        })
    }
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobinSimple {
    async fn entry(&self, _ip: IpAddr) -> Client {
        self.targets.next()
    }
}

pub struct RoundRobinIpHash {
    targets: Targets,
    table: Mutex<HashMap<IpAddr, Client>>,
}

impl RoundRobinIpHash {
    pub(crate) fn new(targets: impl AsRef<[SocketAddr]>) -> FaucetResult<Self> {
        Ok(Self {
            targets: Targets::new(targets)?,
            table: Mutex::new(HashMap::new()),
        })
    }
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobinIpHash {
    async fn entry(&self, ip: IpAddr) -> Client {
        let mut table = self.table.lock().await;
        table
            .entry(ip)
            .or_insert_with(|| self.targets.next())
            .clone()
    }
}
