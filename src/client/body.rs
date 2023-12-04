use std::pin::Pin;

use super::pool::HttpConnection;
use crate::error::FaucetError;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::{Body, Bytes, SizeHint};

pub struct ExclusiveBody {
    inner: Pin<Box<dyn Body<Data = Bytes, Error = FaucetError> + Send + 'static>>,
    _connection: Option<HttpConnection>,
}

impl ExclusiveBody {
    pub fn new(
        body: impl Body<Data = Bytes, Error = FaucetError> + Send + Sync + 'static,
        connection: Option<HttpConnection>,
    ) -> Self {
        Self {
            inner: Box::pin(body),
            _connection: connection,
        }
    }
    pub fn empty() -> Self {
        Self::new(Empty::new().map_err(Into::into), None)
    }
    pub fn plain_text(text: impl Into<String>) -> Self {
        Self::new(Full::from(text.into()).map_err(Into::into), None)
    }
}

impl Body for ExclusiveBody {
    type Data = Bytes;
    type Error = FaucetError;
    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        self.inner.as_mut().poll_frame(cx)
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}
