use std::net::IpAddr;

pub trait Service<Request>: Send + Sync {
    type Response;
    type Error;
    fn call(
        &self,
        req: Request,
        ip_addr: Option<IpAddr>,
    ) -> impl std::future::Future<Output = Result<Self::Response, Self::Error>>;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn basic_service_response() {
        struct Svc;

        impl Service<()> for Svc {
            type Response = String;
            type Error = ();
            async fn call(&self, _: (), _: Option<IpAddr>) -> Result<Self::Response, Self::Error> {
                Ok("Hello, world!".to_string())
            }
        }

        let svc = ServiceBuilder::new(Svc).build();

        assert_eq!(svc.call((), None).await.unwrap(), "Hello, world!");
    }

    #[tokio::test]
    async fn basic_service_middleware() {
        struct Svc;

        impl Service<&'static str> for Svc {
            type Response = String;
            type Error = ();
            async fn call(
                &self,
                _: &'static str,
                _: Option<IpAddr>,
            ) -> Result<Self::Response, Self::Error> {
                Ok("Hello, world!".to_string())
            }
        }

        struct GoodByeService<S> {
            inner: S,
        }

        impl<S> Service<&'static str> for GoodByeService<S>
        where
            S: Service<&'static str, Response = String, Error = ()>,
        {
            type Response = String;
            type Error = ();
            async fn call(
                &self,
                req: &'static str,
                _: Option<IpAddr>,
            ) -> Result<Self::Response, Self::Error> {
                if req == "Goodbye" {
                    Ok("Goodbye, world!".to_string())
                } else {
                    self.inner.call(req, None).await
                }
            }
        }

        struct GoodByeLayer;

        impl<S> Layer<S> for GoodByeLayer {
            type Service = GoodByeService<S>;
            fn layer(&self, inner: S) -> Self::Service {
                GoodByeService { inner }
            }
        }

        let svc = ServiceBuilder::new(Svc).layer(GoodByeLayer).build();

        assert_eq!(svc.call("Goodbye", None).await.unwrap(), "Goodbye, world!");
        assert_eq!(svc.call("Hello", None).await.unwrap(), "Hello, world!");
    }

    #[tokio::test]
    async fn multiple_layer_middleware() {
        struct Svc;

        impl Service<&'static str> for Svc {
            type Response = String;
            type Error = ();
            async fn call(
                &self,
                _: &'static str,
                _: Option<IpAddr>,
            ) -> Result<Self::Response, Self::Error> {
                Ok("Hello, world!".to_string())
            }
        }

        struct GoodByeService<S> {
            inner: S,
        }

        impl<S> Service<&'static str> for GoodByeService<S>
        where
            S: Service<&'static str, Response = String, Error = ()>,
        {
            type Response = String;
            type Error = ();
            async fn call(
                &self,
                req: &'static str,
                _: Option<IpAddr>,
            ) -> Result<Self::Response, Self::Error> {
                if req == "Goodbye" {
                    Ok("Goodbye, world!".to_string())
                } else {
                    self.inner.call(req, None).await
                }
            }
        }

        struct GoodByeLayer;

        impl<S> Layer<S> for GoodByeLayer {
            type Service = GoodByeService<S>;
            fn layer(&self, inner: S) -> Self::Service {
                GoodByeService { inner }
            }
        }

        struct HowAreYouService<S> {
            inner: S,
        }

        impl<S> Service<&'static str> for HowAreYouService<S>
        where
            S: Service<&'static str, Response = String, Error = ()>,
        {
            type Response = String;
            type Error = ();
            async fn call(
                &self,
                req: &'static str,
                _: Option<IpAddr>,
            ) -> Result<Self::Response, Self::Error> {
                if req == "How are you?" {
                    Ok("I'm fine, thank you!".to_string())
                } else {
                    self.inner.call(req, None).await
                }
            }
        }

        struct HowAreYouLayer;

        impl<S> Layer<S> for HowAreYouLayer {
            type Service = HowAreYouService<S>;
            fn layer(&self, inner: S) -> Self::Service {
                HowAreYouService { inner }
            }
        }

        let svc = ServiceBuilder::new(Svc)
            .layer(GoodByeLayer)
            .layer(HowAreYouLayer)
            .build();

        assert_eq!(svc.call("Goodbye", None).await.unwrap(), "Goodbye, world!");
        assert_eq!(svc.call("Hello", None).await.unwrap(), "Hello, world!");
        assert_eq!(
            svc.call("How are you?", None).await.unwrap(),
            "I'm fine, thank you!"
        );
    }
}
