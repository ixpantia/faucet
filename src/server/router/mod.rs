use std::{
    net::SocketAddr,
    pin::{self, pin, Pin},
};

use fxhash::FxHashMap;
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Request};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use super::onion::{Service, ServiceBuilder};
use crate::{
    error::FaucetResult,
    server::{logging, service::ProxyService, FaucetServerConfig},
};

#[derive(serde::Deserialize)]
struct RouterConfig {
    bind: SocketAddr,
    routes: FxHashMap<Box<str>, FaucetServerConfig>,
}

impl RouterConfig {
    pub async fn run(self) -> FaucetResult<()> {
        // Bind to the port and listen for incoming TCP connections
        let listener = TcpListener::bind(self.bind).await?;
        log::info!(target: "faucet", "Listening on http://{}", self.bind);
        loop {
            let (tcp, client_addr) = listener.accept().await?;
            log::debug!(target: "faucet", "Accepted TCP connection from {}", client_addr);
            let tcp = TokioIo::new(tcp);

            tokio::task::spawn(async move {
                let service = ServiceBuilder::new(ProxyService)
                    .layer(logging::LogLayer)
                    .build();
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
