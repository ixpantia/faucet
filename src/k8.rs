use crate::map_mutex::{Dispatcher, LockGuard};
use crate::prelude::{convert_request, convert_response};
use bytes::Bytes;
use std::net::SocketAddr;
use url::Url;

pub struct K8PlumberDispatcher {
    /// The URL of the service to be load balanced.
    url: Url,
    /// The HTTP client used to send requests to the service.
    client: reqwest::Client,
    /// The dispatcher used to load balance requests.
    dispatcher: Dispatcher<SocketAddr>,
}

impl K8PlumberDispatcher {
    pub fn new(url: Url) -> Self {
        let client = reqwest::Client::new();
        Self {
            url,
            client,
            dispatcher: Dispatcher::new(),
        }
    }

    /// Get the socket addresses of the pods backing the service
    /// from the URL.
    fn get_socket_addrs(&self) -> Vec<SocketAddr> {
        self.url
            .socket_addrs(|| None)
            .expect("Failed to resolve hostname")
    }

    /// Convert a socket address to a URL.
    fn socket_to_url(&self, socket: SocketAddr) -> Url {
        let mut url = self.url.clone();
        // Replace the host with the IP address of the pod.
        url.set_ip_host(socket.ip()).unwrap();
        url
    }

    /// Acquire a lock on a socket address.
    async fn acquire(&self) -> LockGuard<SocketAddr> {
        // Cycle through the socket addresses until we acquire a lock.
        let mut addrs = self.get_socket_addrs().into_iter().cycle();
        loop {
            let addr = addrs.next().unwrap();
            if let Some(lock) = self.dispatcher.try_acquire(addr).await {
                return lock;
            }
        }
    }

    /// Send a request to a pod.
    pub async fn send(
        &self,
        req: actix_web::HttpRequest,
        payload: Bytes,
    ) -> actix_web::HttpResponse {
        // Acquire a lock on any available pod.
        let lock = self.acquire().await;
        // Get the socket address of the pod from the lock.
        let socket = lock.key();
        // Convert the socket address to a URL.
        let pod_url = self.socket_to_url(socket);
        // Convert the request to a request to the pod.
        let pod_req = convert_request(&self.client, &pod_url, req, payload);
        // Send the request to the pod and wait for the response.
        let pod_res = self
            .client
            .execute(pod_req)
            .await
            .expect("failed to send request");
        // Convert the response from the pod to a response to the client.
        let res = convert_response(pod_res).await;
        // Release the lock on the pod.
        lock.release().await;
        // Return the response to the client.
        res
    }
}
