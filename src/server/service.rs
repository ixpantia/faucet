use std::net::{IpAddr, SocketAddr};

use crate::{
    client::{Client, ExclusiveBody, UpgradeStatus},
    error::FaucetError,
    server::load_balancing::LoadBalancer,
};
use hyper::body::Incoming;

use super::onion::{Layer, Service};

#[derive(Clone)]
pub(crate) struct State {
    pub remote_addr: IpAddr,
    pub client: Client,
}

#[derive(Clone)]
pub struct AddStateService<S> {
    inner: S,
    load_balancer: LoadBalancer,
}

impl<S, ReqBody> Service<hyper::Request<ReqBody>> for AddStateService<S>
where
    ReqBody: hyper::body::Body + Send + Sync + 'static,
    S: Service<
            hyper::Request<ReqBody>,
            Response = hyper::Response<ExclusiveBody>,
            Error = FaucetError,
        > + Send
        + Sync,
{
    type Error = FaucetError;
    type Response = hyper::Response<ExclusiveBody>;

    async fn call(
        &self,
        mut req: hyper::Request<ReqBody>,
        socket_addr: Option<IpAddr>,
    ) -> Result<Self::Response, Self::Error> {
        let remote_addr = match self.load_balancer.extract_ip(&req, socket_addr) {
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
        self.inner.call(req, Some(remote_addr)).await
    }
}

pub struct AddStateLayer {
    load_balancer: LoadBalancer,
}

impl AddStateLayer {
    #[inline]
    pub fn new(load_balancer: LoadBalancer) -> Self {
        Self { load_balancer }
    }
}

impl<S> Layer<S> for AddStateLayer {
    type Service = AddStateService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AddStateService {
            inner,
            load_balancer: self.load_balancer.clone(),
        }
    }
}

pub(crate) struct ProxyService;

impl Service<hyper::Request<Incoming>> for ProxyService {
    type Error = FaucetError;
    type Response = hyper::Response<ExclusiveBody>;

    async fn call(
        &self,
        req: hyper::Request<Incoming>,
        _: Option<IpAddr>,
    ) -> Result<Self::Response, Self::Error> {
        let state = req
            .extensions()
            .get::<State>()
            .expect("State not found")
            .clone();
        match state.client.attempt_upgrade(req).await? {
            UpgradeStatus::Upgraded(res) => {
                log::debug!(
                    target: "faucet",
                    "Initializing WebSocket bridge from {} to {}",
                    state.remote_addr,
                    state.client.config.target
                );
                Ok(res)
            }
            UpgradeStatus::NotUpgraded(req) => {
                let connection = state.client.get().await?;
                connection.send_request(req).await
            }
        }
    }
}
