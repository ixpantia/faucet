use std::{net::SocketAddr, ops::RangeInclusive};

use rand::Rng;

use tokio::{io, net::TcpListener};

use crate::error::{FaucetError, FaucetResult};

const UNSAFE_PORTS: &[u16] = &[
    1, 7, 9, 11, 13, 15, 17, 19, 20, 21, 22, 23, 25, 37, 42, 43, 53, 77, 79, 87, 95, 101, 102, 103,
    104, 109, 110, 111, 113, 115, 117, 119, 123, 135, 139, 143, 179, 389, 427, 465, 512, 513, 514,
    515, 526, 530, 531, 532, 540, 548, 556, 563, 587, 601, 636, 993, 995, 2049, 3659, 4045, 6000,
    6665, 6666, 6667, 6668, 6669, 6697,
];

pub async fn socket_is_available(socket_addr: SocketAddr) -> FaucetResult<bool> {
    let result = TcpListener::bind(socket_addr).await;
    match result {
        Ok(_) => Ok(true),
        Err(e) => match e.kind() {
            io::ErrorKind::AddrInUse => Ok(false),
            _ => Err(FaucetError::Io(e)),
        },
    }
}

const PORT_RANGE: RangeInclusive<u16> = 1024..=49151;

pub async fn get_available_socket(tries: usize) -> Result<SocketAddr, FaucetError> {
    let mut rng = rand::rng();

    for _ in 0..tries {
        let port: u16 = rng.random_range(PORT_RANGE);

        let socket_addr = SocketAddr::from(([127, 0, 0, 1], port));

        if UNSAFE_PORTS.contains(&port) {
            continue;
        }

        if socket_is_available(socket_addr).await? {
            return Ok(socket_addr);
        }
    }

    Err(FaucetError::NoSocketsAvailable)
}
