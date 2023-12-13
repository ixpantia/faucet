use super::body::ExclusiveBody;
use crate::error::{FaucetError, FaucetResult};
use crate::worker::WorkerState;
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

async fn create_http_client(addr: SocketAddr) -> FaucetResult<ConnectionHandle> {
    let stream = TokioIo::new(TcpStream::connect(addr).await?);
    let (sender, conn) = hyper::client::conn::http1::handshake(stream).await?;
    tokio::spawn(async move {
        conn.await.expect("client conn");
    });
    Ok(ConnectionHandle { sender })
}

struct ConnectionManager {
    addr: SocketAddr,
}

impl ConnectionManager {
    fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

#[async_trait]
impl managed::Manager for ConnectionManager {
    type Type = ConnectionHandle;
    type Error = FaucetError;

    async fn create(&self) -> FaucetResult<Self::Type> {
        create_http_client(self.addr).await
    }

    async fn recycle(
        &self,
        conn: &mut ConnectionHandle,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<FaucetError> {
        if conn.sender.is_closed() {
            Err(RecycleError::StaticMessage("Connection closed"))
        } else {
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
    worker_state: Option<WorkerState>,
}

const DEFAULT_MAX_SIZE: usize = 32;

impl ClientBuilder {
    pub fn build(self) -> FaucetResult<Client> {
        let worker_state = self
            .worker_state
            .expect("Unable to create connection without worker state");
        let builder = Pool::builder(ConnectionManager::new(worker_state.socket_addr()))
            .max_size(self.max_size.unwrap_or(DEFAULT_MAX_SIZE));
        Ok(Client {
            pool: builder.build()?,
            worker_state,
        })
    }
}

#[derive(Clone)]
pub(crate) struct Client {
    pool: Pool<ConnectionManager>,
    worker_state: WorkerState,
}

impl Client {
    pub fn builder(worker_state: WorkerState) -> ClientBuilder {
        ClientBuilder {
            max_size: None,
            worker_state: Some(worker_state),
        }
    }
    pub fn socket_addr(&self) -> SocketAddr {
        self.worker_state.socket_addr()
    }
    pub async fn get(&self) -> FaucetResult<HttpConnection> {
        Ok(HttpConnection {
            inner: self.pool.get().await?,
        })
    }
    pub fn is_online(&self) -> bool {
        self.worker_state.is_online()
    }
}
