use crate::error::{FaucetError, FaucetResult};
use async_trait::async_trait;
use base64::Engine;
use hyper::{
    body::Incoming,
    header::UPGRADE,
    http::{uri::PathAndQuery, HeaderValue},
    upgrade::Upgraded,
    HeaderMap, Request, Response, StatusCode, Uri,
};
use hyper_util::rt::TokioIo;
use sha1::{Digest, Sha1};
use std::net::SocketAddr;

use super::{Client, ExclusiveBody};

struct UpgradeInfo {
    headers: HeaderMap,
    uri: Uri,
}

impl UpgradeInfo {
    fn new(req: &Request<Incoming>, socket_addr: SocketAddr) -> FaucetResult<Self> {
        let headers = req.headers().clone();
        let uri = build_uri(socket_addr, req.uri().path_and_query())?;
        Ok(Self { headers, uri })
    }
}

const SEC_WEBSOCKET_APPEND: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const SEC_WEBSOCKET_KEY: &str = "Sec-WebSocket-Key";
const SEC_WEBSOCKET_ACCEPT: &str = "Sec-WebSocket-Accept";

fn calculate_sec_websocket_accept(key: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key);
    hasher.update(SEC_WEBSOCKET_APPEND);
    base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}

fn build_uri(socket_addr: SocketAddr, path: Option<&PathAndQuery>) -> FaucetResult<Uri> {
    let mut uri_builder = Uri::builder()
        .scheme("ws")
        .authority(socket_addr.to_string());
    if let Some(path) = path {
        uri_builder = uri_builder.path_and_query(path.clone());
    }
    Ok(uri_builder.build()?)
}

async fn server_upgraded_io(upgraded: Upgraded, mut upgrade_info: UpgradeInfo) -> FaucetResult<()> {
    let mut upgraded = TokioIo::new(upgraded);
    // Bridge a websocket connection to ws://localhost:3838/websocket
    // Use tokio-tungstenite to do the websocket handshake
    let mut request = Request::builder().uri(upgrade_info.uri).body(())?;
    std::mem::swap(request.headers_mut(), &mut upgrade_info.headers);
    let (mut ws_tx, _) = tokio_tungstenite::connect_async(request)
        .await
        .expect("Failed to connect");

    // Bridge the websocket stream to the upgraded connection
    tokio::io::copy_bidirectional(&mut upgraded, ws_tx.get_mut())
        .await
        .expect("Failed to copy");

    Ok(())
}

pub enum UpgradeStatus {
    Upgraded(Response<ExclusiveBody>),
    NotUpgraded(Request<Incoming>),
}

async fn upgrade_connection_from_request(
    mut req: Request<Incoming>,
    client: Client,
) -> FaucetResult<()> {
    let upgrade_info = UpgradeInfo::new(&req, client.socket_addr())?;
    let upgraded = hyper::upgrade::on(&mut req).await?;
    server_upgraded_io(upgraded, upgrade_info).await?;
    Ok(())
}

async fn init_upgrade(
    req: Request<Incoming>,
    client: Client,
) -> FaucetResult<Response<ExclusiveBody>> {
    let mut res = Response::new(ExclusiveBody::empty());
    let sec_websocket_key = req
        .headers()
        .get(SEC_WEBSOCKET_KEY)
        .cloned()
        .ok_or(FaucetError::no_sec_web_socket_key())?;
    tokio::task::spawn(async move {
        if let Err(e) = upgrade_connection_from_request(req, client).await {
            log::error!("upgrade error: {:?}", e);
        }
    });
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    res.headers_mut()
        .insert(UPGRADE, HeaderValue::from_static("websocket"));
    res.headers_mut().insert(
        hyper::header::CONNECTION,
        HeaderValue::from_static("Upgrade"),
    );
    res.headers_mut().insert(
        SEC_WEBSOCKET_ACCEPT,
        HeaderValue::from_str(&calculate_sec_websocket_accept(
            sec_websocket_key.as_bytes(),
        ))?,
    );
    Ok(res)
}

async fn attemp_upgrade(
    req: Request<hyper::body::Incoming>,
    client: Client,
) -> FaucetResult<UpgradeStatus> {
    if req.headers().contains_key(UPGRADE) {
        return Ok(UpgradeStatus::Upgraded(init_upgrade(req, client).await?));
    }
    Ok(UpgradeStatus::NotUpgraded(req))
}

#[async_trait]
pub trait WebsocketHandler {
    async fn attemp_upgrade(&self, req: Request<Incoming>) -> FaucetResult<UpgradeStatus>;
}

#[async_trait]
impl WebsocketHandler for Client {
    async fn attemp_upgrade(&self, req: Request<Incoming>) -> FaucetResult<UpgradeStatus> {
        attemp_upgrade(req, self.clone()).await
    }
}
