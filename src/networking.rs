use std::net::SocketAddr;

use tokio::net::TcpListener;

use crate::error::FaucetResult;

pub async fn get_available_sockets(n: usize) -> impl Iterator<Item = SocketAddr> {
    let mut tcp_listeners = Vec::with_capacity(n);
    for _ in 0..n {
        match bind_to_random_port().await {
            Ok(tcp_listener) => {
                let tcp_listener = TempTcpListener(tcp_listener);
                log::debug!(target: "faucet", "Reserving SocketAddr {}", tcp_listener.local_addr());
                tcp_listeners.push(tcp_listener);
            }
            Err(e) => {
                log::error!(target: "faucet", "Failed to bind to random port: {}", e);
                std::process::exit(1);
            }
        }
    }
    tcp_listeners
        .into_iter()
        .map(|tcp_listener| tcp_listener.local_addr())
}

pub struct TempTcpListener(TcpListener);

impl Drop for TempTcpListener {
    fn drop(&mut self) {
        log::debug!(target: "faucet", "Dropping bind to socket {}, a child process can now bind to it", self.0.local_addr().expect("Failed to get local addr"));
    }
}

impl TempTcpListener {
    fn local_addr(&self) -> SocketAddr {
        match self.0.local_addr() {
            Ok(addr) => addr,
            Err(e) => {
                log::error!(target: "faucet", "Failed to get local addr: {}", e);
                std::process::exit(1);
            }
        }
    }
}

async fn bind_to_random_port() -> FaucetResult<TcpListener> {
    TcpListener::bind("127.0.0.1:0").await.map_err(Into::into)
}
