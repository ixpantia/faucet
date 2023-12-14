use crate::{client::ExclusiveBody, error::FaucetResult};
use async_trait::async_trait;
use hyper::{body::Incoming, Request, Response};

#[derive(Clone)]
struct DoNothingService<S>
where
    S: Service + Send + Sync,
{
    inner: S,
}

#[async_trait]
impl<S: Service + Send + Sync> Service for DoNothingService<S> {
    async fn call(&self, req: Request<Incoming>) -> FaucetResult<Response<ExclusiveBody>> {
        self.inner.call(req).await
    }
}

#[derive(Clone)]
struct DoNothingLayer {}

impl<S: Service + Send + Sync> Layer<S> for DoNothingLayer {
    type Service = DoNothingService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        DoNothingService { inner }
    }
}

#[async_trait]
pub(crate) trait Service {
    async fn call(&self, req: Request<Incoming>) -> FaucetResult<Response<ExclusiveBody>>;
}

pub(crate) trait Layer<S> {
    type Service: Service;
    fn layer(&self, inner: S) -> Self::Service;
}

pub(crate) struct ServiceBuilder<S> {
    service: S,
}

impl<S> ServiceBuilder<S> {
    pub fn new(service: S) -> Self {
        ServiceBuilder { service }
    }
    pub fn layer<L>(self, layer: L) -> ServiceBuilder<L::Service>
    where
        L: Layer<S>,
    {
        ServiceBuilder::new(layer.layer(self.service))
    }
    pub fn build(self) -> S {
        self.service
    }
}
