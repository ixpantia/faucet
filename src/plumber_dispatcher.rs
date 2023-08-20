use crate::k8::K8PlumberDispatcher;
use crate::OnPremPlumberDispatcher;

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
        req: actix_web::HttpRequest,
        payload: bytes::Bytes,
    ) -> actix_web::HttpResponse {
        match self {
            PlumberDispatcher::OnPrem(dispatcher) => dispatcher.send(req, payload).await,
            PlumberDispatcher::K8(dispatcher) => dispatcher.send(req, payload).await,
        }
    }
}
