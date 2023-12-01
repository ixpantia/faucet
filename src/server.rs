use crate::error::FaucetResult;
use log::{info, warn};
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::{
    load_balancing,
    worker::{WorkerType, Workers},
};

pub struct FaucetServer {
    strategy: load_balancing::Strategy,
    bind: SocketAddr,
    n_workers: usize,
    server_type: Option<WorkerType>,
    workdir: Box<std::path::Path>,
}

impl Default for FaucetServer {
    fn default() -> Self {
        Self::new()
    }
}

impl FaucetServer {
    pub fn new() -> Self {
        Self {
            strategy: load_balancing::Strategy::RoundRobinIpHash,
            bind: ([0, 0, 0, 0], 3000).into(),
            n_workers: 1,
            server_type: None,
            workdir: std::env::current_dir().unwrap().into(),
        }
    }
    pub fn strategy(mut self, strategy: load_balancing::Strategy) -> Self {
        self.strategy = strategy;
        self
    }
    pub fn bind(mut self, bind: SocketAddr) -> Self {
        self.bind = bind;
        self
    }
    pub fn workers(mut self, n: usize) -> Self {
        self.n_workers = n;
        self
    }
    pub fn server_type(mut self, server_type: WorkerType) -> Self {
        self.server_type = Some(server_type);
        if server_type == WorkerType::Shiny {
            warn!("Using server type Shiny, switching to RoundRobinIpHash strategy");
            self.strategy = load_balancing::Strategy::RoundRobinIpHash;
        }
        self
    }
    pub fn workdir(mut self, workdir: impl AsRef<std::path::Path>) -> Self {
        self.workdir = workdir.as_ref().into();
        self
    }
    pub async fn run(self) -> FaucetResult<()> {
        let mut workers = Workers::new(
            self.server_type.unwrap_or(WorkerType::Plumber),
            self.workdir,
        );
        workers.spawn(self.n_workers)?;
        let targets = workers.get_socket_addrs();
        let load_balancer = load_balancing::LoadBalancer::new(self.strategy, targets);

        // Bind to the port and listen for incoming TCP connections
        let listener = TcpListener::bind(self.bind).await?;
        info!("Listening on http://{}", self.bind);
        loop {
            let load_balancer = load_balancer.clone();
            // When an incoming TCP connection is received grab a TCP stream for
            // client<->server communication.
            //
            // Note, this is a .await point, this loop will loop forever but is not a busy loop. The
            // .await point allows the Tokio runtime to pull the task off of the thread until the task
            // has work to do. In this case, a connection arrives on the port we are listening on and
            // the task is woken up, at which point the task is then put back on a thread, and is
            // driven forward by the runtime, eventually yielding a TCP stream.
            let (tcp, x) = listener.accept().await?;
            // Use an adapter to access something implementing `tokio::io` traits as if they implement
            // `hyper::rt` IO traits.

            // Spin up a new task in Tokio so we can continue to listen for new TCP connection on the
            // current task without waiting for the processing of the HTTP1 connection we just received
            // to finish
            tokio::task::spawn(async move {
                if let Err(e) = load_balancer.bridge(tcp, x).await {
                    log::warn!("Dropping connection due to error: {}", e);
                };
            });
        }
    }
}
