pub mod cli;
pub mod client;
pub mod error;
pub mod http;
pub(crate) mod networking;
pub mod server;

#[macro_use]
macro_rules! leak {
    ($val:expr, $ty:ty) => {
        std::boxed::Box::leak(std::boxed::Box::from($val)) as &'static $ty
    };
    ($val:expr) => {
        std::boxed::Box::leak(std::boxed::Box::from($val))
    };
}

pub(crate) use leak;
