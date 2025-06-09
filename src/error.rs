use std::convert::Infallible;

use crate::client::ExclusiveBody;

pub enum BadRequestReason {
    MissingHeader(&'static str),
    InvalidHeader(&'static str),
    NoPathOrQuery,
    NoHostName,
    UnsupportedUrlScheme,
}

pub type FaucetResult<T> = std::result::Result<T, FaucetError>;

pub enum FaucetError {
    PoolBuild(deadpool::managed::BuildError),
    PoolTimeout(deadpool::managed::TimeoutType),
    PoolPostCreateHook,
    PoolClosed,
    PoolNoRuntimeSpecified,
    NoSocketsAvailable,
    ConnectionClosed,
    Io(std::io::Error),
    Unknown(String),
    HostParseError(std::net::AddrParseError),
    Hyper(hyper::Error),
    BadRequest(BadRequestReason),
    InvalidHeaderValues(hyper::header::InvalidHeaderValue),
    Http(hyper::http::Error),
    MissingArgument(&'static str),
    DuplicateRoute(String),
    Utf8Coding,
    BufferCapacity(tokio_tungstenite::tungstenite::error::CapacityError),
    ProtocolViolation(tokio_tungstenite::tungstenite::error::ProtocolError),
    WSWriteBufferFull(tokio_tungstenite::tungstenite::Message),
    PostgreSQL(tokio_postgres::Error),
    AttackAttempt,
}

impl From<tokio_postgres::Error> for FaucetError {
    fn from(value: tokio_postgres::Error) -> Self {
        Self::PostgreSQL(value)
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for FaucetError {
    fn from(value: tokio_tungstenite::tungstenite::Error) -> Self {
        use tokio_tungstenite::tungstenite::error::UrlError;
        use tokio_tungstenite::tungstenite::Error;
        match value {
            Error::Io(err) => FaucetError::Io(err),
            Error::Url(err) => match err {
                UrlError::NoPathOrQuery => FaucetError::BadRequest(BadRequestReason::NoPathOrQuery),
                UrlError::NoHostName | UrlError::EmptyHostName => {
                    FaucetError::BadRequest(BadRequestReason::NoHostName)
                }
                UrlError::TlsFeatureNotEnabled => panic!("TLS Not enabled"),
                UrlError::UnableToConnect(err) => FaucetError::Unknown(err),
                UrlError::UnsupportedUrlScheme => {
                    FaucetError::BadRequest(BadRequestReason::UnsupportedUrlScheme)
                }
            },
            Error::Tls(err) => FaucetError::Unknown(err.to_string()),
            Error::Utf8 => FaucetError::Utf8Coding,
            Error::Http(_) => FaucetError::Unknown("Unknown HTTP error".to_string()),
            Error::Capacity(err) => FaucetError::BufferCapacity(err),
            Error::HttpFormat(err) => FaucetError::Http(err),
            Error::Protocol(err) => FaucetError::ProtocolViolation(err),
            Error::AlreadyClosed | Error::ConnectionClosed => FaucetError::ConnectionClosed,
            Error::AttackAttempt => FaucetError::AttackAttempt,
            Error::WriteBufferFull(msg) => FaucetError::WSWriteBufferFull(msg),
        }
    }
}

impl From<hyper::header::InvalidHeaderValue> for FaucetError {
    fn from(e: hyper::header::InvalidHeaderValue) -> Self {
        Self::InvalidHeaderValues(e)
    }
}

impl From<hyper::http::Error> for FaucetError {
    fn from(e: hyper::http::Error) -> Self {
        Self::Http(e)
    }
}

impl From<deadpool::managed::PoolError<FaucetError>> for FaucetError {
    fn from(value: deadpool::managed::PoolError<FaucetError>) -> Self {
        match value {
            deadpool::managed::PoolError::Backend(e) => e,
            deadpool::managed::PoolError::Timeout(e) => Self::PoolTimeout(e),
            deadpool::managed::PoolError::Closed => Self::PoolClosed,
            deadpool::managed::PoolError::PostCreateHook(_) => Self::PoolPostCreateHook,
            deadpool::managed::PoolError::NoRuntimeSpecified => Self::PoolNoRuntimeSpecified,
        }
    }
}

impl From<Infallible> for FaucetError {
    fn from(_: Infallible) -> Self {
        unreachable!("Infallible error")
    }
}

impl From<deadpool::managed::BuildError> for FaucetError {
    fn from(e: deadpool::managed::BuildError) -> Self {
        Self::PoolBuild(e)
    }
}

impl From<std::io::Error> for FaucetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<std::net::AddrParseError> for FaucetError {
    fn from(e: std::net::AddrParseError) -> Self {
        Self::HostParseError(e)
    }
}

impl From<hyper::Error> for FaucetError {
    fn from(e: hyper::Error) -> Self {
        Self::Hyper(e)
    }
}

impl std::fmt::Display for FaucetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::PoolBuild(e) => write!(f, "Pool build error: {}", e),
            Self::PoolTimeout(e) => write!(f, "Pool timeout error: {:?}", e),
            Self::PoolPostCreateHook => write!(f, "Pool post create hook error"),
            Self::PoolClosed => write!(f, "Pool closed error"),
            Self::PoolNoRuntimeSpecified => write!(f, "Pool no runtime specified error"),
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Unknown(e) => write!(f, "Unknown error: {}", e),
            Self::HostParseError(e) => write!(f, "Error parsing host address: {}", e),
            Self::Hyper(e) => write!(f, "Hyper error: {}", e),
            Self::Http(e) => write!(f, "Http error: {}", e),
            Self::InvalidHeaderValues(e) => write!(f, "Invalid header values: {}", e),
            Self::MissingArgument(s) => write!(f, "Missing argument: {}", s),
            Self::DuplicateRoute(route) => write!(f, "Route '{route}' is duplicated"),
            Self::AttackAttempt => write!(f, "Attack attempt detected"),
            Self::ConnectionClosed => write!(f, "Connection closed"),
            Self::ProtocolViolation(e) => write!(f, "Protocol violation: {e}"),
            Self::Utf8Coding => write!(f, "Utf8 Coding error"),
            Self::BufferCapacity(cap_err) => write!(f, "Buffer Capacity: {cap_err}"),
            Self::WSWriteBufferFull(buf) => write!(f, "Web Socket Write buffer full, {buf}"),
            Self::PostgreSQL(value) => write!(f, "PostgreSQL error: {value}"),
            Self::BadRequest(r) => match r {
                BadRequestReason::UnsupportedUrlScheme => {
                    write!(f, "UnsupportedUrlScheme use ws:// os wss://")
                }
                BadRequestReason::NoHostName => write!(f, "No Host Name"),
                BadRequestReason::MissingHeader(header) => {
                    write!(f, "Missing header: {}", header)
                }
                BadRequestReason::InvalidHeader(header) => {
                    write!(f, "Invalid header: {}", header)
                }
                BadRequestReason::NoPathOrQuery => write!(f, "No path and/or query"),
            },
            Self::NoSocketsAvailable => write!(f, "No sockets available"),
        }
    }
}

impl std::fmt::Debug for FaucetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::error::Error for FaucetError {}

impl FaucetError {
    pub fn no_sec_web_socket_key() -> Self {
        Self::BadRequest(BadRequestReason::MissingHeader("Sec-WebSocket-Key"))
    }
    pub fn unknown(s: impl ToString) -> Self {
        Self::Unknown(s.to_string())
    }
}

impl From<FaucetError> for hyper::Response<ExclusiveBody> {
    fn from(val: FaucetError) -> Self {
        let mut resp = hyper::Response::new(ExclusiveBody::plain_text(val.to_string()));
        *resp.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faucet_error() {
        let err = FaucetError::unknown("test");
        assert_eq!(err.to_string(), "Unknown error: test");
    }

    #[test]
    fn test_faucet_error_debug() {
        let err = FaucetError::unknown("test");
        assert_eq!(format!("{:?}", err), r#"Unknown error: test"#);
    }

    #[test]
    fn test_faucet_error_from_hyper_error() {
        let err = hyper::Request::builder()
            .uri("INVALID URI")
            .body(())
            .unwrap_err();

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_io_error() {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "test");

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_pool_error() {
        let err = deadpool::managed::PoolError::Backend(FaucetError::unknown("test"));

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_pool_build_error() {
        let err = deadpool::managed::BuildError::NoRuntimeSpecified;

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_pool_timeout_error() {
        let err = deadpool::managed::PoolError::<FaucetError>::Timeout(
            deadpool::managed::TimeoutType::Create,
        );

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_pool_closed_error() {
        let err = deadpool::managed::PoolError::<FaucetError>::Closed;

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_pool_post_create_hook_error() {
        let err = deadpool::managed::PoolError::<FaucetError>::PostCreateHook(
            deadpool::managed::HookError::message("test"),
        );

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_pool_no_runtime_specified_error() {
        let err = deadpool::managed::PoolError::<FaucetError>::NoRuntimeSpecified;

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_hyper_invalid_header_value_error() {
        let err = hyper::header::HeaderValue::from_bytes([0x00].as_ref()).unwrap_err();

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_from_addr_parse_error() {
        let err = "INVALID".parse::<std::net::SocketAddr>().unwrap_err();

        let _err: FaucetError = From::from(err);
    }

    #[test]
    fn test_faucet_error_displat_missing_header() {
        let _err = FaucetError::BadRequest(BadRequestReason::MissingHeader("test"));
    }

    #[test]
    fn test_faucet_error_displat_invalid_header() {
        let _err = FaucetError::BadRequest(BadRequestReason::InvalidHeader("test"));
    }

    #[test]
    fn test_from_fauct_error_to_hyper_response() {
        let err = FaucetError::unknown("test");
        let resp: hyper::Response<ExclusiveBody> = err.into();
        assert_eq!(resp.status(), hyper::StatusCode::INTERNAL_SERVER_ERROR);
    }
}
