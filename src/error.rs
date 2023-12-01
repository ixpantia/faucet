use std::convert::Infallible;

pub type FaucetResult<T> = std::result::Result<T, FaucetError>;

pub enum FaucetError {
    PoolBuild(deadpool::managed::BuildError),
    PoolTimeout(deadpool::managed::TimeoutType),
    PoolPostCreateHook,
    PoolClosed,
    PoolNoRuntimeSpecified,
    Io(std::io::Error),
    Unknown(String),
    HostParseError(std::net::AddrParseError),
    Hyper(hyper::Error),
    Infallible(Infallible),
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
        }
    }
}

impl std::fmt::Debug for FaucetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
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
        }
    }
}

impl std::error::Error for FaucetError {}
