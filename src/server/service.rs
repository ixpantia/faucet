use std::net::IpAddr;

use crate::{
    client::{load_balancing::Strategy, Client, ExclusiveBody, UpgradeStatus},
    error::FaucetError,
    server::load_balancing::LoadBalancer,
    shutdown::ShutdownSignal,
};
use hyper::{body::Incoming, header::HeaderValue};

use super::onion::{Layer, Service};

#[derive(Clone)]
pub(crate) struct State {
    pub uuid: uuid::Uuid,
    pub remote_addr: IpAddr,
    pub client: Client,
}

impl State {
    #[inline(always)]
    fn new(remote_addr: IpAddr, client: Client) -> State {
        let uuid = uuid::Uuid::now_v7();
        State {
            remote_addr,
            client,
            uuid,
        }
    }
}

#[derive(Clone)]
pub struct AddStateService<S> {
    inner: S,
    load_balancer: LoadBalancer,
}

fn uuid_to_header_value(uuid: uuid::Uuid) -> HeaderValue {
    let mut buffer = [0u8; uuid::fmt::Hyphenated::LENGTH];
    HeaderValue::from_str(uuid.hyphenated().encode_lower(&mut buffer))
        .expect("Unable to convert from uuid to header value, this is a bug")
}

fn extract_lb_uuid_from_req_cookies<B>(req: &hyper::Request<B>) -> Option<uuid::Uuid> {
    req.headers().get("Cookie").and_then(|cookie| {
        cookie.to_str().ok().and_then(|cookie_str| {
            for cookie in cookie::Cookie::split_parse(cookie_str) {
                match cookie {
                    Err(e) => {
                        log::error!(target: "faucet", "Error parsing cookie: {}", e);
                        continue;
                    }
                    Ok(cookie) => {
                        if cookie.name() == "FAUCET_LB_COOKIE" {
                            let parse_res = cookie.value().parse::<uuid::Uuid>();
                            return match parse_res {
                                Ok(uuid) => Some(uuid),
                                Err(e) => {
                                    log::error!(target: "faucet", "Error parsing UUID from cookie: {}", e);
                                    None
                                }
                            };
                        }
                    }
                }

            }
            None
        })
    })
}

fn add_lb_cookie_to_resp(resp: &mut hyper::Response<ExclusiveBody>, lb_cookie: Option<uuid::Uuid>) {
    if let Some(lb_cookie) = lb_cookie {
        resp.headers_mut().append(
            "Set-Cookie",
            HeaderValue::from_str(&format!(
                "FAUCET_LB_COOKIE={}; Path=/; HttpOnly; SameSite=Lax",
                lb_cookie
            ))
            .expect("UUID is invalid, this is a bug! Report it please!"),
        );
    }
}

// Interesting behavior:
//
// If using a cookie hash strategy and the browser starts by sending N simultaneous requests
// to the server, there will be a period on time where the server will send the
// request to random workers. It will eventually settle down to the
// Last-Used worker for the given cookie hash.
//
// Does this have any impact? I don't believe but just to take into account.
//
// Andrés

const RESEREVED_RECONNECT_PATH: &str = "__faucet__/reconnect.js";
const RECONNECT_JS: &str = include_str!("reconnect.js");

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

        // Check if the user is asking for "/__faucet__/reconnect.js"
        if req.uri().path().ends_with(RESEREVED_RECONNECT_PATH) {
            return Ok(hyper::Response::builder()
                .status(200)
                .body(ExclusiveBody::plain_text(RECONNECT_JS))
                .expect("Response should build"));
        }

        let is_cookie_hash = self.load_balancer.get_strategy() == Strategy::CookieHash;

        let lb_cookie = (is_cookie_hash)
            .then_some(extract_lb_uuid_from_req_cookies(&req).unwrap_or(uuid::Uuid::now_v7()));

        let client = self
            .load_balancer
            .get_client(remote_addr, lb_cookie)
            .await?;

        let state = State::new(remote_addr, client);

        // Add the state's UUID to the request. `X-` headers are depracted
        // https://www.rfc-editor.org/rfc/rfc6648
        req.headers_mut()
            .insert("Faucet-Request-Uuid", uuid_to_header_value(state.uuid));

        req.extensions_mut().insert(state);
        let mut resp = self.inner.call(req, Some(remote_addr)).await;

        if let Ok(resp) = &mut resp {
            if is_cookie_hash {
                add_lb_cookie_to_resp(resp, lb_cookie);
            }
        }

        resp
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

pub(crate) struct ProxyService {
    pub shutdown: &'static ShutdownSignal,
}

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
        match state.client.attempt_upgrade(req, self.shutdown).await? {
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
