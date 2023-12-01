pub mod round_robin;

use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;

use tokio::io;
use tokio::net::TcpStream;

#[async_trait::async_trait]
trait LoadBalancingStrategy {
    async fn entry(&self, ip: IpAddr) -> SocketAddr;
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
    pub fn new(strategy: Strategy, targets: impl AsRef<[SocketAddr]>) -> Self {
        let strategy: DynLoadBalancer = match strategy {
            Strategy::RoundRobinSimple => Arc::new(round_robin::RoundRobinSimple::new(targets)),
            Strategy::RoundRobinIpHash => Arc::new(round_robin::RoundRobinIpHash::new(targets)),
        };
        Self { strategy }
    }
    async fn entry(&self, socket: SocketAddr) -> SocketAddr {
        self.strategy.entry(socket.ip()).await
    }
    async fn connect(&self, socket: SocketAddr) -> io::Result<TcpStream> {
        TcpStream::connect(self.entry(socket).await).await
    }
    pub async fn bridge(&self, mut tcp: TcpStream, socket: SocketAddr) -> io::Result<()> {
        let mut target_tcp = self.connect(socket).await?;
        io::copy_bidirectional(&mut target_tcp, &mut tcp).await?;
        Ok(())
    }
}

impl Clone for LoadBalancer {
    fn clone(&self) -> Self {
        Self {
            strategy: Arc::clone(&self.strategy),
        }
    }
}
