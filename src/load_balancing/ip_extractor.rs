use crate::error::{BadRequestReason, FaucetError, FaucetResult};
use std::net::{IpAddr, SocketAddr};

use hyper::{body::Incoming, http::HeaderValue, Request};

#[derive(Clone, Copy)]
pub enum IpExtractor {
    ClientAddr,
    XForwardedFor,
    XRealIp,
}

const MISSING_X_FORWARDED_FOR: FaucetError =
    FaucetError::BadRequest(BadRequestReason::MissingHeader("X-Forwarded-For"));

const INVALID_X_FORWARDED_FOR: FaucetError =
    FaucetError::BadRequest(BadRequestReason::InvalidHeader("X-Forwarded-For"));

fn extract_ip_from_x_forwarded_for(x_forwarded_for: &HeaderValue) -> FaucetResult<IpAddr> {
    let x_forwarded_for = x_forwarded_for
        .to_str()
        .map_err(|_| MISSING_X_FORWARDED_FOR)?;
    let ip_str = x_forwarded_for
        .split(',')
        .next()
        .map(|ip| ip.trim())
        .ok_or(INVALID_X_FORWARDED_FOR)?;
    ip_str.parse().map_err(|_| INVALID_X_FORWARDED_FOR)
}

const MISSING_X_REAL_IP: FaucetError =
    FaucetError::BadRequest(BadRequestReason::MissingHeader("X-Real-IP"));

const INVALID_X_REAL_IP: FaucetError =
    FaucetError::BadRequest(BadRequestReason::InvalidHeader("X-Real-IP"));

fn extract_ip_from_x_real_ip(x_real_ip: &HeaderValue) -> FaucetResult<IpAddr> {
    let x_real_ip = x_real_ip.to_str().map_err(|_| MISSING_X_REAL_IP)?;
    x_real_ip.parse().map_err(|_| INVALID_X_REAL_IP)
}

impl IpExtractor {
    pub fn extract(self, req: &Request<Incoming>, client_addr: SocketAddr) -> FaucetResult<IpAddr> {
        use IpExtractor::*;
        let ip = match self {
            ClientAddr => client_addr.ip(),
            XForwardedFor => match req.headers().get("X-Forwarded-For") {
                Some(header) => extract_ip_from_x_forwarded_for(header)?,
                None => return Err(MISSING_X_FORWARDED_FOR),
            },
            XRealIp => match req.headers().get("X-Real-IP") {
                Some(header) => extract_ip_from_x_real_ip(header)?,
                None => return Err(MISSING_X_REAL_IP),
            },
        };
        Ok(ip)
    }
}
