use super::body::ExclusiveBody;
use super::worker::WorkerConfig;
use crate::error::{FaucetError, FaucetResult};
use crate::global_conn::{add_connection, remove_connection};
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

struct ConnectionManager {
    config: &'static WorkerConfig,
}

impl ConnectionManager {
    fn new(config: &'static WorkerConfig) -> Self {
        Self { config }
    }
}

impl managed::Manager for ConnectionManager {
    type Type = ConnectionHandle;
    type Error = FaucetError;

    async fn create(&self) -> FaucetResult<Self::Type> {
        log::debug!(target: "faucet", "Establishing TCP connection to {}", self.config.target);
        let stream = TokioIo::new(TcpStream::connect(self.config.addr).await?);
        let (sender, conn) = hyper::client::conn::http1::handshake(stream).await?;
        tokio::spawn(async move {
            match conn.await {
                Ok(_) => (),
                Err(err) => {
                    log::debug!(target: "faucet", "{err}");
                }
            }
        });
        log::debug!(target: "faucet", "Established TCP connection to {}", self.config.target);
        Ok(ConnectionHandle { sender })
    }

    async fn recycle(
        &self,
        conn: &mut ConnectionHandle,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<FaucetError> {
        if conn.sender.is_closed() {
            Err(RecycleError::message("Connection closed"))
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
        add_connection();
        let (parts, body) = self.inner.sender.send_request(request).await?.into_parts();
        let body = ExclusiveBody::new(body.map_err(Into::into), Some(self));
        Ok(Response::from_parts(parts, body))
    }
}

impl Drop for HttpConnection {
    fn drop(&mut self) {
        remove_connection();
    }
}

pub(crate) struct ClientBuilder {
    max_size: Option<usize>,
    config: Option<WorkerConfig>,
}

const DEFAULT_MAX_SIZE: usize = 1024;

#[derive(Clone)]
pub(crate) struct Client {
    pool: Pool<ConnectionManager>,
    pub(crate) config: &'static WorkerConfig,
}

impl Client {
    pub fn new(config: &'static WorkerConfig) -> Self {
        let builder = Pool::builder(ConnectionManager::new(config)).max_size(DEFAULT_MAX_SIZE);
        let pool = builder
            .build()
            .expect("Failed to create connection pool. This is a bug");
        Self { pool, config }
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
