use async_trait::async_trait;

use crate::{client::Client, error::FaucetResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::net::SocketAddr;

use super::LoadBalancingStrategy;

struct Targets {
    targets: &'static [Client],
}

impl Targets {
    fn new(targets: impl AsRef<[SocketAddr]>) -> FaucetResult<Self> {
        let targets = targets
            .as_ref()
            .iter()
            .map(|addr| Client::builder(*addr).build())
            .collect::<FaucetResult<Box<[Client]>>>()?;
        let targets = Box::leak(targets);
        Ok(Targets { targets })
    }
}

pub struct IpHash {
    targets: Targets,
    targets_len: usize,
}

impl IpHash {
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
impl LoadBalancingStrategy for IpHash {
    async fn entry(&self, ip: IpAddr) -> Client {
        let index = hash_to_index(ip, self.targets_len);
        self.targets.targets[index].clone()
    }
}
