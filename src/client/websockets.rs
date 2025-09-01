use super::{pool::ExtractSocketAddr, Client, ExclusiveBody};
use crate::{
    error::{BadRequestReason, FaucetError, FaucetResult},
    global_conn::{add_connection, remove_connection},
    server::logging::{EventLogData, FaucetTracingLevel},
    shutdown::ShutdownSignal,
    telemetry::send_log_event,
};
use base64::Engine;
use bytes::Bytes;
use futures_util::StreamExt;
use hyper::{
    header::UPGRADE,
    http::{uri::PathAndQuery, HeaderValue},
    upgrade::Upgraded,
    HeaderMap, Request, Response, StatusCode, Uri,
};
use hyper_util::rt::TokioIo;
use serde_json::json;
use sha1::{Digest, Sha1};
use std::{
    collections::HashMap, future::Future, net::SocketAddr, str::FromStr, sync::LazyLock,
    time::Duration,
};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Message, Utf8Bytes,
};
use uuid::Uuid;

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

// We want to keep the shiny tx and rx in memory in case the upgraded connection is dropped. If the user reconnect we want to immediately
// re establish the connection back to shiny
use futures_util::SinkExt;

type ConnectionPair = (
    futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Message,
    >,
    futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
);

#[derive(Default)]
struct ConnectionInstance {
    purged: bool,
    access_count: usize,
    pair: Option<ConnectionPair>,
}

impl ConnectionInstance {
    fn take(&mut self) -> ConnectionPair {
        self.access_count += 1;
        self.pair.take().unwrap()
    }
    fn put_back(&mut self, pair: ConnectionPair) {
        self.access_count += 1;
        self.pair = Some(pair);
    }
}

struct ConnectionManagerInner {
    map: HashMap<Uuid, ConnectionInstance>,
    purge_count: usize,
}

struct ConnectionManager {
    inner: Mutex<ConnectionManagerInner>,
}

impl ConnectionManager {
    fn new() -> Self {
        ConnectionManager {
            inner: Mutex::new(ConnectionManagerInner {
                map: HashMap::new(),
                purge_count: 0,
            }),
        }
    }
    async fn initialize_if_not(
        &self,
        session_id: Uuid,
        attempt: usize,
        init: impl Future<Output = FaucetResult<ConnectionPair>>,
    ) -> Option<FaucetResult<ConnectionPair>> {
        {
            let mut inner = self.inner.lock().await;
            let entry = inner.map.entry(session_id).or_default();
            if entry.access_count != 0 {
                return None;
            }
            if entry.purged {
                return Some(Err(FaucetError::WebSocketConnectionPurged));
            }

            if entry.access_count == 0 && attempt > 0 {
                return Some(Err(FaucetError::WebSocketConnectionPurged));
            }

            entry.access_count += 1;
        }
        let connection_pair = match init.await {
            Ok(connection_pair) => connection_pair,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok(connection_pair))
    }
    async fn attempt_take(&self, session_id: Uuid) -> FaucetResult<ConnectionPair> {
        match self.inner.try_lock() {
            Ok(mut inner) => {
                let instance = inner.map.entry(session_id).or_default();

                if instance.access_count % 2 == 0 {
                    return Ok(instance.take());
                }

                Err(FaucetError::WebSocketConnectionInUse)
            }
            _ => Err(FaucetError::WebSocketConnectionInUse),
        }
    }
    async fn put_pack(&self, session_id: Uuid, pair: ConnectionPair) {
        let mut inner = self.inner.lock().await;
        if let Some(instance) = inner.map.get_mut(&session_id) {
            instance.put_back(pair);
        }
    }
    async fn remove_session(&self, session_id: Uuid) {
        let mut inner = self.inner.lock().await;
        inner.map.remove(&session_id);
        inner.purge_count += 1;
        if let Some(instance) = inner.map.get_mut(&session_id) {
            instance.purged = true;
        }
    }
}

// Note: This is a simplified cache for a single shiny connection using a static Mutex.
// A more robust solution would use a session identifier to cache multiple connections.
// We use a std::sync::Mutex as the lock is not held across .await points.
static SHINY_CONNECTION_CACHE: LazyLock<ConnectionManager> = LazyLock::new(ConnectionManager::new);

async fn connect_to_worker(
    mut upgrade_info: UpgradeInfo,
    session_id: Uuid,
) -> FaucetResult<ConnectionPair> {
    let mut request = Request::builder().uri(upgrade_info.uri).body(())?;
    upgrade_info.headers.append(
        "FAUCET_SESSION_ID",
        HeaderValue::from_str(&session_id.to_string())
            .expect("Unable to set Session ID as header. This is a bug. please report it!"),
    );
    *request.headers_mut() = upgrade_info.headers;
    let (shiny_ws, _) = tokio_tungstenite::connect_async(request).await?;
    send_log_event(EventLogData {
        target: "faucet".into(),
        event_id: session_id,
        parent_event_id: None,
        level: FaucetTracingLevel::Info,
        event_type: "websocket_connection".into(),
        message: "Established new WebSocket connection to shiny".to_string(),
        body: None,
    });
    Ok(shiny_ws.split())
}

async fn connect_or_retrieve(
    upgrade_info: UpgradeInfo,
    session_id: Uuid,
    attempt: usize,
) -> FaucetResult<ConnectionPair> {
    let init_pair = SHINY_CONNECTION_CACHE
        .initialize_if_not(
            session_id,
            attempt,
            connect_to_worker(upgrade_info, session_id),
        )
        .await;

    match init_pair {
        None => {
            // This means that the connection has already been initialized
            // in the past
            match SHINY_CONNECTION_CACHE.attempt_take(session_id).await {
                Ok(con) => {
                    send_log_event(EventLogData {
                        target: "faucet".into(),
                        event_id: Uuid::new_v4(),
                        parent_event_id: Some(session_id),
                        event_type: "websocket_connection".into(),
                        level: FaucetTracingLevel::Info,
                        message: "Client successfully reconnected".to_string(),
                        body: Some(json!({"attempts": attempt})),
                    });
                    Ok(con)
                }
                Err(e) => FaucetResult::Err(e),
            }
        }
        Some(init_pair_res) => init_pair_res,
    }
}

const RECHECK_TIME: Duration = Duration::from_secs(60);
const PING_INTERVAL: Duration = Duration::from_secs(1);
const PING_INTERVAL_TIMEOUT: Duration = Duration::from_secs(30);
const PING_BYTES: Bytes = Bytes::from_static(b"Ping");

async fn server_upgraded_io(
    upgraded: Upgraded,
    upgrade_info: UpgradeInfo,
    session_id: Uuid,
    attempt: usize,
    shutdown: &'static ShutdownSignal,
) -> FaucetResult<()> {
    // Set up the WebSocket connection with the client.
    let upgraded = TokioIo::new(upgraded);
    let upgraded_ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        upgraded,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    )
    .await;
    let (mut upgraded_tx, mut upgraded_rx) = upgraded_ws.split();

    // Attempt to retrieve a cached connection to Shiny.
    let (mut shiny_tx, mut shiny_rx) =
        match connect_or_retrieve(upgrade_info, session_id, attempt).await {
            Ok(pair) => pair,
            Err(e) => match e {
                FaucetError::WebSocketConnectionPurged => {
                    upgraded_tx
                        .send(Message::Close(Some(CloseFrame {
                            code: CloseCode::Normal,
                            reason: Utf8Bytes::from_static(
                                "Connection purged due to inactivity, update or error.",
                            ),
                        })))
                        .await?;
                    return Err(FaucetError::WebSocketConnectionPurged);
                }
                e => return Err(e),
            },
        };

    // Manually pump messages in both directions.
    // This allows us to regain ownership of the streams after a disconnect.
    let client_to_shiny = async {
        loop {
            log::debug!("Waiting for message or ping timeout");
            tokio::select! {
                msg = upgraded_rx.next() => {
                    log::debug!("Received msg: {msg:?}");
                    match msg {
                        Some(Ok(msg)) => {
                            if shiny_tx.send(msg).await.is_err() {
                                break; // Shiny connection closed
                            }
                        },
                        _ => break
                    }
                },
                _ = tokio::time::sleep(PING_INTERVAL_TIMEOUT) => {
                    log::debug!("Ping timeout reached for session {session_id}");
                    break;
                }
            }
        }
    };

    let shiny_to_client = async {
        loop {
            let ping_future = async {
                tokio::time::sleep(PING_INTERVAL).await;
                upgraded_tx.send(Message::Ping(PING_BYTES)).await
            };
            tokio::select! {
                msg = shiny_rx.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            if upgraded_tx.send(msg).await.is_err() {
                                break; // Client connection closed
                            }
                        },
                        _ => break
                    }
                },
                _ = ping_future => {}
            }
        }
    };

    // Wait for either the client or Shiny to disconnect.
    tokio::select! {
        _ = client_to_shiny => {
            send_log_event(EventLogData {
                target: "faucet".into(),
                event_id: Uuid::new_v4(),
                parent_event_id: Some(session_id),
                event_type: "websocket_connection".into(),
                level: FaucetTracingLevel::Info,
                message: "Session ended by client.".to_string(),
                body: None,
            });
            log::debug!("Client connection closed for session {session_id}.")
        },
        _ = shiny_to_client => {
            // If this happens that means shiny ended the session, immediately
            // remove the session from the cache
            SHINY_CONNECTION_CACHE.remove_session(session_id).await;
            send_log_event(EventLogData {
                target: "faucet".into(),
                event_id: Uuid::new_v4(),
                parent_event_id: Some(session_id),
                event_type: "websocket_connection".into(),
                level: FaucetTracingLevel::Info,
                message: "Shiny session ended by Shiny.".to_string(),
                body: None,
            });
            log::debug!("Shiny connection closed for session {session_id}.");
            return Ok(());
        },
        _ = shutdown.wait() => {
            log::debug!("Received shutdown signal. Exiting websocket bridge.");
            return Ok(());
        }
    };

    // Getting here meant that the only possible way the session ended is if
    // the client ended the connection

    log::debug!("Client websocket connection to session {session_id} ended but the Shiny connection is still alive. Saving for reconnection.");
    SHINY_CONNECTION_CACHE
        .put_pack(session_id, (shiny_tx, shiny_rx))
        .await;

    // Schedule a check in 30 seconds. If the connection is not in use
    tokio::select! {
        _ = tokio::time::sleep(RECHECK_TIME) => {
            let entry = SHINY_CONNECTION_CACHE.attempt_take(session_id).await;
            match entry {
                Err(_) => (),
                Ok((shiny_tx, shiny_rx)) => {
                    let mut ws = shiny_tx
                        .reunite(shiny_rx)
                        .expect("shiny_rx and tx always have the same origin.");
                    //
                    if ws
                        .close(Some(CloseFrame {
                            code: CloseCode::Abnormal,
                            reason: Utf8Bytes::default(),
                        }))
                        .await
                        .is_ok()
                    {
                        log::debug!("Closed reserved connection for session {session_id}");
                    }
                    SHINY_CONNECTION_CACHE.remove_session(session_id).await;
                }
            }
        },
        _ = shutdown.wait() => {
            log::debug!("Shutdown signaled, not running websocket cleanup for session {session_id}");
        }
    }

    Ok(())
}

pub enum UpgradeStatus<ReqBody> {
    Upgraded(Response<ExclusiveBody>),
    NotUpgraded(Request<ReqBody>),
}

const SESSION_ID_QUERY: &str = "sessionId";

async fn upgrade_connection_from_request<ReqBody>(
    mut req: Request<ReqBody>,
    client: impl ExtractSocketAddr,
    shutdown: &'static ShutdownSignal,
) -> FaucetResult<()> {
    // Extract sessionId query parameter
    let query = req.uri().query().ok_or(FaucetError::BadRequest(
        BadRequestReason::MissingQueryParam("sessionId"),
    ))?;

    let mut session_id: Option<uuid::Uuid> = None;
    let mut attempt: Option<usize> = None;

    url::form_urlencoded::parse(query.as_bytes()).for_each(|(key, value)| {
        if key == SESSION_ID_QUERY {
            session_id = uuid::Uuid::from_str(&value).ok();
        } else if key == "attempt" {
            attempt = value.parse::<usize>().ok();
        }
    });

    let session_id = session_id.ok_or(FaucetError::BadRequest(
        BadRequestReason::MissingQueryParam("sessionId"),
    ))?;

    let attempt = attempt.ok_or(FaucetError::BadRequest(
        BadRequestReason::MissingQueryParam("attempt"),
    ))?;

    let upgrade_info = UpgradeInfo::new(&req, client.socket_addr())?;
    let upgraded = hyper::upgrade::on(&mut req).await?;
    server_upgraded_io(upgraded, upgrade_info, session_id, attempt, shutdown).await?;
    Ok(())
}

async fn init_upgrade<ReqBody: Send + Sync + 'static>(
    req: Request<ReqBody>,
    client: impl ExtractSocketAddr + Send + Sync + 'static,
    shutdown: &'static ShutdownSignal,
) -> FaucetResult<Response<ExclusiveBody>> {
    let mut res = Response::new(ExclusiveBody::empty());
    let sec_websocket_key = req
        .headers()
        .get(SEC_WEBSOCKET_KEY)
        .cloned()
        .ok_or(FaucetError::no_sec_web_socket_key())?;
    tokio::task::spawn(async move {
        add_connection();
        if let Err(e) = upgrade_connection_from_request(req, client, shutdown).await {
            log::error!("upgrade error: {e:?}");
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
    shutdown: &'static ShutdownSignal,
) -> FaucetResult<UpgradeStatus<ReqBody>> {
    if req.headers().contains_key(UPGRADE) {
        return Ok(UpgradeStatus::Upgraded(
            init_upgrade(req, client, shutdown).await?,
        ));
    }
    Ok(UpgradeStatus::NotUpgraded(req))
}

impl Client {
    pub async fn attempt_upgrade<ReqBody>(
        &self,
        req: Request<ReqBody>,
        shutdown: &'static ShutdownSignal,
    ) -> FaucetResult<UpgradeStatus<ReqBody>>
    where
        ReqBody: Send + Sync + 'static,
    {
        attempt_upgrade(req, self.clone(), shutdown).await
    }
}

#[cfg(test)]
mod tests {
    use crate::{leak, networking::get_available_socket, shutdown::ShutdownSignal};

    use super::*;
    use uuid::Uuid;

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
            .path_and_query(format!("/?{}={}", SESSION_ID_QUERY, Uuid::now_v7()))
            .build()
            .unwrap();

        let req = Request::builder()
            .uri(uri.clone())
            .header(UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .body(())
            .unwrap();

        let shutdown = leak!(ShutdownSignal::new());
        let result = init_upgrade(req, client, shutdown).await.unwrap();

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
            .path_and_query(format!("/?{}={}", SESSION_ID_QUERY, Uuid::now_v7()))
            .build()
            .unwrap();

        let req = Request::builder()
            .uri(uri.clone())
            .header(UPGRADE, "websocket")
            .body(())
            .unwrap();

        let shutdown = leak!(ShutdownSignal::new());
        let result = init_upgrade(req, client, shutdown).await;

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

        let shutdown = leak!(ShutdownSignal::new());
        let result = attempt_upgrade(req, client, shutdown).await.unwrap();

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
            .path_and_query(format!("/?{}={}", SESSION_ID_QUERY, Uuid::now_v7()))
            .build()
            .unwrap();

        let req = Request::builder()
            .uri(uri)
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header(UPGRADE, "websocket")
            .body(())
            .unwrap();

        let shutdown = leak!(ShutdownSignal::new());
        let result = attempt_upgrade(req, client, shutdown).await.unwrap();

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
            _ => panic!("Expected Upgraded"),
        }
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
            info!("Listening on: {addr}");

            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(accept_connection(stream));
            }

            Ok(())
        }

        async fn accept_connection(stream: TcpStream) {
            let addr = stream
                .peer_addr()
                .expect("connected streams should have a peer address");
            info!("Peer address: {addr}");

            let ws_stream = tokio_tungstenite::accept_async(stream)
                .await
                .expect("Error during the websocket handshake occurred");

            info!("New WebSocket connection: {addr}");

            let (write, read) = ws_stream.split();
            // We should not forward messages other than text or binary.
            read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
                .forward(write)
                .await
                .expect("Failed to forward messages")
        }
    }
}
