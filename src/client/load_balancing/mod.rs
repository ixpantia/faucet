pub mod cookie_hash;
mod ip_extractor;
pub mod ip_hash;
pub mod round_robin;
pub mod round_robin_rps;

use super::worker::WorkerConfig;
use crate::client::Client;
use crate::error::FaucetResult;
use crate::leak;
use cookie_hash::CookieHash;
use hyper::Request;
pub use ip_extractor::IpExtractor;
use std::net::IpAddr;
use std::str::FromStr;
use uuid::Uuid;

use self::ip_hash::IpHash;
use self::round_robin::RoundRobin;

trait LoadBalancingStrategy {
    type Input;
    async fn entry(&self, ip: Self::Input) -> Client;
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, Eq, PartialEq, serde::Deserialize)]
#[serde(rename = "snake_case")]
pub enum Strategy {
    #[serde(alias = "round_robin", alias = "RoundRobin", alias = "round-robin")]
    RoundRobin,
    #[serde(alias = "ip_hash", alias = "IpHash", alias = "ip-hash")]
    IpHash,
    #[serde(alias = "cookie_hash", alias = "CookieHash", alias = "cookie-hash")]
    CookieHash,
    #[serde(alias = "rps", alias = "Rps", alias = "rps")]
    Rps,
}

impl FromStr for Strategy {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "round_robin" => Ok(Self::RoundRobin),
            "ip_hash" => Ok(Self::IpHash),
            "cookie_hash" => Ok(Self::CookieHash),
            "rps" => Ok(Self::Rps),
            _ => Err("invalid strategy"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LBIdent {
    Ip(IpAddr),
    Uuid(Uuid),
}

#[derive(Copy, Clone)]
enum DynLoadBalancer {
    IpHash(&'static ip_hash::IpHash),
    RoundRobin(&'static round_robin::RoundRobin),
    CookieHash(&'static cookie_hash::CookieHash),
    Rps(&'static round_robin_rps::RoundRobinRps),
}

impl LoadBalancingStrategy for DynLoadBalancer {
    type Input = LBIdent;
    async fn entry(&self, ip: LBIdent) -> Client {
        match ip {
            LBIdent::Ip(ip) => match self {
                DynLoadBalancer::RoundRobin(rr) => rr.entry(ip).await,
                DynLoadBalancer::IpHash(ih) => ih.entry(ip).await,
                DynLoadBalancer::Rps(rr) => rr.entry(ip).await,
                _ => unreachable!(
                    "This should never happen, ip should never be passed to cookie hash"
                ),
            },
            LBIdent::Uuid(uuid) => match self {
                DynLoadBalancer::CookieHash(ch) => ch.entry(uuid).await,
                _ => unreachable!(
                    "This should never happen, uuid should never be passed to round robin or ip hash"
                ),
            },
        }
    }
}

pub(crate) struct LoadBalancer {
    strategy: DynLoadBalancer,
    extractor: IpExtractor,
}

impl LoadBalancer {
    pub async fn new(
        strategy: Strategy,
        extractor: IpExtractor,
        workers: &[&'static WorkerConfig],
    ) -> FaucetResult<Self> {
        let strategy: DynLoadBalancer = match strategy {
            Strategy::RoundRobin => {
                DynLoadBalancer::RoundRobin(leak!(RoundRobin::new(workers).await))
            }
            Strategy::IpHash => DynLoadBalancer::IpHash(leak!(IpHash::new(workers).await)),
            Strategy::CookieHash => {
                DynLoadBalancer::CookieHash(leak!(CookieHash::new(workers).await))
            }
            Strategy::Rps => {
                DynLoadBalancer::Rps(leak!(round_robin_rps::RoundRobinRps::new(workers).await))
            }
        };
        Ok(Self {
            strategy,
            extractor,
        })
    }
    pub fn get_strategy(&self) -> Strategy {
        match self.strategy {
            DynLoadBalancer::RoundRobin(_) => Strategy::RoundRobin,
            DynLoadBalancer::IpHash(_) => Strategy::IpHash,
            DynLoadBalancer::CookieHash(_) => Strategy::CookieHash,
            DynLoadBalancer::Rps(_) => Strategy::Rps,
        }
    }
    async fn get_client_ip(&self, ip: IpAddr) -> FaucetResult<Client> {
        Ok(self.strategy.entry(LBIdent::Ip(ip)).await)
    }
    async fn get_client_uuid(&self, uuid: Uuid) -> FaucetResult<Client> {
        Ok(self.strategy.entry(LBIdent::Uuid(uuid)).await)
    }
    pub async fn get_client(&self, ip: IpAddr, uuid: Option<Uuid>) -> FaucetResult<Client> {
        if let Some(uuid) = uuid {
            self.get_client_uuid(uuid).await
        } else {
            self.get_client_ip(ip).await
        }
    }
    pub fn extract_ip<B>(
        &self,
        request: &Request<B>,
        socket: Option<IpAddr>,
    ) -> FaucetResult<IpAddr> {
        self.extractor.extract(request, socket)
    }
}

impl Clone for LoadBalancer {
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy,
            extractor: self.extractor,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_strategy_from_str() {
        assert_eq!(
            Strategy::from_str("round_robin").unwrap(),
            Strategy::RoundRobin
        );
        assert_eq!(Strategy::from_str("ip_hash").unwrap(), Strategy::IpHash);
        assert!(Strategy::from_str("invalid").is_err());
    }

    #[test]
    fn test_load_balancer_new_round_robin() {
        let configs = Vec::new();
        let _ = LoadBalancer::new(Strategy::RoundRobin, IpExtractor::XForwardedFor, &configs)
            .expect("failed to create load balancer");
    }

    #[test]
    fn test_load_balancer_new_ip_hash() {
        let configs = Vec::new();
        let _ = LoadBalancer::new(Strategy::IpHash, IpExtractor::XForwardedFor, &configs)
            .expect("failed to create load balancer");
    }

    #[test]
    fn test_load_balancer_extract_ip() {
        let configs = Vec::new();
        let load_balancer =
            LoadBalancer::new(Strategy::RoundRobin, IpExtractor::XForwardedFor, &configs)
                .expect("failed to create load balancer");
        let request = Request::builder()
            .header("x-forwarded-for", "192.168.0.1")
            .body(())
            .unwrap();
        let ip = load_balancer
            .extract_ip(&request, Some("127.0.0.1".parse().unwrap()))
            .expect("failed to extract ip");

        assert_eq!(ip, "192.168.0.1".parse::<IpAddr>().unwrap());
    }

    #[tokio::test]
    async fn test_load_balancer_get_client() {
        use crate::client::ExtractSocketAddr;
        let configs = [
            WorkerConfig::dummy("test", "127.0.0.1:9999", true),
            WorkerConfig::dummy("test", "127.0.0.1:9998", true),
        ];
        let load_balancer =
            LoadBalancer::new(Strategy::RoundRobin, IpExtractor::XForwardedFor, &configs)
                .expect("failed to create load balancer");
        let ip = "192.168.0.1".parse().unwrap();
        let client = load_balancer
            .get_client_ip(ip)
            .await
            .expect("failed to get client");
        assert_eq!(client.socket_addr(), "127.0.0.1:9999".parse().unwrap());

        let client = load_balancer
            .get_client_ip(ip)
            .await
            .expect("failed to get client");

        assert_eq!(client.socket_addr(), "127.0.0.1:9998".parse().unwrap());
    }

    #[test]
    fn test_clone_load_balancer() {
        let configs = Vec::new();
        let load_balancer =
            LoadBalancer::new(Strategy::RoundRobin, IpExtractor::XForwardedFor, &configs)
                .expect("failed to create load balancer");
        let _ = load_balancer.clone();
    }
}
