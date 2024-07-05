use std::{
    collections::HashSet, ffi::OsStr, net::SocketAddr, num::NonZeroUsize, path::PathBuf, pin::pin,
};

use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use super::{onion::Service, FaucetServerBuilder, FaucetServerService};
use crate::{
    client::{
        load_balancing::{IpExtractor, Strategy},
        worker::WorkerType,
        ExclusiveBody,
    },
    error::{FaucetError, FaucetResult},
};

fn default_workdir() -> PathBuf {
    PathBuf::from(".")
}

#[derive(serde::Deserialize)]
struct ReducedServerConfig {
    pub strategy: Option<Strategy>,
    #[serde(default = "default_workdir")]
    pub workdir: PathBuf,
    pub app_dir: Option<String>,
    pub workers: NonZeroUsize,
    pub server_type: WorkerType,
    pub qmd: Option<PathBuf>,
}

#[derive(serde::Deserialize)]
struct RouteConfig {
    route: String,
    #[serde(flatten)]
    config: ReducedServerConfig,
}

#[derive(serde::Deserialize)]
pub struct RouterConfig {
    route: Vec<RouteConfig>,
}

#[derive(Copy, Clone)]
struct RouterService {
    routes: &'static [&'static str],
    clients: &'static [FaucetServerService],
}

fn strip_prefix(uri: &Uri, prefix: &str) -> Option<Uri> {
    let path_and_query = uri.path_and_query()?;

    // Try to strip the prefix. It is fails we short-circuit.
    let after_prefix = path_and_query.path().strip_prefix(prefix)?;

    let new_path_and_query = match (after_prefix.starts_with('/'), path_and_query.query()) {
        (true, None) => after_prefix.parse().unwrap(),
        (true, Some(query)) => format!("{after_prefix}?{query}").parse().unwrap(),
        (false, None) => format!("/{after_prefix}").parse().unwrap(),
        (false, Some(query)) => format!("/{after_prefix}?{query}").parse().unwrap(),
    };

    let mut parts = uri.clone().into_parts();
    parts.path_and_query = Some(new_path_and_query);

    Some(Uri::from_parts(parts).unwrap())
}

impl Service<hyper::Request<Incoming>> for RouterService {
    type Error = FaucetError;
    type Response = hyper::Response<ExclusiveBody>;
    async fn call(
        &self,
        mut req: hyper::Request<Incoming>,
        ip_addr: Option<std::net::IpAddr>,
    ) -> Result<Self::Response, Self::Error> {
        let mut client = None;
        for i in 0..self.routes.len() {
            let route = self.routes[i];
            if let Some(new_uri) = strip_prefix(req.uri(), route) {
                client = Some(&self.clients[i]);
                *req.uri_mut() = new_uri;
                break;
            }
        }
        match client {
            None => Ok(hyper::Response::builder()
                .status(404)
                .body(ExclusiveBody::plain_text("404 not found"))
                .expect("Response should build")),
            Some(client) => client.call(req, ip_addr).await,
        }
    }
}

impl RouterConfig {
    async fn into_service(
        self,
        rscript: impl AsRef<OsStr>,
        quarto: impl AsRef<OsStr>,
        ip_from: IpExtractor,
    ) -> FaucetResult<RouterService> {
        let mut routes = Vec::with_capacity(self.route.len());
        let mut clients = Vec::with_capacity(self.route.len());
        let mut routes_set = HashSet::with_capacity(self.route.len());
        for route_conf in self.route.into_iter() {
            let route: &'static str = route_conf.route.leak();
            if !routes_set.insert(route) {
                return Err(FaucetError::DuplicateRoute(route));
            }
            routes.push(route);
            let client = FaucetServerBuilder::new()
                .workdir(route_conf.config.workdir)
                .server_type(route_conf.config.server_type)
                .strategy(route_conf.config.strategy)
                .rscript(&rscript)
                .quarto(&quarto)
                .qmd(route_conf.config.qmd)
                .workers(route_conf.config.workers.get())
                .extractor(ip_from)
                .app_dir(route_conf.config.app_dir)
                .build()?
                .extract_service(&format!("[{route}]::"))
                .await?;
            clients.push(client);
        }
        let routes = routes.leak();
        let clients = clients.leak();
        let service = RouterService { clients, routes };
        Ok(service)
    }
}

impl RouterConfig {
    pub async fn run(
        self,
        rscript: impl AsRef<OsStr>,
        quarto: impl AsRef<OsStr>,
        ip_from: IpExtractor,
        addr: SocketAddr,
    ) -> FaucetResult<()> {
        let service = self.into_service(rscript, quarto, ip_from).await?;
        // Bind to the port and listen for incoming TCP connections
        let listener = TcpListener::bind(addr).await?;
        log::info!(target: "faucet", "Listening on http://{}", addr);
        loop {
            let (tcp, client_addr) = listener.accept().await?;
            let tcp = TokioIo::new(tcp);
            log::debug!(target: "faucet", "Accepted TCP connection from {}", client_addr);

            tokio::task::spawn(async move {
                let mut conn = http1::Builder::new()
                    .serve_connection(
                        tcp,
                        service_fn(|req: Request<Incoming>| {
                            service.call(req, Some(client_addr.ip()))
                        }),
                    )
                    .with_upgrades();

                let conn = pin!(&mut conn);

                if let Err(e) = conn.await {
                    log::error!(target: "faucet", "Connection error: {}", e);
                }
            });
        }
    }
}
