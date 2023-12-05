use crate::{
    client::{ExclusiveBody, UpgradeStatus, WebsocketHandler},
    error::FaucetResult,
    load_balancing::LoadBalancer,
};
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use log::{error, info, warn};
use std::{net::SocketAddr, pin::Pin};
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
            warn!(target: "faucet", "Using server type Shiny, switching to IpHash strategy");
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
        workers.spawn(self.n_workers)?;
        let targets = workers.get_socket_addrs();
        let load_balancer = LoadBalancer::new(self.strategy, self.extractor, targets)?;

        // Bind to the port and listen for incoming TCP connections
        let listener = TcpListener::bind(self.bind).await?;
        info!(target: "faucet", "Listening on http://{}", self.bind);
        loop {
            let load_balancer = load_balancer.clone();

            let (tcp, client_addr) = listener.accept().await?;
            let tcp = TokioIo::new(tcp);

            tokio::task::spawn(async move {
                let mut conn = http1::Builder::new()
                    .serve_connection(
                        tcp,
                        service_fn(move |req: Request<Incoming>| {
                            handle_connection(req, client_addr, load_balancer.clone())
                        }),
                    )
                    .with_upgrades();

                let conn = Pin::new(&mut conn);

                if let Err(e) = conn.await {
                    error!("Connection error: {}", e);
                }
            });
        }
    }
}

// response.
async fn handle_connection(
    req: Request<Incoming>,
    client_addr: SocketAddr,
    load_balancer: LoadBalancer,
) -> FaucetResult<Response<ExclusiveBody>> {
    let client = match load_balancer.get_client(&req, client_addr).await {
        Ok(client) => client,
        Err(e) => {
            error!("Error getting client: {}", e);
            return Ok(Response::builder()
                .status(500)
                .body(ExclusiveBody::plain_text("Internal Server Error"))
                .unwrap());
        }
    };
    match client.attemp_upgrade(req).await? {
        UpgradeStatus::Upgraded(res) => Ok(res),
        UpgradeStatus::NotUpgraded(req) => {
            let connection = client.get().await?;
            connection.send_request(req).await
        }
    }
}
