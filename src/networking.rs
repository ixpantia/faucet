use std::net::SocketAddr;

use crate::error::FaucetResult;

pub async fn get_available_socket() -> FaucetResult<SocketAddr> {
    use tokio::net::TcpListener;
    TcpListener::bind("127.0.0.1:0")
        .await?
        .local_addr()
        .map_err(Into::into)
}
