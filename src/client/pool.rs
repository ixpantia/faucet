use super::body::ExclusiveBody;
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

pub struct ClientBuilder {
    max_size: Option<usize>,
    addr: Option<SocketAddr>,
}

const DEFAULT_MAX_SIZE: usize = 32;

impl ClientBuilder {
    pub fn build(self) -> FaucetResult<Client> {
        let addr = self
            .addr
            .expect("Unable to create connection, no SocketAddr");
        let builder = Pool::builder(ConnectionManager::new(addr))
            .max_size(self.max_size.unwrap_or(DEFAULT_MAX_SIZE));
        Ok(Client {
            pool: builder.build()?,
            socket_addr: addr,
        })
    }
}

#[derive(Clone)]
pub struct Client {
    pool: Pool<ConnectionManager>,
    socket_addr: SocketAddr,
}

impl Client {
    pub fn builder(addr: SocketAddr) -> ClientBuilder {
        ClientBuilder {
            max_size: None,
            addr: Some(addr),
        }
    }
    pub fn socket_addr(&self) -> SocketAddr {
        self.socket_addr
    }
    pub async fn get(&self) -> FaucetResult<HttpConnection> {
        Ok(HttpConnection {
            inner: self.pool.get().await?,
        })
    }
}
