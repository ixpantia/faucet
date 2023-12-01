pub type FaucetResult<T> = std::result::Result<T, FucetError>;

pub enum FucetError {
    Io(std::io::Error),
    Unknown(String),
    HostParseError(std::net::AddrParseError),
}

impl From<std::io::Error> for FucetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<std::net::AddrParseError> for FucetError {
    fn from(e: std::net::AddrParseError) -> Self {
        Self::HostParseError(e)
    }
}

impl std::fmt::Display for FucetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Unknown(e) => write!(f, "Unknown error: {}", e),
            Self::HostParseError(e) => write!(f, "Error parsing host address: {}", e),
        }
    }
}

impl std::fmt::Debug for FucetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {:?}", e),
            Self::Unknown(e) => write!(f, "Unknown error: {:?}", e),
            Self::HostParseError(e) => write!(f, "Error parsing host address: {:?}", e),
        }
    }
}

impl std::error::Error for FucetError {}
