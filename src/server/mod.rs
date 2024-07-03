mod logging;
mod onion;
mod service;
use crate::{
    client::{
        load_balancing::{self, LoadBalancer, Strategy},
        worker::{WorkerType, Workers},
    },
    error::{FaucetError, FaucetResult},
};
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request};
use hyper_util::rt::TokioIo;
use onion::{Service, ServiceBuilder};
use service::{AddStateLayer, ProxyService};
use std::{
    ffi::{OsStr, OsString},
    net::SocketAddr,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::net::TcpListener;

fn determine_strategy(server_type: WorkerType, strategy: Option<Strategy>) -> Strategy {
    match server_type {
        WorkerType::Plumber =>
            strategy.unwrap_or_else(|| {
                log::info!(target: "faucet", "No load balancing strategy specified. Defaulting to round robin for plumber.");
                Strategy::RoundRobin
            }),
        WorkerType::Shiny => match strategy {
            None => {
                log::info!(target: "faucet", "No load balancing strategy specified. Defaulting to IP hash for shiny.");
                Strategy::IpHash
            },
            Some(Strategy::RoundRobin) => {
                log::info!(target: "faucet", "Round robin load balancing strategy specified for shiny, switching to IP hash.");
                Strategy::IpHash
            },
            Some(Strategy::IpHash) => Strategy::IpHash,
        }
    }
}

pub struct FaucetServerBuilder {
    strategy: Option<Strategy>,
    bind: Option<SocketAddr>,
    n_workers: Option<NonZeroUsize>,
    server_type: Option<WorkerType>,
    workdir: Option<PathBuf>,
    extractor: Option<load_balancing::IpExtractor>,
    rscript: Option<OsString>,
    app_dir: Option<String>,
}

impl FaucetServerBuilder {
    pub fn new() -> Self {
        FaucetServerBuilder {
            strategy: None,
            bind: None,
            n_workers: None,
            server_type: None,
            workdir: None,
            extractor: None,
            rscript: None,
            app_dir: None,
        }
    }
    pub fn app_dir(mut self, app_dir: Option<impl AsRef<str>>) -> Self {
        self.app_dir = app_dir.map(|s| s.as_ref().into());
        self
    }
    pub fn strategy(mut self, strategy: Strategy) -> Self {
        log::info!(target: "faucet", "Using load balancing strategy: {:?}", strategy);
        self.strategy = Some(strategy);
        self
    }
    pub fn bind(mut self, bind: SocketAddr) -> Self {
        log::info!(target: "faucet", "Will bind to: {}", bind);
        self.bind = Some(bind);
        self
    }
    pub fn extractor(mut self, extractor: load_balancing::IpExtractor) -> Self {
        log::info!(target: "faucet", "Using IP extractor: {:?}", extractor);
        self.extractor = Some(extractor);
        self
    }
    pub fn workers(mut self, n: usize) -> Self {
        log::info!(target: "faucet", "Will spawn {} workers", n);
        self.n_workers = match n.try_into() {
            Ok(n) => Some(n),
            Err(_) => {
                log::error!(target: "faucet", "Number of workers must be greater than 0");
                std::process::exit(1);
            }
        };
        self
    }
    pub fn server_type(mut self, server_type: WorkerType) -> Self {
        log::info!(target: "faucet", "Using worker type: {:?}", server_type);
        self.server_type = Some(server_type);
        self
    }
    pub fn workdir(mut self, workdir: impl AsRef<Path>) -> Self {
        log::info!(target: "faucet", "Using workdir: {:?}", workdir.as_ref());
        self.workdir = Some(workdir.as_ref().into());
        self
    }
    pub fn rscript(mut self, rscript: impl AsRef<OsStr>) -> Self {
        log::info!(target: "faucet", "Using Rscript command: {:?}", rscript.as_ref());
        self.rscript = Some(rscript.as_ref().into());
        self
    }
    pub fn build(self) -> FaucetResult<FaucetServerConfig> {
        let server_type = self
            .server_type
            .ok_or(FaucetError::MissingArgument("server_type"))?;
        let strategy = determine_strategy(server_type, self.strategy);
        let bind = self.bind.ok_or(FaucetError::MissingArgument("bind"))?;
        let n_workers = self.n_workers.unwrap_or_else(|| {
            log::info!(target: "faucet", "No number of workers specified. Defaulting to the number of logical cores.");
            num_cpus::get().try_into().expect("num_cpus::get() returned 0")
        });
        let workdir = self.workdir
            .map(|wd| Box::leak(wd.into_boxed_path()) as &'static Path)
            .unwrap_or_else(|| {
                log::info!(target: "faucet", "No workdir specified. Defaulting to the current directory.");
                Path::new(".")
            });
        let rscript = self.rscript
            .map(|wd| Box::leak(wd.into_boxed_os_str()) as &'static OsStr)
            .unwrap_or_else(|| {
                log::info!(target: "faucet", "No Rscript command specified. Defaulting to `Rscript`.");
                OsStr::new("Rscript")
            });
        let extractor = self.extractor.unwrap_or_else(|| {
            log::info!(target: "faucet", "No IP extractor specified. Defaulting to client address.");
            load_balancing::IpExtractor::ClientAddr
        });
        let app_dir = self.app_dir.map(|app_dir| app_dir.leak() as &'static str);
        Ok(FaucetServerConfig {
            strategy,
            bind,
            n_workers,
            server_type,
            workdir,
            extractor,
            rscript,
            app_dir,
        })
    }
}

impl Default for FaucetServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
pub struct FaucetServerConfig {
    pub strategy: Strategy,
    pub bind: SocketAddr,
    pub n_workers: NonZeroUsize,
    pub server_type: WorkerType,
    pub workdir: &'static Path,
    pub extractor: load_balancing::IpExtractor,
    pub rscript: &'static OsStr,
    pub app_dir: Option<&'static str>,
}

impl FaucetServerConfig {
    pub async fn run(self) -> FaucetResult<()> {
        let workers = Workers::new(self).await?;
        let targets = workers.get_workers_config();
        let load_balancer = LoadBalancer::new(self.strategy, self.extractor, &targets)?;

        // Bind to the port and listen for incoming TCP connections
        let listener = TcpListener::bind(self.bind).await?;
        log::info!(target: "faucet", "Listening on http://{}", self.bind);
        loop {
            let load_balancer = load_balancer.clone();

            let (tcp, client_addr) = listener.accept().await?;
            log::debug!(target: "faucet", "Accepted TCP connection from {}", client_addr);
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
