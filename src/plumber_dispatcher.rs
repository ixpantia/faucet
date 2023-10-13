use crate::k8s::K8sPlumberDispatcher;
use crate::OnPremPlumberDispatcher;
use hyper::{Body, Request, Response};

pub enum PlumberDispatcher {
    OnPrem(OnPremPlumberDispatcher),
    K8s(K8sPlumberDispatcher),
}

impl From<OnPremPlumberDispatcher> for PlumberDispatcher {
    fn from(val: OnPremPlumberDispatcher) -> Self {
        PlumberDispatcher::OnPrem(val)
    }
}

impl From<K8sPlumberDispatcher> for PlumberDispatcher {
    fn from(val: K8sPlumberDispatcher) -> Self {
        PlumberDispatcher::K8s(val)
    }
}

impl PlumberDispatcher {
    pub async fn send(&self, req: Request<Body>) -> Response<Body> {
        let res = match self {
            PlumberDispatcher::OnPrem(dispatcher) => dispatcher.send(req).await,
            PlumberDispatcher::K8s(dispatcher) => dispatcher.send(req).await,
        };
        match res {
            Ok(res) => res,
            Err(err) => {
                log::error!("error: {}", err);
                Response::builder()
                    .status(500)
                    .body(Body::from(format!("error: {}", err)))
                    .unwrap()
            }
        }
    }
}
