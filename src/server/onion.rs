pub trait Service<Request> {
    type Response;
    type Error;
    async fn call(&self, req: Request) -> Result<Self::Response, Self::Error>;
}

pub trait Layer<S> {
    type Service;
    fn layer(&self, inner: S) -> Self::Service;
}

pub struct ServiceBuilder<S> {
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
