use super::LoadBalancingStrategy;
use async_trait::async_trait;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};

use std::sync::atomic::AtomicUsize;

use tokio::sync::Mutex;

struct Targets {
    targets: Box<[SocketAddr]>,
    index: AtomicUsize,
}

impl Targets {
    fn new(targets: impl AsRef<[SocketAddr]>) -> Self {
        Targets {
            targets: targets.as_ref().into(),
            index: AtomicUsize::new(0),
        }
    }
    fn next(&self) -> SocketAddr {
        let index = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.targets[index % self.targets.len()]
    }
}

pub struct RoundRobinSimple {
    targets: Targets,
}

impl RoundRobinSimple {
    pub(crate) fn new(targets: impl AsRef<[SocketAddr]>) -> Self {
        Self {
            targets: Targets::new(targets),
        }
    }
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobinSimple {
    async fn entry(&self, _ip: IpAddr) -> SocketAddr {
        self.targets.next()
    }
}

pub struct RoundRobinIpHash {
    targets: Targets,
    table: Mutex<HashMap<IpAddr, SocketAddr>>,
}

impl RoundRobinIpHash {
    pub(crate) fn new(targets: impl AsRef<[SocketAddr]>) -> Self {
        Self {
            targets: Targets::new(targets),
            table: Mutex::new(HashMap::new()),
        }
    }
    async fn entry(&self, ip: IpAddr) -> SocketAddr {
        let mut table = self.table.lock().await;
        *table.entry(ip).or_insert_with(|| self.targets.next())
    }
}

#[async_trait]
impl LoadBalancingStrategy for RoundRobinIpHash {
    async fn entry(&self, ip: IpAddr) -> SocketAddr {
        self.entry(ip).await
    }
}
