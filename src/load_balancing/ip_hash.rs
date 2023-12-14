use async_trait::async_trait;

use crate::worker::WorkerState;
use crate::{client::Client, error::FaucetResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;

use std::time::Duration;

use super::LoadBalancingStrategy;

struct Targets {
    targets: &'static [Client],
}

impl Targets {
    fn new(workers_state: &[WorkerState]) -> FaucetResult<Self> {
        let mut targets = Vec::new();
        for state in workers_state {
            let client = Client::builder(state.clone()).build()?;
            targets.push(client);
        }
        let targets = Box::leak(targets.into_boxed_slice());
        Ok(Targets { targets })
    }
}

pub struct IpHash {
    targets: Targets,
    targets_len: usize,
}

impl IpHash {
    pub(crate) fn new(targets: &[WorkerState]) -> FaucetResult<Self> {
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

// 50ms is the minimum backoff time for exponential backoff
const BASE_BACKOFF: Duration = Duration::from_millis(50);

fn calculate_exponential_backoff(retries: u32) -> Duration {
    BASE_BACKOFF * 2u32.pow(retries)
}

#[async_trait]
impl LoadBalancingStrategy for IpHash {
    async fn entry(&self, ip: IpAddr) -> Client {
        let mut retries = 0;
        let index = hash_to_index(ip, self.targets_len);
        let client = self.targets.targets[index].clone();
        loop {
            if client.is_online() {
                break client;
            }
            let backoff = calculate_exponential_backoff(retries);
            tokio::time::sleep(backoff).await;
            retries += 1;
        }
    }
}
