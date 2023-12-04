use std::convert::Infallible;

pub enum BadRequestReason {
    MissingHeader(&'static str),
    InvalidHeader(&'static str),
}

pub type FaucetResult<T> = std::result::Result<T, FaucetError>;

pub enum FaucetError {
    PoolBuild(deadpool::managed::BuildError),
    PoolTimeout(deadpool::managed::TimeoutType),
    PoolPostCreateHook,
    PoolClosed,
    PoolNoRuntimeSpecified,
    RecvError(tokio::sync::watch::error::RecvError),
    Io(std::io::Error),
    Unknown(String),
    HostParseError(std::net::AddrParseError),
    Hyper(hyper::Error),
    Infallible(Infallible),
    BadRequest(BadRequestReason),
    InvalidHeaderValues(hyper::header::InvalidHeaderValue),
    Http(hyper::http::Error),
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

impl From<tokio::sync::watch::error::RecvError> for FaucetError {
    fn from(e: tokio::sync::watch::error::RecvError) -> Self {
        Self::RecvError(e)
    }
}

impl From<Infallible> for FaucetError {
    fn from(e: Infallible) -> Self {
        Self::Infallible(e)
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
            Self::RecvError(e) => write!(f, "Recv error: {}", e),
            Self::PoolBuild(e) => write!(f, "Pool build error: {}", e),
            Self::PoolTimeout(e) => write!(f, "Pool timeout error: {:?}", e),
            Self::PoolPostCreateHook => write!(f, "Pool post create hook error"),
            Self::PoolClosed => write!(f, "Pool closed error"),
            Self::PoolNoRuntimeSpecified => write!(f, "Pool no runtime specified error"),
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Unknown(e) => write!(f, "Unknown error: {}", e),
            Self::HostParseError(e) => write!(f, "Error parsing host address: {}", e),
            Self::Hyper(e) => write!(f, "Hyper error: {}", e),
            Self::Infallible(e) => write!(f, "Infallible error: {}", e),
            Self::Http(e) => write!(f, "Http error: {}", e),
            Self::InvalidHeaderValues(e) => write!(f, "Invalid header values: {}", e),
            Self::BadRequest(r) => match r {
                BadRequestReason::MissingHeader(header) => {
                    write!(f, "Missing header: {}", header)
                }
                BadRequestReason::InvalidHeader(header) => {
                    write!(f, "Invalid header: {}", header)
                }
            },
        }
    }
}

impl std::fmt::Debug for FaucetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::RecvError(e) => write!(f, "Recv error: {:?}", e),
            Self::PoolTimeout(e) => write!(f, "Pool timeout error: {:?}", e),
            Self::PoolPostCreateHook => write!(f, "Pool post create hook error"),
            Self::PoolClosed => write!(f, "Pool closed error"),
            Self::PoolNoRuntimeSpecified => write!(f, "Pool no runtime specified error"),
            Self::PoolBuild(e) => write!(f, "Pool build error: {:?}", e),
            Self::Io(e) => write!(f, "IO error: {:?}", e),
            Self::Unknown(e) => write!(f, "Unknown error: {:?}", e),
            Self::HostParseError(e) => write!(f, "Error parsing host address: {:?}", e),
            Self::Hyper(e) => write!(f, "Hyper error: {:?}", e),
            Self::Infallible(e) => write!(f, "Infallible error: {:?}", e),
            Self::Http(e) => write!(f, "Http error: {:?}", e),
            Self::InvalidHeaderValues(e) => write!(f, "Invalid header values: {:?}", e),
            Self::BadRequest(r) => match r {
                BadRequestReason::MissingHeader(header) => {
                    write!(f, "Missing header: {}", header)
                }
                BadRequestReason::InvalidHeader(header) => {
                    write!(f, "Invalid header: {}", header)
                }
            },
        }
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
