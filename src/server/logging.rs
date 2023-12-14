use super::State;
use crate::{
    client::ExclusiveBody,
    error::FaucetResult,
    middleware::{Layer, Service},
};
use async_trait::async_trait;
use hyper::{body::Incoming, Request, Response};
use std::time;

enum LogOption<T> {
    None,
    Some(T),
}

impl<T> From<Option<T>> for LogOption<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => LogOption::None,
            Some(v) => LogOption::Some(v),
        }
    }
}

impl<T> std::fmt::Display for LogOption<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogOption::None => write!(f, "-"),
            LogOption::Some(v) => write!(f, "{}", v),
        }
    }
}

impl<T> std::fmt::Debug for LogOption<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogOption::None => write!(f, r#""-""#),
            LogOption::Some(v) => write!(f, "{:?}", v),
        }
    }
}

pub(super) struct LogService<S> {
    inner: S,
}

#[async_trait]
impl<S: Service + Send + Sync> Service for LogService<S> {
    async fn call(&self, req: Request<Incoming>) -> FaucetResult<Response<ExclusiveBody>> {
        // start timer
        let start = time::Instant::now();

        // Extract request info for logging
        let state = req.extensions().get::<State>().expect("State not found");
        let ip = state.remote_addr;
        let target = state.client.target();
        let method = req.method().clone();
        let path = req.uri().clone();
        let version = req.version();
        let user_agent: LogOption<_> = req.headers().get(hyper::header::USER_AGENT).cloned().into();

        // Make the request
        let res = self.inner.call(req).await?;

        // Extract response info for logging
        let status = res.status().as_u16();
        let elapsed = start.elapsed().as_millis();

        // Log the request
        log::info!(
            target: target,
            r#"{ip} "{method} {path} {version:?}" {status} {user_agent:?} {elapsed}"#,
        );

        // Return the response
        Ok(res)
    }
}

pub(super) struct LogLayer;

impl<S: Service + Send + Sync> Layer<S> for LogLayer {
    type Service = LogService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        LogService { inner }
    }
}
