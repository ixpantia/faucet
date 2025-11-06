use std::convert::Infallible;

use crate::client::ExclusiveBody;

pub enum BadRequestReason {
    MissingHeader(&'static str),
    InvalidHeader(&'static str),
    MissingQueryParam(&'static str),
    InvalidQueryParam(&'static str),
    NoPathOrQuery,
    NoHostName,
    UnsupportedUrlScheme,
}

pub type FaucetResult<T> = std::result::Result<T, FaucetError>;

use thiserror::Error;

impl std::fmt::Display for BadRequestReason {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BadRequestReason::MissingQueryParam(param) => {
                write!(f, "Missing query parameter: {param}")
            }
            BadRequestReason::InvalidQueryParam(param) => {
                write!(f, "Invalid query parameter: {param}")
            }
            BadRequestReason::UnsupportedUrlScheme => {
                write!(f, "UnsupportedUrlScheme use ws:// or wss://")
            }
            BadRequestReason::NoHostName => write!(f, "No Host Name"),
            BadRequestReason::MissingHeader(header) => write!(f, "Missing header: {header}"),
            BadRequestReason::InvalidHeader(header) => write!(f, "Invalid header: {header}"),
            BadRequestReason::NoPathOrQuery => write!(f, "No path and/or query"),
        }
    }
}

#[derive(Error)]
pub enum FaucetError {
    #[error("Pool build error: {0}")]
    PoolBuild(#[from] deadpool::managed::BuildError),
    #[error("Pool timeout error: {0:?}")]
    PoolTimeout(deadpool::managed::TimeoutType),
    #[error("Pool post create hook error")]
    PoolPostCreateHook,
    #[error("Pool closed error")]
    PoolClosed,
    #[error("Pool no runtime specified error")]
    PoolNoRuntimeSpecified,
    #[error("No sockets available")]
    NoSocketsAvailable,
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unknown error: {0}")]
    Unknown(String),
    #[error("Error parsing host address: {0}")]
    HostParseError(#[from] std::net::AddrParseError),
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("{0}")]
    BadRequest(BadRequestReason),
    #[error("Invalid header values: {0}")]
    InvalidHeaderValues(#[from] hyper::header::InvalidHeaderValue),
    #[error("Http error: {0}")]
    Http(#[from] hyper::http::Error),
    #[error("Missing argument: {0}")]
    MissingArgument(&'static str),
    #[error("Route '{0}' is duplicated")]
    DuplicateRoute(String),
    #[error("Utf8 Coding error: {0}")]
    Utf8Coding(String),
    #[error("Buffer Capacity: {0}")]
    BufferCapacity(tokio_tungstenite::tungstenite::error::CapacityError),
    #[error("Protocol violation: {0}")]
    ProtocolViolation(tokio_tungstenite::tungstenite::error::ProtocolError),
    #[error("Web Socket Write buffer full, {0}")]
    WSWriteBufferFull(tokio_tungstenite::tungstenite::Message),
    #[error("PostgreSQL error: {0}")]
    PostgreSQL(#[from] tokio_postgres::Error),
    #[error("WebSocket Connection in use")]
    WebSocketConnectionInUse,
    #[error(
        "WebSocket Connection purged. The client is trying to access a Shiny connection that does not exist."
    )]
    WebSocketConnectionPurged,
    #[error("Attack attempt detected")]
    AttackAttempt,
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
            Error::Utf8(err) => FaucetError::Utf8Coding(err),
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

impl std::fmt::Debug for FaucetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

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
        assert_eq!(format!("{err:?}"), r#"Unknown error: test"#);
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
        let err = std::io::Error::other("test");

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
