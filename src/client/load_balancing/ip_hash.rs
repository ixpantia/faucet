use super::LoadBalancingStrategy;
use super::WorkerState;
use crate::{client::Client, error::FaucetResult};
use async_trait::async_trait;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::time::Duration;

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

            log::debug!(
                target: "faucet",
                "IP {} tried to connect to offline {}, retrying in {:?}",
                ip,
                client.target(),
                backoff
            );

            tokio::time::sleep(backoff).await;
            retries += 1;
        }
    }
}

#[cfg(test)]
mod tests {

    use std::sync::{atomic::AtomicBool, Arc};

    use super::*;

    #[test]
    fn test_hash_to_index() {
        let index = hash_to_index("test", 10);
        assert!(index < 10);
    }

    #[test]
    fn test_hash_to_index_same() {
        let index = hash_to_index("test", 10);
        let index2 = hash_to_index("test", 10);
        assert_eq!(index, index2);
    }

    #[test]
    fn test_hash_to_index_different() {
        let index = hash_to_index("test", 10);
        let index2 = hash_to_index("test2", 10);
        assert_ne!(index, index2);
    }

    #[test]
    fn test_hash_to_index_different_length() {
        let index = hash_to_index("test", 10);
        let index2 = hash_to_index("test", 3);
        assert_ne!(index, index2);
    }

    #[test]
    fn test_new_targets() {
        let worker_state = WorkerState {
            target: "test",
            is_online: Arc::new(AtomicBool::new(true)),
            socket_addr: "127.0.0.1:9999".parse().unwrap(),
        };
        let Targets { targets } = Targets::new(&[worker_state]).unwrap();

        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_new_ip_hash() {
        let worker_state = WorkerState {
            target: "test",
            is_online: Arc::new(AtomicBool::new(true)),
            socket_addr: "127.0.0.1:9999".parse().unwrap(),
        };
        let IpHash {
            targets,
            targets_len,
        } = IpHash::new(&[worker_state]).unwrap();

        assert_eq!(targets.targets.len(), 1);
        assert_eq!(targets_len, 1);
    }

    #[test]
    fn test_calculate_exponential_backoff() {
        assert_eq!(calculate_exponential_backoff(0), BASE_BACKOFF);
        assert_eq!(calculate_exponential_backoff(1), BASE_BACKOFF * 2);
        assert_eq!(calculate_exponential_backoff(2), BASE_BACKOFF * 4);
        assert_eq!(calculate_exponential_backoff(3), BASE_BACKOFF * 8);
    }

    #[tokio::test]
    async fn test_load_balancing_strategy() {
        use crate::client::ExtractSocketAddr;
        let workers = [
            WorkerState {
                target: "test",
                is_online: Arc::new(AtomicBool::new(true)),
                socket_addr: "127.0.0.1:9999".parse().unwrap(),
            },
            WorkerState {
                target: "test",
                is_online: Arc::new(AtomicBool::new(true)),
                socket_addr: "127.0.0.1:8888".parse().unwrap(),
            },
        ];
        let ip_hash = IpHash::new(&workers).unwrap();
        let client1 = ip_hash.entry("192.168.0.1".parse().unwrap()).await;
        let client2 = ip_hash.entry("192.168.0.1".parse().unwrap()).await;
        assert_eq!(client1.socket_addr(), client2.socket_addr());

        // This IP address should hash to a different index
        let client3 = ip_hash.entry("192.168.0.43".parse().unwrap()).await;
        let client4 = ip_hash.entry("192.168.0.43".parse().unwrap()).await;

        assert_eq!(client3.socket_addr(), client4.socket_addr());

        assert_ne!(client1.socket_addr(), client3.socket_addr());
    }

    #[tokio::test]
    async fn test_load_balancing_strategy_offline() {
        use crate::client::ExtractSocketAddr;

        let online = Arc::new(AtomicBool::new(false));
        let worker = WorkerState {
            target: "test",
            is_online: online.clone(),
            socket_addr: "127.0.0.1:9999".parse().unwrap(),
        };

        let ip_hash = IpHash::new(&[worker]).unwrap();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            online.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let entry = ip_hash.entry("192.168.0.1".parse().unwrap()).await;

        assert_eq!(entry.socket_addr(), "127.0.0.1:9999".parse().unwrap());
    }
}
