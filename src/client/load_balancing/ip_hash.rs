use super::LoadBalancingStrategy;
use super::WorkerConfig;
use crate::leak;
use crate::{client::Client, error::FaucetResult};
use std::net::IpAddr;
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

pub struct IpHash {
    targets: Targets,
    targets_len: usize,
}

impl IpHash {
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

fn calculate_hash(ip: IpAddr) -> u64 {
    let mut hash_value = match ip {
        IpAddr::V4(ip) => ip.to_bits() as u64,
        IpAddr::V6(ip) => ip.to_bits() as u64,
    };
    hash_value ^= hash_value >> 33;
    hash_value = hash_value.wrapping_mul(0xff51afd7ed558ccd);
    hash_value ^= hash_value >> 33;
    hash_value = hash_value.wrapping_mul(0xc4ceb9fe1a85ec53);
    hash_value ^= hash_value >> 33;

    hash_value
}

fn hash_to_index(value: IpAddr, length: usize) -> usize {
    let hash = calculate_hash(value);
    (hash % length as u64) as usize
}

// 50ms is the minimum backoff time for exponential backoff
const BASE_BACKOFF: Duration = Duration::from_millis(50);

fn calculate_exponential_backoff(retries: u32) -> Duration {
    BASE_BACKOFF * 2u32.pow(retries)
}

impl LoadBalancingStrategy for IpHash {
    type Input = IpAddr;
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

    use std::sync::{atomic::AtomicBool, Arc};

    use super::*;

    #[test]
    fn ip_v4_test_distribution_of_hash_function_len_4() {
        const N_IP: usize = 100_000;

        // Generate 10_000 ip address and see the
        // distribution over diferent lengths
        let ips: Vec<IpAddr> = (0..N_IP)
            .map(|_| IpAddr::V4(std::net::Ipv4Addr::from_bits(rand::random::<u32>())))
            .collect();

        // Counts when length == 4
        let mut counts = [0; 4];

        ips.iter().for_each(|ip| {
            let index = hash_to_index(*ip, 4);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_IP as f64;
        let percent_1 = counts[1] as f64 / N_IP as f64;
        let percent_2 = counts[2] as f64 / N_IP as f64;
        let percent_3 = counts[3] as f64 / N_IP as f64;
        assert!((0.24..=0.26).contains(&percent_0));
        assert!((0.24..=0.26).contains(&percent_1));
        assert!((0.24..=0.26).contains(&percent_2));
        assert!((0.24..=0.26).contains(&percent_3));
    }

    #[test]
    fn ip_v4_test_distribution_of_hash_function_len_3() {
        const N_IP: usize = 100_000;

        // Generate 10_000 ip address and see the
        // distribution over diferent lengths
        let ips: Vec<IpAddr> = (0..N_IP)
            .map(|_| IpAddr::V4(std::net::Ipv4Addr::from_bits(rand::random::<u32>())))
            .collect();

        // Counts when length == 4
        let mut counts = [0; 3];

        ips.iter().for_each(|ip| {
            let index = hash_to_index(*ip, 3);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_IP as f64;
        let percent_1 = counts[1] as f64 / N_IP as f64;
        let percent_2 = counts[2] as f64 / N_IP as f64;
        assert!((0.32..=0.34).contains(&percent_0));
        assert!((0.32..=0.34).contains(&percent_1));
        assert!((0.32..=0.34).contains(&percent_2));
    }

    #[test]
    fn ip_v4_test_distribution_of_hash_function_len_2() {
        const N_IP: usize = 100_000;

        // Generate 10_000 ip address and see the
        // distribution over diferent lengths
        let ips: Vec<IpAddr> = (0..N_IP)
            .map(|_| IpAddr::V4(std::net::Ipv4Addr::from_bits(rand::random::<u32>())))
            .collect();

        // Counts when length == 4
        let mut counts = [0; 2];

        ips.iter().for_each(|ip| {
            let index = hash_to_index(*ip, 2);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_IP as f64;
        let percent_1 = counts[1] as f64 / N_IP as f64;
        assert!((0.49..=0.51).contains(&percent_0));
        assert!((0.49..=0.51).contains(&percent_1));
    }

    #[test]
    fn ip_v6_test_distribution_of_hash_function_len_4() {
        const N_IP: usize = 100_000;

        // Generate 10_000 ip address and see the
        // distribution over diferent lengths
        let ips: Vec<IpAddr> = (0..N_IP)
            .map(|_| IpAddr::V6(std::net::Ipv6Addr::from_bits(rand::random::<u128>())))
            .collect();

        // Counts when length == 4
        let mut counts = [0; 4];

        ips.iter().for_each(|ip| {
            let index = hash_to_index(*ip, 4);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_IP as f64;
        let percent_1 = counts[1] as f64 / N_IP as f64;
        let percent_2 = counts[2] as f64 / N_IP as f64;
        let percent_3 = counts[3] as f64 / N_IP as f64;
        assert!((0.24..=0.26).contains(&percent_0));
        assert!((0.24..=0.26).contains(&percent_1));
        assert!((0.24..=0.26).contains(&percent_2));
        assert!((0.24..=0.26).contains(&percent_3));
    }

    #[test]
    fn ip_v6_test_distribution_of_hash_function_len_3() {
        const N_IP: usize = 100_000;

        // Generate 10_000 ip address and see the
        // distribution over diferent lengths
        let ips: Vec<IpAddr> = (0..N_IP)
            .map(|_| IpAddr::V6(std::net::Ipv6Addr::from_bits(rand::random::<u128>())))
            .collect();

        // Counts when length == 4
        let mut counts = [0; 3];

        ips.iter().for_each(|ip| {
            let index = hash_to_index(*ip, 3);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_IP as f64;
        let percent_1 = counts[1] as f64 / N_IP as f64;
        let percent_2 = counts[2] as f64 / N_IP as f64;
        assert!((0.32..=0.34).contains(&percent_0));
        assert!((0.32..=0.34).contains(&percent_1));
        assert!((0.32..=0.34).contains(&percent_2));
    }

    #[test]
    fn ip_v6_test_distribution_of_hash_function_len_2() {
        const N_IP: usize = 100_000;

        // Generate 10_000 ip address and see the
        // distribution over diferent lengths
        let ips: Vec<IpAddr> = (0..N_IP)
            .map(|_| IpAddr::V6(std::net::Ipv6Addr::from_bits(rand::random::<u128>())))
            .collect();

        // Counts when length == 4
        let mut counts = [0; 2];

        ips.iter().for_each(|ip| {
            let index = hash_to_index(*ip, 2);
            counts[index] += 1;
        });

        let percent_0 = counts[0] as f64 / N_IP as f64;
        let percent_1 = counts[1] as f64 / N_IP as f64;
        assert!((0.49..=0.51).contains(&percent_0));
        assert!((0.49..=0.51).contains(&percent_1));
    }

    #[test]
    fn test_new_targets() {
        let worker_state = WorkerConfig::dummy("test", "127.0.0.1:9999", true);
        let Targets { targets } = Targets::new(&[worker_state]).unwrap();

        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_new_ip_hash() {
        let worker_state = WorkerConfig::dummy("test", "127.0.0.1:9999", true);
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
            WorkerConfig::dummy("test", "127.0.0.1:9999", true),
            WorkerConfig::dummy("test", "127.0.0.1:8888", true),
        ];
        let ip_hash = IpHash::new(&workers).unwrap();
        let client1 = ip_hash.entry("192.168.0.1".parse().unwrap()).await;
        let client2 = ip_hash.entry("192.168.0.1".parse().unwrap()).await;
        assert_eq!(client1.socket_addr(), client2.socket_addr());

        // This IP address should hash to a different index
        let client3 = ip_hash.entry("192.168.0.10".parse().unwrap()).await;
        let client4 = ip_hash.entry("192.168.0.10".parse().unwrap()).await;

        assert_eq!(client3.socket_addr(), client4.socket_addr());
        assert_eq!(client1.socket_addr(), client2.socket_addr());

        assert_ne!(client1.socket_addr(), client3.socket_addr());
    }

    #[tokio::test]
    async fn test_load_balancing_strategy_offline() {
        use crate::client::ExtractSocketAddr;

        let online = Arc::new(AtomicBool::new(false));
        let worker = WorkerConfig::dummy("test", "127.0.0.1:9999", true);

        let ip_hash = IpHash::new(&[worker]).unwrap();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            online.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let entry = ip_hash.entry("192.168.0.1".parse().unwrap()).await;

        assert_eq!(entry.socket_addr(), "127.0.0.1:9999".parse().unwrap());
    }
}
