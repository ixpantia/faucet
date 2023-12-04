use super::LoadBalancingStrategy;
use crate::client::Client;
use crate::error::FaucetResult;
use async_trait::async_trait;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
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
    targets_len: usize,
}

impl RoundRobinIpHash {
    pub(crate) fn new(targets: impl AsRef<[SocketAddr]>) -> FaucetResult<Self> {
        Ok(Self {
            targets_len: targets.as_ref().len(),
            targets: Targets::new(targets)?,
        })
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn hash_to_index(value: impl Hash, length: usize) -> usize {
    let hash = calculate_hash(&value);
    (hash % length as u64) as usize
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobinIpHash {
    async fn entry(&self, ip: IpAddr) -> Client {
        let index = hash_to_index(ip, self.targets_len);
        self.targets.targets[index].clone()
    }
}
