mod logging;
use crate::{
    client::{Client, ExclusiveBody, UpgradeStatus, WebsocketHandler},
    error::FaucetResult,
    load_balancing::LoadBalancer,
    middleware::{Layer, Service, ServiceBuilder},
};
use async_trait::async_trait;
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use std::{
    net::{IpAddr, SocketAddr},
    pin::Pin,
};
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
                    .layer(AddStateLayer {
                        socket_addr: client_addr,
                        load_balancer,
                    })
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

#[derive(Clone)]
pub(crate) struct State {
    pub remote_addr: IpAddr,
    pub client: Client,
}

#[derive(Clone)]
struct AddStateService<S> {
    inner: S,
    socket_addr: SocketAddr,
    load_balancer: LoadBalancer,
}

#[async_trait]
impl<S: Service + Send + Sync> Service for AddStateService<S> {
    async fn call(&self, mut req: Request<Incoming>) -> FaucetResult<Response<ExclusiveBody>> {
        let remote_addr = match self.load_balancer.extract_ip(&req, self.socket_addr) {
            Ok(ip) => ip,
            Err(e) => {
                log::error!(target: "faucet", "Error extracting IP, verify that proxy headers are set correctly: {}", e);
                return Err(e);
            }
        };
        let client = self.load_balancer.get_client(remote_addr).await?;
        req.extensions_mut().insert(State {
            remote_addr,
            client,
        });
        self.inner.call(req).await
    }
}

struct AddStateLayer {
    socket_addr: SocketAddr,
    load_balancer: LoadBalancer,
}

impl<S: Service + Send + Sync> Layer<S> for AddStateLayer {
    type Service = AddStateService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AddStateService {
            inner,
            socket_addr: self.socket_addr,
            load_balancer: self.load_balancer.clone(),
        }
    }
}

struct ProxyService;

#[async_trait]
impl Service for ProxyService {
    async fn call(&self, req: Request<Incoming>) -> FaucetResult<Response<ExclusiveBody>> {
        let state = req
            .extensions()
            .get::<State>()
            .expect("State not found")
            .clone();
        match state.client.attemp_upgrade(req).await? {
            UpgradeStatus::Upgraded(res) => Ok(res),
            UpgradeStatus::NotUpgraded(req) => {
                let connection = state.client.get().await?;
                connection.send_request(req).await
            }
        }
    }
}
