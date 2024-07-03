use std::{
    collections::HashSet, ffi::OsStr, io::Write, net::SocketAddr, num::NonZeroUsize, path::PathBuf,
    pin::pin,
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
    pub app_dir: String,
    pub workers: NonZeroUsize,
    pub server_type: WorkerType,
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

impl Service<hyper::Request<Incoming>> for RouterService {
    type Error = FaucetError;
    type Response = hyper::Response<ExclusiveBody>;
    async fn call(
        &self,
        mut req: hyper::Request<Incoming>,
        ip_addr: Option<std::net::IpAddr>,
    ) -> Result<Self::Response, Self::Error> {
        println!("{:?}", req.uri());
        let mut client = None;
        for i in 0..self.routes.len() {
            let route = self.routes[i];
            if req.uri().path().starts_with(route) {
                client = Some(&self.clients[i]);
                let mut new_uri = Uri::builder();
                let mut new_path_and_query = Vec::<u8>::new();
                let path = req
                    .uri()
                    .path()
                    .strip_prefix(route)
                    .expect("You may strip route from prefix");

                let _ = write!(&mut new_path_and_query, "/{path}");

                if let Some(query) = req.uri().query() {
                    let _ = write!(&mut new_path_and_query, "{query}");
                }

                new_uri = new_uri.path_and_query(new_path_and_query);

                *req.uri_mut() = new_uri.build().expect("Should work");

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
                .workers(route_conf.config.workers.get())
                .extractor(ip_from)
                .app_dir(Some(route_conf.config.app_dir))
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
        ip_from: IpExtractor,
        addr: SocketAddr,
    ) -> FaucetResult<()> {
        let service = self.into_service(rscript, ip_from).await?;
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
