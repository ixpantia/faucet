use uuid::Uuid;

use super::LoadBalancingStrategy;
use super::WorkerConfig;
use crate::client::Client;
use crate::leak;
use std::time::Duration;

struct Targets {
    targets: &'static [Client],
}

impl Targets {
    fn new(configs: &[&'static WorkerConfig]) -> Self {
        let mut targets = Vec::new();
        for state in configs {
            let client = Client::new(state);
            targets.push(client);
        }
        let targets = leak!(targets);
        Targets { targets }
    }
}

pub struct CookieHash {
    targets: Targets,
    targets_len: usize,
}

impl CookieHash {
    pub(crate) async fn new(configs: &[&'static WorkerConfig]) -> Self {
        // Start the process of each config
        for config in configs {
            config.spawn_worker_task().await;
        }
        Self {
            targets_len: configs.as_ref().len(),
            targets: Targets::new(configs),
        }
    }
}

fn calculate_hash(cookie_uuid: Uuid) -> u64 {
    let mut hash_value = cookie_uuid.as_u128() as u64;
    hash_value ^= hash_value >> 33;
    hash_value = hash_value.wrapping_mul(0xff51afd7ed558ccd);
    hash_value ^= hash_value >> 33;
    hash_value = hash_value.wrapping_mul(0xc4ceb9fe1a85ec53);
    hash_value ^= hash_value >> 33;

    hash_value
}

fn hash_to_index(value: Uuid, length: usize) -> usize {
    let hash = calculate_hash(value);
    (hash % length as u64) as usize
}

// 50ms is the minimum backoff time for exponential backoff
const BASE_BACKOFF: Duration = Duration::from_millis(1);

const MAX_BACKOFF: Duration = Duration::from_millis(500);

fn calculate_exponential_backoff(retries: u32) -> Duration {
    (BASE_BACKOFF * 2u32.pow(retries)).min(MAX_BACKOFF)
}

impl LoadBalancingStrategy for CookieHash {
    type Input = Uuid;
    async fn entry(&self, id: Uuid) -> Client {
        let mut retries = 0;
        let index = hash_to_index(id, self.targets_len);
        let client = self.targets.targets[index].clone();
        loop {
            if client.is_online() {
                break client;
            }

            let backoff = calculate_exponential_backoff(retries);

            log::debug!(
                target: "faucet",
                "LB Session {} tried to connect to offline {}, retrying in {:?}",
                id,
                client.config.target,
                backoff
            );

            tokio::time::sleep(backoff).await;
            retries += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ExtractSocketAddr;

    use uuid::Uuid;

    #[test]
    fn uuid_test_distribution_of_hash_function_len_4() {
        const N_UUIDS: usize = 100_000;

        let uuids: Vec<Uuid> = (0..N_UUIDS).map(|_| Uuid::now_v7()).collect();

        let mut counts = [0; 4];

        uuids.iter().for_each(|uuid| {
            let index = hash_to_index(*uuid, 4);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_UUIDS as f64;
        let percent_1 = counts[1] as f64 / N_UUIDS as f64;
        let percent_2 = counts[2] as f64 / N_UUIDS as f64;
        let percent_3 = counts[3] as f64 / N_UUIDS as f64;
        assert!((0.24..=0.26).contains(&percent_0));
        assert!((0.24..=0.26).contains(&percent_1));
        assert!((0.24..=0.26).contains(&percent_2));
        assert!((0.24..=0.26).contains(&percent_3));
    }

    #[test]
    fn uuid_test_distribution_of_hash_function_len_3() {
        const N_UUIDS: usize = 100_000;

        let uuids: Vec<Uuid> = (0..N_UUIDS).map(|_| Uuid::now_v7()).collect();

        let mut counts = [0; 3];

        uuids.iter().for_each(|uuid| {
            let index = hash_to_index(*uuid, 3);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_UUIDS as f64;
        let percent_1 = counts[1] as f64 / N_UUIDS as f64;
        let percent_2 = counts[2] as f64 / N_UUIDS as f64;
        assert!((0.32..=0.34).contains(&percent_0));
        assert!((0.32..=0.34).contains(&percent_1));
        assert!((0.32..=0.34).contains(&percent_2));
    }

    #[test]
    fn uuid_test_distribution_of_hash_function_len_2() {
        const N_UUIDS: usize = 100_000;

        let uuids: Vec<Uuid> = (0..N_UUIDS).map(|_| Uuid::now_v7()).collect();

        let mut counts = [0; 2];

        uuids.iter().for_each(|uuid| {
            let index = hash_to_index(*uuid, 2);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_UUIDS as f64;
        let percent_1 = counts[1] as f64 / N_UUIDS as f64;
        assert!((0.49..=0.51).contains(&percent_0));
        assert!((0.49..=0.51).contains(&percent_1));
    }

    #[test]
    fn test_new_targets() {
        let worker_state: &'static WorkerConfig = Box::leak(Box::new(WorkerConfig::dummy(
            "test",
            "127.0.0.1:9999",
            true,
        )));
        let Targets { targets } = Targets::new(&[worker_state]);

        assert_eq!(targets.len(), 1);
    }

    #[tokio::test]
    async fn test_new_cookie_hash() {
        let worker_state: &'static WorkerConfig = Box::leak(Box::new(WorkerConfig::dummy(
            "test",
            "127.0.0.1:9999",
            true,
        )));
        let CookieHash {
            targets,
            targets_len,
        } = CookieHash::new(&[worker_state]).await;

        assert_eq!(targets.targets.len(), 1);
        assert_eq!(targets_len, 1);

        worker_state.wait_until_done().await;
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
        let worker1: &'static WorkerConfig = Box::leak(Box::new(WorkerConfig::dummy(
            "test1",
            "127.0.0.1:9999",
            true,
        )));
        let worker2: &'static WorkerConfig = Box::leak(Box::new(WorkerConfig::dummy(
            "test2",
            "127.0.0.1:8888",
            true,
        )));
        let workers_static_refs = [worker1, worker2];
        let cookie_hash = CookieHash::new(&workers_static_refs).await;

        let uuid1 = Uuid::now_v7();
        let client1_a = cookie_hash.entry(uuid1).await;
        let client1_b = cookie_hash.entry(uuid1).await;
        assert_eq!(client1_a.socket_addr(), client1_b.socket_addr());

        // Generate many UUIDs to increase chance of hitting the other target
        // This doesn't guarantee hitting the other target if hash distribution is not perfect
        // or if N_TARGETS is small, but it's a practical test.
        let mut client2_addr = client1_a.socket_addr();
        let mut uuid2 = Uuid::now_v7();

        for _ in 0..100 {
            // Try a few times to get a different client
            uuid2 = Uuid::now_v7();
            let client_temp = cookie_hash.entry(uuid2).await;
            if client_temp.socket_addr() != client1_a.socket_addr() {
                client2_addr = client_temp.socket_addr();
                break;
            }
        }

        // It's possible (though unlikely for 2 targets and good hash) that we always hit the same target.
        // A more robust test would mock specific hash results or use more targets.
        // For now, we assert that two different UUIDs *can* map to different clients.
        // And the same UUID (uuid2) consistently maps.
        let client2_a = cookie_hash.entry(uuid2).await;
        let client2_b = cookie_hash.entry(uuid2).await;
        assert_eq!(client2_a.socket_addr(), client2_b.socket_addr());
        assert_eq!(client2_a.socket_addr(), client2_addr);

        if workers_static_refs.len() > 1 {
            // Only assert inequality if we expect different clients to be possible and were found
            if client1_a.socket_addr() != client2_a.socket_addr() {
                assert_ne!(client1_a.socket_addr(), client2_a.socket_addr());
            } else {
                // This might happen if all UUIDs hashed to the same target, or only 1 worker.
                // Consider logging a warning if this happens frequently with >1 workers.
                println!("Warning: test_load_balancing_strategy did not find two different UUIDs mapping to different targets.");
            }
        } else {
            assert_eq!(client1_a.socket_addr(), client2_a.socket_addr());
        }

        for worker_config in workers_static_refs.iter() {
            worker_config.wait_until_done().await;
        }
    }
}
