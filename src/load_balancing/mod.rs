mod ip_extractor;
pub mod ip_hash;
pub mod round_robin;

pub use ip_extractor::IpExtractor;

use hyper::body::Incoming;
use hyper::Request;

use crate::client::Client;
use crate::error::FaucetResult;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;

use self::ip_hash::IpHash;
use self::round_robin::RoundRobin;

#[async_trait::async_trait]
trait LoadBalancingStrategy {
    async fn entry(&self, ip: IpAddr) -> Client;
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Strategy {
    RoundRobin,
    IpHash,
}

impl FromStr for Strategy {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "round_robin" => Ok(Self::RoundRobin),
            "ip_hash" => Ok(Self::IpHash),
            _ => Err("invalid strategy"),
        }
    }
}

type DynLoadBalancer = Arc<dyn LoadBalancingStrategy + Send + Sync>;

pub struct LoadBalancer {
    strategy: DynLoadBalancer,
    extractor: IpExtractor,
}

impl LoadBalancer {
    pub fn new(
        strategy: Strategy,
        extractor: IpExtractor,
        targets: impl AsRef<[SocketAddr]>,
    ) -> FaucetResult<Self> {
        let strategy: DynLoadBalancer = match strategy {
            Strategy::RoundRobin => Arc::new(RoundRobin::new(targets)?),
            Strategy::IpHash => Arc::new(IpHash::new(targets)?),
        };
        Ok(Self {
            strategy,
            extractor,
        })
    }
    pub async fn get_client(
        &self,
        reqest: &Request<Incoming>,
        socket: SocketAddr,
    ) -> FaucetResult<Client> {
        let ip = self.extractor.extract(reqest, socket)?;
        Ok(self.strategy.entry(ip).await)
    }
}

impl Clone for LoadBalancer {
    fn clone(&self) -> Self {
        Self {
            strategy: Arc::clone(&self.strategy),
            extractor: self.extractor,
        }
    }
}
