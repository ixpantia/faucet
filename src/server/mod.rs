mod logging;
mod onion;
mod service;
use crate::{
    client::{
        load_balancing::{self, LoadBalancer},
        worker::{WorkerType, Workers},
    },
    error::FaucetResult,
};
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request};
use hyper_util::rt::TokioIo;
use onion::{Service, ServiceBuilder};
use service::{AddStateLayer, ProxyService};
use std::{net::SocketAddr, pin::Pin};
use tokio::net::TcpListener;

pub struct FaucetServer {
    strategy: load_balancing::Strategy,
    bind: SocketAddr,
    n_workers: usize,
    server_type: Option<WorkerType>,
    workdir: Box<std::path::Path>,
    extractor: load_balancing::IpExtractor,
}

impl Default for FaucetServer {
    fn default() -> Self {
        Self::new()
    }
}

impl FaucetServer {
    pub fn new() -> Self {
        Self {
            strategy: load_balancing::Strategy::RoundRobin,
            bind: ([0, 0, 0, 0], 3000).into(),
            n_workers: 1,
            server_type: None,
            workdir: std::env::current_dir().unwrap().into(),
            extractor: load_balancing::IpExtractor::ClientAddr,
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
    pub fn extractor(mut self, extractor: load_balancing::IpExtractor) -> Self {
        self.extractor = extractor;
        self
    }
    pub fn workers(mut self, n: usize) -> Self {
        self.n_workers = n;
        self
    }
    pub fn server_type(mut self, server_type: WorkerType) -> Self {
        self.server_type = Some(server_type);
        if server_type == WorkerType::Shiny {
            log::warn!(target: "faucet", "Using server type Shiny, switching to IpHash strategy");
            self.strategy = load_balancing::Strategy::IpHash;
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
        workers.spawn(self.n_workers).await?;
        let targets = workers.get_workers_state();
        let load_balancer = LoadBalancer::new(self.strategy, self.extractor, &targets)?;

        // Bind to the port and listen for incoming TCP connections
        let listener = TcpListener::bind(self.bind).await?;
        log::info!(target: "faucet", "Listening on http://{}", self.bind);
        loop {
            let load_balancer = load_balancer.clone();

            let (tcp, client_addr) = listener.accept().await?;
            let tcp = TokioIo::new(tcp);

            tokio::task::spawn(async move {
                let service = ServiceBuilder::new(ProxyService)
                    .layer(logging::LogLayer)
                    .layer(AddStateLayer::new(client_addr, load_balancer))
                    .build();
                let mut conn = http1::Builder::new()
                    .serve_connection(tcp, service_fn(|req: Request<Incoming>| service.call(req)))
                    .with_upgrades();

                let conn = Pin::new(&mut conn);

                if let Err(e) = conn.await {
                    log::error!(target: "faucet", "Connection error: {}", e);
                }
            });
        }
    }
}
