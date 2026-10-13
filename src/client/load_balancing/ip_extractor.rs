use crate::error::{BadRequestReason, FaucetError, FaucetResult};
use hyper::{http::HeaderValue, Request};
use std::net::{IpAddr, SocketAddr};

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename = "snake_case")]
pub enum IpExtractor {
    ClientAddr,
    XForwardedFor,
    XRealIp,
}

/// Parses a forwarded client address.
///
/// Some reverse proxies (Azure App Service, for one) forward `ip:port`
/// rather than a bare IP address, so a plain `IpAddr` parse rejects every
/// request. Accept a bare address first, then fall back to a socket address
/// and keep only its IP. `SocketAddr` parsing also covers the bracketed IPv6
/// form (`[2001:db8::1]:443`).
fn parse_forwarded_ip(ip_str: &str) -> Option<IpAddr> {
    ip_str
        .parse::<IpAddr>()
        .ok()
        .or_else(|| ip_str.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
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
    parse_forwarded_ip(ip_str).ok_or(INVALID_X_FORWARDED_FOR)
}

const MISSING_X_REAL_IP: FaucetError =
    FaucetError::BadRequest(BadRequestReason::MissingHeader("X-Real-IP"));

const INVALID_X_REAL_IP: FaucetError =
    FaucetError::BadRequest(BadRequestReason::InvalidHeader("X-Real-IP"));

fn extract_ip_from_x_real_ip(x_real_ip: &HeaderValue) -> FaucetResult<IpAddr> {
    let x_real_ip = x_real_ip.to_str().map_err(|_| MISSING_X_REAL_IP)?;
    parse_forwarded_ip(x_real_ip.trim()).ok_or(INVALID_X_REAL_IP)
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
    fn extract_ip_from_x_forwarded_for_ipv4_with_port() {
        // Azure App Service forwards `ip:port`.
        let header_value = HeaderValue::from_static("203.0.113.7:51234, 10.0.0.1");
        let ip = extract_ip_from_x_forwarded_for(&header_value).unwrap();
        assert_eq!(ip, IpAddr::from([203, 0, 113, 7]));
    }

    #[test]
    fn extract_ip_from_x_forwarded_for_ipv6_with_port() {
        let header_value = HeaderValue::from_static("[2001:db8::1]:443");
        let ip = extract_ip_from_x_forwarded_for(&header_value).unwrap();
        assert_eq!(ip, "2001:db8::1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn extract_ip_from_x_forwarded_for_garbage_is_invalid() {
        let header_value = HeaderValue::from_static("not-an-ip:12");
        assert!(extract_ip_from_x_forwarded_for(&header_value).is_err());
    }

    #[test]
    fn extract_ip_from_x_real_ip_with_port() {
        let header_value = HeaderValue::from_static("203.0.113.7:51234");
        let ip = extract_ip_from_x_real_ip(&header_value).unwrap();
        assert_eq!(ip, IpAddr::from([203, 0, 113, 7]));
    }

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
            .extract(&request, Some(IpAddr::from([0, 0, 0, 0])))
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
            .extract(&request, Some(IpAddr::from([0, 0, 0, 0])))
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
            .extract(&request, Some(IpAddr::from([0, 0, 0, 0])))
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
            .extract(&request, Some(IpAddr::from([0, 0, 0, 0])))
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
            .extract(&request, Some(IpAddr::from([0, 0, 0, 0])))
            .unwrap();
        assert_eq!(ip, IpAddr::from([192, 168, 0, 1]));
    }

    #[test]
    fn extract_client_addr_ipv4_from_request() {
        let request = Request::builder().body(()).unwrap();
        let ip = IpExtractor::ClientAddr
            .extract(&request, Some(IpAddr::from([127, 0, 0, 1])))
            .unwrap();
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }

    #[test]
    fn extract_client_addr_ipv6_from_request() {
        let request = Request::builder().body(()).unwrap();
        let ip = IpExtractor::ClientAddr
            .extract(&request, Some(IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1])))
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
            .extract(&request, Some(IpAddr::from([127, 0, 0, 1])))
            .unwrap();
        assert_eq!(ip, IpAddr::from([127, 0, 0, 1]));
    }
}
