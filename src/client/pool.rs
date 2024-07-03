use super::body::ExclusiveBody;
use super::worker::WorkerConfig;
use crate::error::{FaucetError, FaucetResult};
use async_trait::async_trait;
use deadpool::managed::{self, Object, Pool, RecycleError};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::client::conn::http1::SendRequest;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpStream;

struct ConnectionHandle {
    sender: SendRequest<Incoming>,
}

async fn create_http_client(config: WorkerConfig) -> FaucetResult<ConnectionHandle> {
    log::debug!(target: "faucet", "Establishing TCP connection to {}", config.target);
    let stream = TokioIo::new(TcpStream::connect(config.addr).await?);
    let (sender, conn) = hyper::client::conn::http1::handshake(stream).await?;
    tokio::spawn(async move {
        conn.await.expect("client conn");
    });
    log::debug!(target: "faucet", "Established TCP connection to {}", config.target);
    Ok(ConnectionHandle { sender })
}

struct ConnectionManager {
    config: WorkerConfig,
}

impl ConnectionManager {
    fn new(config: WorkerConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl managed::Manager for ConnectionManager {
    type Type = ConnectionHandle;
    type Error = FaucetError;

    async fn create(&self) -> FaucetResult<Self::Type> {
        create_http_client(self.config).await
    }

    async fn recycle(
        &self,
        conn: &mut ConnectionHandle,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<FaucetError> {
        if conn.sender.is_closed() {
            Err(RecycleError::StaticMessage("Connection closed"))
        } else {
            log::debug!(target: "faucet", "Recycling TCP connection to {}", self.config.target);
            Ok(())
        }
    }
}

pub struct HttpConnection {
    inner: Object<ConnectionManager>,
}

impl HttpConnection {
    pub async fn send_request(
        mut self,
        request: Request<Incoming>,
    ) -> FaucetResult<Response<ExclusiveBody>> {
        let (parts, body) = self.inner.sender.send_request(request).await?.into_parts();
        let body = ExclusiveBody::new(body.map_err(Into::into), Some(self));
        Ok(Response::from_parts(parts, body))
    }
}

pub(crate) struct ClientBuilder {
    max_size: Option<usize>,
    config: Option<WorkerConfig>,
}

const DEFAULT_MAX_SIZE: usize = 32;

impl ClientBuilder {
    pub fn build(self) -> FaucetResult<Client> {
        let config = self
            .config
            .expect("Unable to create connection without worker state");
        let builder = Pool::builder(ConnectionManager::new(config))
            .max_size(self.max_size.unwrap_or(DEFAULT_MAX_SIZE));
        let pool = builder.build()?;
        Ok(Client { pool, config })
    }
}

#[derive(Clone)]
pub(crate) struct Client {
    pool: Pool<ConnectionManager>,
    pub(crate) config: WorkerConfig,
}

impl Client {
    pub fn builder(config: WorkerConfig) -> ClientBuilder {
        ClientBuilder {
            max_size: None,
            config: Some(config),
        }
    }

    pub async fn get(&self) -> FaucetResult<HttpConnection> {
        Ok(HttpConnection {
            inner: self.pool.get().await?,
        })
    }
    pub fn is_online(&self) -> bool {
        self.config
            .is_online
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub trait ExtractSocketAddr {
    fn socket_addr(&self) -> SocketAddr;
}

impl ExtractSocketAddr for Client {
    #[inline(always)]
    fn socket_addr(&self) -> SocketAddr {
        self.config.addr
    }
}
