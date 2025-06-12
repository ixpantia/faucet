mod logging;
pub use logging::{logger, LogData, LogOption};
pub mod onion;
mod router;
mod service;
use crate::{
    client::{
        load_balancing::{self, LoadBalancer, Strategy},
        worker::{WorkerConfigs, WorkerType},
        ExclusiveBody,
    },
    error::{FaucetError, FaucetResult},
    leak,
    shutdown::ShutdownSignal,
    telemetry::{TelemetryManager, TelemetrySender},
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
    pin::pin,
    sync::Arc,
};
use tokio::net::TcpListener;

pub use router::RouterConfig;

use self::{logging::LogService, service::AddStateService};

fn determine_strategy(server_type: WorkerType, strategy: Option<Strategy>) -> Strategy {
    match server_type {
        WorkerType::Plumber =>
            strategy.unwrap_or_else(|| {
                log::debug!(target: "faucet", "No load balancing strategy specified. Defaulting to round robin for plumber.");
                Strategy::RoundRobin
            }),
        WorkerType::Shiny | WorkerType::QuartoShiny => match strategy {
            None => {
                log::debug!(target: "faucet", "No load balancing strategy specified. Defaulting to IP hash for shiny.");
                Strategy::IpHash
            },
            Some(Strategy::Rps) => {
                log::debug!(target: "faucet", "RPS load balancing strategy specified for shiny, switching to IP hash.");
                Strategy::IpHash
            },
            Some(Strategy::CookieHash) => Strategy::CookieHash,
            Some(Strategy::RoundRobin) => {
                log::debug!(target: "faucet", "Round robin load balancing strategy specified for shiny, switching to IP hash.");
                Strategy::IpHash
            },
            Some(Strategy::IpHash) => Strategy::IpHash,
        },
        #[cfg(test)]
        WorkerType::Dummy => {
            log::debug!(target: "faucet", "WorkerType is Dummy, defaulting strategy to RoundRobin for tests.");
            Strategy::RoundRobin
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
    quarto: Option<OsString>,
    qmd: Option<PathBuf>,
    route: Option<String>,
    telemetry: Option<TelemetrySender>,
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
            route: None,
            quarto: None,
            qmd: None,
            telemetry: None,
        }
    }
    pub fn app_dir(mut self, app_dir: Option<impl AsRef<str>>) -> Self {
        self.app_dir = app_dir.map(|s| s.as_ref().into());
        self
    }
    pub fn strategy(mut self, strategy: Option<Strategy>) -> Self {
        log::debug!(target: "faucet", "Using load balancing strategy: {:?}", strategy);
        self.strategy = strategy;
        self
    }
    pub fn bind(mut self, bind: SocketAddr) -> Self {
        log::debug!(target: "faucet", "Will bind to: {}", bind);
        self.bind = Some(bind);
        self
    }
    pub fn extractor(mut self, extractor: load_balancing::IpExtractor) -> Self {
        log::debug!(target: "faucet", "Using IP extractor: {:?}", extractor);
        self.extractor = Some(extractor);
        self
    }
    pub fn workers(mut self, n: usize) -> Self {
        log::debug!(target: "faucet", "Will spawn {} workers", n);
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
        log::debug!(target: "faucet", "Using worker type: {:?}", server_type);
        self.server_type = Some(server_type);
        self
    }
    pub fn workdir(mut self, workdir: impl AsRef<Path>) -> Self {
        log::debug!(target: "faucet", "Using workdir: {:?}", workdir.as_ref());
        self.workdir = Some(workdir.as_ref().into());
        self
    }
    pub fn rscript(mut self, rscript: impl AsRef<OsStr>) -> Self {
        log::debug!(target: "faucet", "Using Rscript command: {:?}", rscript.as_ref());
        self.rscript = Some(rscript.as_ref().into());
        self
    }
    pub fn quarto(mut self, quarto: impl AsRef<OsStr>) -> Self {
        log::debug!(target: "faucet", "Using quarto command: {:?}", quarto.as_ref());
        self.quarto = Some(quarto.as_ref().into());
        self
    }
    pub fn qmd(mut self, qmd: Option<impl AsRef<Path>>) -> Self {
        self.qmd = qmd.map(|s| s.as_ref().into());
        self
    }
    pub fn telemetry(mut self, telemetry_manager: Option<&TelemetryManager>) -> Self {
        self.telemetry = telemetry_manager.map(|m| m.sender.clone());
        self
    }
    pub fn route(mut self, route: String) -> Self {
        self.route = Some(route);
        self
    }
    pub fn build(self) -> FaucetResult<FaucetServerConfig> {
        let server_type = self
            .server_type
            .ok_or(FaucetError::MissingArgument("server_type"))?;
        let strategy = determine_strategy(server_type, self.strategy);
        let bind = self.bind;
        let n_workers = self.n_workers.unwrap_or_else(|| {
            log::debug!(target: "faucet", "No number of workers specified. Defaulting to the number of logical cores.");
            num_cpus::get().try_into().expect("num_cpus::get() returned 0")
        });
        let workdir = self.workdir
            .map(|wd| leak!(wd, Path))
            .unwrap_or_else(|| {
                log::debug!(target: "faucet", "No workdir specified. Defaulting to the current directory.");
                Path::new(".")
            });
        let rscript = self.rscript.map(|wd| leak!(wd, OsStr)).unwrap_or_else(|| {
            log::debug!(target: "faucet", "No Rscript command specified. Defaulting to `Rscript`.");
            OsStr::new("Rscript")
        });
        let extractor = self.extractor.unwrap_or_else(|| {
            log::debug!(target: "faucet", "No IP extractor specified. Defaulting to client address.");
            load_balancing::IpExtractor::ClientAddr
        });
        let app_dir = self.app_dir.map(|app_dir| leak!(app_dir, str));
        let qmd = self.qmd.map(|qmd| leak!(qmd, Path));
        let quarto = self.quarto.map(|qmd| leak!(qmd, OsStr)).unwrap_or_else(|| {
            log::debug!(target: "faucet", "No quarto command specified. Defaulting to `quarto`.");
            OsStr::new("quarto")
        });
        let telemetry = self.telemetry;
        let route = self.route.map(|r| -> &'static _ { leak!(r) });
        Ok(FaucetServerConfig {
            strategy,
            bind,
            n_workers,
            server_type,
            workdir,
            extractor,
            rscript,
            app_dir,
            route,
            quarto,
            telemetry,
            qmd,
        })
    }
}

impl Default for FaucetServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct FaucetServerConfig {
    pub strategy: Strategy,
    pub bind: Option<SocketAddr>,
    pub n_workers: NonZeroUsize,
    pub server_type: WorkerType,
    pub workdir: &'static Path,
    pub extractor: load_balancing::IpExtractor,
    pub rscript: &'static OsStr,
    pub quarto: &'static OsStr,
    pub telemetry: Option<TelemetrySender>,
    pub app_dir: Option<&'static str>,
    pub route: Option<&'static str>,
    pub qmd: Option<&'static Path>,
}

impl FaucetServerConfig {
    pub async fn run(self, shutdown: &'static ShutdownSignal) -> FaucetResult<()> {
        let telemetry = self.telemetry.clone();
        let mut workers = WorkerConfigs::new(self.clone(), shutdown).await?;
        let load_balancer =
            LoadBalancer::new(self.strategy, self.extractor, &workers.workers).await?;
        let bind = self.bind.ok_or(FaucetError::MissingArgument("bind"))?;

        let load_balancer = load_balancer.clone();
        let service = Arc::new(
            ServiceBuilder::new(ProxyService)
                .layer(logging::LogLayer { telemetry })
                .layer(AddStateLayer::new(load_balancer))
                .build(),
        );

        // Bind to the port and listen for incoming TCP connections
        let listener = TcpListener::bind(bind).await?;
        log::info!(target: "faucet", "Listening on http://{}", bind);
        let main_loop = || async {
            loop {
                match listener.accept().await {
                    Err(e) => {
                        log::error!(target: "faucet", "Unable to accept TCP connection: {e}");
                        return;
                    }
                    Ok((tcp, client_addr)) => {
                        let tcp = TokioIo::new(tcp);
                        log::debug!(target: "faucet", "Accepted TCP connection from {}", client_addr);

                        let service = service.clone();

                        tokio::task::spawn(async move {
                            let mut conn = http1::Builder::new()
                                .half_close(true)
                                .serve_connection(
                                    tcp,
                                    service_fn(|req: Request<Incoming>| {
                                        service.call(req, Some(client_addr.ip()))
                                    }),
                                )
                                .with_upgrades();

                            let conn = pin!(&mut conn);

                            tokio::select! {
                                result = conn => {
                                    if let Err(e) = result {
                                        log::error!(target: "faucet", "Connection error: {:?}", e);
                                    }
                                }
                                _ = shutdown.wait() => ()
                            }
                        });
                    }
                };
            }
        };

        // Race the shutdown vs the main loop
        tokio::select! {
            _ = shutdown.wait() => (),
            _ = main_loop() => (),
        }

        for worker in &mut workers.workers {
            worker.wait_until_done().await;
        }

        FaucetResult::Ok(())
    }
    pub async fn extract_service(
        self,
        shutdown: &'static ShutdownSignal,
    ) -> FaucetResult<(FaucetServerService, WorkerConfigs)> {
        let telemetry = self.telemetry.clone();
        let workers = WorkerConfigs::new(self.clone(), shutdown).await?;
        let load_balancer =
            LoadBalancer::new(self.strategy, self.extractor, &workers.workers).await?;
        let service = Arc::new(
            ServiceBuilder::new(ProxyService)
                .layer(logging::LogLayer { telemetry })
                .layer(AddStateLayer::new(load_balancer))
                .build(),
        );

        Ok((FaucetServerService { inner: service }, workers))
    }
}

pub struct FaucetServerService {
    inner: Arc<AddStateService<LogService<ProxyService>>>,
}

impl Clone for FaucetServerService {
    fn clone(&self) -> Self {
        FaucetServerService {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Service<hyper::Request<Incoming>> for FaucetServerService {
    type Error = FaucetError;
    type Response = hyper::Response<ExclusiveBody>;
    async fn call(
        &self,
        req: hyper::Request<Incoming>,
        ip_addr: Option<std::net::IpAddr>,
    ) -> Result<Self::Response, Self::Error> {
        self.inner.call(req, ip_addr).await
    }
}
