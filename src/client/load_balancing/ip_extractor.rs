use crate::error::{BadRequestReason, FaucetError, FaucetResult};
use hyper::{http::HeaderValue, Request};
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename = "snake_case")]
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
    pub fn extract<B>(self, req: &Request<B>, client_addr: Option<IpAddr>) -> FaucetResult<IpAddr> {
        use IpExtractor::*;
        let ip = match self {
            ClientAddr => client_addr.expect("Unable to get client address"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_ip_from_x_forwarded_for_ipv4() {
        let header_value = HeaderValue::from_static("127.0.0.1");
        let ip = extract_ip_from_x_forwarded_for(&header_value).unwrap();
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn extract_ip_from_x_forwarded_for_ipv6() {
        let header_value = HeaderValue::from_static("::1");
        let ip = extract_ip_from_x_forwarded_for(&header_value).unwrap();
        assert_eq!(ip, IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]));
    }

    #[test]
    fn extract_ip_from_x_forwarded_for_multiple() {
        let header_value = HeaderValue::from_static("192.168.0.1, 127.0.0.1");
        let ip = extract_ip_from_x_forwarded_for(&header_value).unwrap();
        assert_eq!(ip, IpAddr::from([192, 168, 0, 1]));
    }

    #[test]
    fn extract_x_real_ip_ipv4_from_request() {
        let header_value = HeaderValue::from_static("127.0.0.1");
        let request = Request::builder()
            .header("X-Real-IP", header_value)
            .body(())
            .unwrap();
        let ip = IpExtractor::XRealIp
            .extract(&request, Some(IpAddr::from(([0, 0, 0, 0]))))
            .unwrap();
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn extract_x_real_ip_ipv6_from_request() {
        let header_value = HeaderValue::from_static("::1");
        let request = Request::builder()
            .header("X-Real-IP", header_value)
            .body(())
            .unwrap();
        let ip = IpExtractor::XRealIp
            .extract(&request, Some(IpAddr::from(([0, 0, 0, 0]))))
            .unwrap();
        assert_eq!(ip, IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]));
    }

    #[test]
    fn extract_x_forwarded_for_ipv4_from_request() {
        let header_value = HeaderValue::from_static("127.0.0.1");
        let request = Request::builder()
            .header("X-Forwarded-For", header_value)
            .body(())
            .unwrap();
        let ip = IpExtractor::XForwardedFor
            .extract(&request, Some(IpAddr::from(([0, 0, 0, 0]))))
            .unwrap();
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn extract_x_forwarded_for_ipv6_from_request() {
        let header_value = HeaderValue::from_static("::1");
        let request = Request::builder()
            .header("X-Forwarded-For", header_value)
            .body(())
            .unwrap();
        let ip = IpExtractor::XForwardedFor
            .extract(&request, Some(IpAddr::from(([0, 0, 0, 0]))))
            .unwrap();
        assert_eq!(ip, IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]));
    }

    #[test]
    fn extract_x_forwarded_for_ipv4_from_request_multiple() {
        let header_value = HeaderValue::from_static("192.168.0.1, 127.0.0.1");
        let request = Request::builder()
            .header("X-Forwarded-For", header_value)
            .body(())
            .unwrap();
        let ip = IpExtractor::XForwardedFor
            .extract(&request, Some(IpAddr::from(([0, 0, 0, 0]))))
            .unwrap();
        assert_eq!(ip, IpAddr::from([192, 168, 0, 1]));
    }

    #[test]
    fn extract_client_addr_ipv4_from_request() {
        let request = Request::builder().body(()).unwrap();
        let ip = IpExtractor::ClientAddr
            .extract(&request, Some(IpAddr::from(([127, 0, 0, 1]))))
            .unwrap();
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn extract_client_addr_ipv6_from_request() {
        let request = Request::builder().body(()).unwrap();
        let ip = IpExtractor::ClientAddr
            .extract(&request, Some(IpAddr::from(([0, 0, 0, 0, 0, 0, 0, 1]))))
            .unwrap();
        assert_eq!(ip, IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]));
    }

    #[test]
    fn extract_client_addr_ipv4_with_x_forwarded_for_from_request() {
        let header_value = HeaderValue::from_static("192.168.0.1");
        let request = Request::builder()
            .header("X-Forwarded-For", header_value)
            .body(())
            .unwrap();
        let ip = IpExtractor::ClientAddr
            .extract(&request, Some(IpAddr::from(([127, 0, 0, 1]))))
            .unwrap();
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }
}
