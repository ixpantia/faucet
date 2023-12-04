pub mod round_robin;

use crate::client::Client;
use crate::error::FaucetResult;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;

#[async_trait::async_trait]
trait LoadBalancingStrategy {
    async fn entry(&self, ip: IpAddr) -> Client;
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Strategy {
    RoundRobinSimple,
    RoundRobinIpHash,
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Strategy::RoundRobinSimple => write!(f, "round_robin"),
            Strategy::RoundRobinIpHash => write!(f, "round_robin_ip_hash"),
        }
    }
}

impl FromStr for Strategy {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "round_robin" => Ok(Self::RoundRobinSimple),
            "round_robin_ip_hash" => Ok(Self::RoundRobinIpHash),
            _ => Err("invalid strategy"),
        }
    }
}

type DynLoadBalancer = Arc<dyn LoadBalancingStrategy + Send + Sync>;

pub struct LoadBalancer {
    strategy: DynLoadBalancer,
}

impl LoadBalancer {
    pub fn new(strategy: Strategy, targets: impl AsRef<[SocketAddr]>) -> FaucetResult<Self> {
        let strategy: DynLoadBalancer = match strategy {
            Strategy::RoundRobinSimple => Arc::new(round_robin::RoundRobinSimple::new(targets)?),
            Strategy::RoundRobinIpHash => Arc::new(round_robin::RoundRobinIpHash::new(targets)?),
        };
        Ok(Self { strategy })
    }
    pub async fn get_client(&self, socket: IpAddr) -> Client {
        self.strategy.entry(socket).await
    }
}

impl Clone for LoadBalancer {
    fn clone(&self) -> Self {
        Self {
            strategy: Arc::clone(&self.strategy),
        }
    }
}
