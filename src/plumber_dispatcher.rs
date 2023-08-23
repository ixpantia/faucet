use crate::k8::K8PlumberDispatcher;
use crate::OnPremPlumberDispatcher;
use hyper:: { Body, Request, Response };

pub enum PlumberDispatcher {
    OnPrem(OnPremPlumberDispatcher),
    K8(K8PlumberDispatcher),
}

impl From<OnPremPlumberDispatcher> for PlumberDispatcher {
    fn from(val: OnPremPlumberDispatcher) -> Self {
        PlumberDispatcher::OnPrem(val)
    }
}

impl From<K8PlumberDispatcher> for PlumberDispatcher {
    fn from(val: K8PlumberDispatcher) -> Self {
        PlumberDispatcher::K8(val)
    }
}

impl PlumberDispatcher {
    pub async fn send(
        &self,
        req: Request<Body>,
    ) -> Response<Body> {
        let res = match self {
            PlumberDispatcher::OnPrem(dispatcher) => dispatcher.send(req).await,
            PlumberDispatcher::K8(dispatcher) => dispatcher.send(req).await,
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
