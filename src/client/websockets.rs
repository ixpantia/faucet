use super::{pool::ExtractSocketAddr, Client, ExclusiveBody};
use crate::{
    error::{FaucetError, FaucetResult},
    global_conn::{add_connection, remove_connection},
};
use base64::Engine;
use futures_util::StreamExt;
use hyper::{
    header::UPGRADE,
    http::{uri::PathAndQuery, HeaderValue},
    upgrade::Upgraded,
    HeaderMap, Request, Response, StatusCode, Uri,
};
use hyper_util::rt::TokioIo;
use sha1::{Digest, Sha1};
use std::net::SocketAddr;

struct UpgradeInfo {
    headers: HeaderMap,
    uri: Uri,
}

impl UpgradeInfo {
    fn new<ReqBody>(req: &Request<ReqBody>, socket_addr: SocketAddr) -> FaucetResult<Self> {
        let headers = req.headers().clone();
        let uri = build_uri(socket_addr, req.uri().path_and_query())?;
        Ok(Self { headers, uri })
    }
}

const SEC_WEBSOCKET_APPEND: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const SEC_WEBSOCKET_KEY: &str = "Sec-WebSocket-Key";
const SEC_WEBSOCKET_ACCEPT: &str = "Sec-WebSocket-Accept";

fn calculate_sec_websocket_accept<'buffer>(key: &[u8], buffer: &'buffer mut [u8]) -> &'buffer [u8] {
    let mut hasher = Sha1::new();
    hasher.update(key);
    hasher.update(SEC_WEBSOCKET_APPEND);
    let len = base64::engine::general_purpose::STANDARD
        .encode_slice(hasher.finalize(), buffer)
        .expect("Should always write the internal buffer");
    &buffer[..len]
}

fn build_uri(socket_addr: SocketAddr, path: Option<&PathAndQuery>) -> FaucetResult<Uri> {
    let mut uri_builder = Uri::builder()
        .scheme("ws")
        .authority(socket_addr.to_string());
    match path {
        Some(path) => uri_builder = uri_builder.path_and_query(path.clone()),
        None => uri_builder = uri_builder.path_and_query("/"),
    }
    Ok(uri_builder.build()?)
}

async fn server_upgraded_io(upgraded: Upgraded, upgrade_info: UpgradeInfo) -> FaucetResult<()> {
    let upgraded = TokioIo::new(upgraded);
    // Bridge a websocket connection to ws://localhost:3838/websocket
    // Use tokio-tungstenite to do the websocket handshake
    let mut request = Request::builder().uri(upgrade_info.uri).body(())?;
    *request.headers_mut() = upgrade_info.headers;
    let (shiny_ws, _) = tokio_tungstenite::connect_async(request).await?;

    // Bridge the websocket stream to the upgraded connection
    // tokio::io::copy_bidirectional(&mut upgraded, ws_tx.get_mut()).await?;

    // Instead of using copy_bidirectional, we can manually intercept the
    // messages and forward them.
    // We want this to add reconnection logic later.
    let (shiny_tx, shiny_rx) = shiny_ws.split();

    let upgraded_ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        upgraded,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    )
    .await;

    let (upgraded_tx, upgraded_rx) = upgraded_ws.split();

    let client_to_shiny = upgraded_rx.forward(shiny_tx);
    let shiny_to_client = shiny_rx.forward(upgraded_tx);

    tokio::select! {
        _ = client_to_shiny => (),
        _ = shiny_to_client => (),
    };

    Ok(())
}

pub enum UpgradeStatus<ReqBody> {
    Upgraded(Response<ExclusiveBody>),
    NotUpgraded(Request<ReqBody>),
}

async fn upgrade_connection_from_request<ReqBody>(
    mut req: Request<ReqBody>,
    client: impl ExtractSocketAddr,
) -> FaucetResult<()> {
    let upgrade_info = UpgradeInfo::new(&req, client.socket_addr())?;
    let upgraded = hyper::upgrade::on(&mut req).await?;
    server_upgraded_io(upgraded, upgrade_info).await?;
    Ok(())
}

async fn init_upgrade<ReqBody: Send + Sync + 'static>(
    req: Request<ReqBody>,
    client: impl ExtractSocketAddr + Send + Sync + 'static,
) -> FaucetResult<Response<ExclusiveBody>> {
    let mut res = Response::new(ExclusiveBody::empty());
    let sec_websocket_key = req
        .headers()
        .get(SEC_WEBSOCKET_KEY)
        .cloned()
        .ok_or(FaucetError::no_sec_web_socket_key())?;
    tokio::task::spawn(async move {
        add_connection();
        if let Err(e) = upgrade_connection_from_request(req, client).await {
            log::error!("upgrade error: {:?}", e);
        }
        remove_connection();
    });
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    res.headers_mut()
        .insert(UPGRADE, HeaderValue::from_static("websocket"));
    res.headers_mut().insert(
        hyper::header::CONNECTION,
        HeaderValue::from_static("Upgrade"),
    );
    let mut buffer = [0u8; 32];
    res.headers_mut().insert(
        SEC_WEBSOCKET_ACCEPT,
        HeaderValue::from_bytes(calculate_sec_websocket_accept(
            sec_websocket_key.as_bytes(),
            &mut buffer,
        ))?,
    );
    Ok(res)
}

#[inline(always)]
async fn attempt_upgrade<ReqBody: Send + Sync + 'static>(
    req: Request<ReqBody>,
    client: impl ExtractSocketAddr + Send + Sync + 'static,
) -> FaucetResult<UpgradeStatus<ReqBody>> {
    if req.headers().contains_key(UPGRADE) {
        return Ok(UpgradeStatus::Upgraded(init_upgrade(req, client).await?));
    }
    Ok(UpgradeStatus::NotUpgraded(req))
}

impl Client {
    pub async fn attempt_upgrade<ReqBody>(
        &self,
        req: Request<ReqBody>,
    ) -> FaucetResult<UpgradeStatus<ReqBody>>
    where
        ReqBody: Send + Sync + 'static,
    {
        attempt_upgrade(req, self.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::networking::get_available_socket;

    use super::*;

    #[test]
    fn test_calculate_sec_websocket_accept() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let mut buffer = [0u8; 32];
        let accept = calculate_sec_websocket_accept(key.as_bytes(), &mut buffer);
        assert_eq!(accept, b"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn test_build_uri() {
        let socket_addr = "127.0.0.1:8000".parse().unwrap();
        let path_and_query = "/websocket".parse().unwrap();
        let path = Some(&path_and_query);
        let result = build_uri(socket_addr, path).unwrap();
        assert_eq!(result, "ws://127.0.0.1:8000/websocket");
    }

    #[test]
    fn build_uri_no_path() {
        let socket_addr = "127.0.0.1:8000".parse().unwrap();
        let path = None;
        let result = build_uri(socket_addr, path).unwrap();
        assert_eq!(result, "ws://127.0.0.1:8000");
    }

    #[tokio::test]
    async fn test_init_upgrade_from_request() {
        struct MockClient {
            socket_addr: SocketAddr,
        }

        impl ExtractSocketAddr for MockClient {
            fn socket_addr(&self) -> SocketAddr {
                self.socket_addr
            }
        }

        let socket_addr = get_available_socket(20).await.unwrap();

        let client = MockClient { socket_addr };

        let server = tokio::spawn(async move {
            dummy_websocket_server::run(socket_addr).await.unwrap();
        });

        let uri = Uri::builder()
            .scheme("http")
            .authority(socket_addr.to_string().as_str())
            .path_and_query("/")
            .build()
            .unwrap();

        let req = Request::builder()
            .uri(uri)
            .header(UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .body(())
            .unwrap();

        let result = init_upgrade(req, client).await.unwrap();

        server.abort();

        assert_eq!(result.status(), StatusCode::SWITCHING_PROTOCOLS);
        assert_eq!(
            result.headers().get(UPGRADE).unwrap(),
            HeaderValue::from_static("websocket")
        );
        assert_eq!(
            result.headers().get(SEC_WEBSOCKET_ACCEPT).unwrap(),
            HeaderValue::from_static("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=")
        );
        assert_eq!(
            result.headers().get(hyper::header::CONNECTION).unwrap(),
            HeaderValue::from_static("Upgrade")
        );
    }

    #[tokio::test]
    async fn test_init_upgrade_from_request_no_sec_key() {
        struct MockClient {
            socket_addr: SocketAddr,
        }

        impl ExtractSocketAddr for MockClient {
            fn socket_addr(&self) -> SocketAddr {
                self.socket_addr
            }
        }

        let socket_addr = get_available_socket(20).await.unwrap();

        let client = MockClient { socket_addr };

        let server = tokio::spawn(async move {
            dummy_websocket_server::run(socket_addr).await.unwrap();
        });

        let uri = Uri::builder()
            .scheme("http")
            .authority(socket_addr.to_string().as_str())
            .path_and_query("/")
            .build()
            .unwrap();

        let req = Request::builder()
            .uri(uri)
            .header(UPGRADE, "websocket")
            .body(())
            .unwrap();

        let result = init_upgrade(req, client).await;

        server.abort();

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_attempt_upgrade_no_upgrade_header() {
        struct MockClient {
            socket_addr: SocketAddr,
        }

        impl ExtractSocketAddr for MockClient {
            fn socket_addr(&self) -> SocketAddr {
                self.socket_addr
            }
        }

        let socket_addr = get_available_socket(20).await.unwrap();

        let client = MockClient { socket_addr };

        let server = tokio::spawn(async move {
            dummy_websocket_server::run(socket_addr).await.unwrap();
        });

        let uri = Uri::builder()
            .scheme("http")
            .authority(socket_addr.to_string().as_str())
            .path_and_query("/")
            .build()
            .unwrap();

        let req = Request::builder()
            .uri(uri)
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .body(())
            .unwrap();

        let result = attempt_upgrade(req, client).await.unwrap();

        server.abort();

        match result {
            UpgradeStatus::NotUpgraded(_) => {}
            _ => panic!("Expected NotUpgraded"),
        }
    }

    #[tokio::test]
    async fn test_attempt_upgrade_with_upgrade_header() {
        struct MockClient {
            socket_addr: SocketAddr,
        }

        impl ExtractSocketAddr for MockClient {
            fn socket_addr(&self) -> SocketAddr {
                self.socket_addr
            }
        }

        let socket_addr = get_available_socket(20).await.unwrap();

        let client = MockClient { socket_addr };

        let server = tokio::spawn(async move {
            dummy_websocket_server::run(socket_addr).await.unwrap();
        });

        let uri = Uri::builder()
            .scheme("http")
            .authority(socket_addr.to_string().as_str())
            .path_and_query("/")
            .build()
            .unwrap();

        let req = Request::builder()
            .uri(uri)
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header(UPGRADE, "websocket")
            .body(())
            .unwrap();

        let result = attempt_upgrade(req, client).await.unwrap();

        server.abort();

        match result {
            UpgradeStatus::Upgraded(res) => {
                assert_eq!(res.status(), StatusCode::SWITCHING_PROTOCOLS);
                assert_eq!(
                    res.headers().get(UPGRADE).unwrap(),
                    HeaderValue::from_static("websocket")
                );
                assert_eq!(
                    res.headers().get(SEC_WEBSOCKET_ACCEPT).unwrap(),
                    HeaderValue::from_static("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=")
                );
                assert_eq!(
                    res.headers().get(hyper::header::CONNECTION).unwrap(),
                    HeaderValue::from_static("Upgrade")
                );
            }
            _ => panic!("Expected NotUpgraded"),
        }
    }

    #[tokio::test]
    async fn test_upgrade_connection_from_request() {
        struct MockClient {
            socket_addr: SocketAddr,
        }

        impl ExtractSocketAddr for MockClient {
            fn socket_addr(&self) -> SocketAddr {
                self.socket_addr
            }
        }

        let socket_addr = get_available_socket(20).await.unwrap();

        let client = MockClient { socket_addr };

        let server = tokio::spawn(async move {
            dummy_websocket_server::run(socket_addr).await.unwrap();
        });

        let uri = Uri::builder()
            .scheme("http")
            .authority(socket_addr.to_string().as_str())
            .path_and_query("/")
            .build()
            .unwrap();

        let req = Request::builder()
            .uri(uri)
            .header(UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .body(())
            .unwrap();

        let _ = tokio::spawn(async move {
            let result = upgrade_connection_from_request(req, client).await;
            assert!(result.is_ok());
        })
        .await;

        server.abort();
    }

    mod dummy_websocket_server {
        use std::{io::Error, net::SocketAddr};

        use futures_util::{future, StreamExt, TryStreamExt};
        use log::info;
        use tokio::net::{TcpListener, TcpStream};

        pub async fn run(addr: SocketAddr) -> Result<(), Error> {
            // Create the event loop and TCP listener we'll accept connections on.
            let try_socket = TcpListener::bind(&addr).await;
            let listener = try_socket.expect("Failed to bind");
            info!("Listening on: {}", addr);

            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(accept_connection(stream));
            }

            Ok(())
        }

        async fn accept_connection(stream: TcpStream) {
            let addr = stream
                .peer_addr()
                .expect("connected streams should have a peer address");
            info!("Peer address: {}", addr);

            let ws_stream = tokio_tungstenite::accept_async(stream)
                .await
                .expect("Error during the websocket handshake occurred");

            info!("New WebSocket connection: {}", addr);

            let (write, read) = ws_stream.split();
            // We should not forward messages other than text or binary.
            read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
                .forward(write)
                .await
                .expect("Failed to forward messages")
        }
    }
}
