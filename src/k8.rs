use crate::map_mutex::{Dispatcher, LockGuard};
use crate::prelude::{convert_request};

use std::net::{ SocketAddr, ToSocketAddrs };
use hyper::{Body, Request, Response, Uri, client::HttpConnector};

pub struct K8PlumberDispatcher {
    /// The URL of the service to be load balanced.
    uri: Uri,
    /// The HTTP client used to send requests to the service.
    client: hyper::Client<HttpConnector>,
    /// The dispatcher used to load balance requests.
    dispatcher: Dispatcher<SocketAddr>,
}

impl K8PlumberDispatcher {
    pub fn new(uri: Uri) -> Self {
        let client = hyper::Client::new();
        Self {
            uri,
            client,
            dispatcher: Dispatcher::new(),
        }
    }

    /// Get the socket addresses of the pods backing the service
    /// from the URL.
    fn get_socket_addrs(&self) -> Vec<SocketAddr> {
        self.uri
            .authority()
            .expect("failed to get authority")
            .as_str()
            .to_socket_addrs()
            .expect("failed to get socket addresses")
            .collect()
    }

    /// Convert a socket address to a URL.
    fn socket_to_uri(&self, socket: SocketAddr) -> Uri {
        let mut uri = self.uri.clone().into_parts();
        uri.authority = Some(socket.to_string().try_into().expect("failed to convert socket to authority"));
        Uri::from_parts(uri).expect("failed to convert socket to url")
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

    /// Send a request to a worker.
    pub async fn send(
        &self,
        req: Request<Body>,
    ) -> Response<Body> {
        // Acquire a lock on any available pod.
        let lock = self.acquire().await;
        // Get the socket address of the pod from the lock.
        let socket = lock.key();
        // Convert the socket address to a URL.
        let pod_url = self.socket_to_uri(socket);
        // Convert the request to a request to the pod.
        let pod_req = convert_request(&pod_url, req);
        // Send the request to the pod and wait for the response.
        let pod_res = self.client.request(pod_req).await.expect("failed to send request");
        // Release the lock on the pod.
        lock.release().await;
        // Return the response to the client.
        pod_res
    }
}
