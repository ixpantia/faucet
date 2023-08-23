use crate::map_mutex::{Dispatcher, LockGuard};
use crate::prelude::convert_request;
use anyhow::Result;
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
    fn get_socket_addrs(&self) -> Result<Vec<SocketAddr>> {
        let sockets = self.uri
            .authority()
            .ok_or_else(|| anyhow::anyhow!("missing authority"))?
            .as_str()
            .to_socket_addrs()?
            .collect();
        Ok(sockets)
    }

    /// Convert a socket address to a URL.
    fn socket_to_uri(&self, socket: SocketAddr) -> Result<Uri> {
        let mut uri = self.uri.clone().into_parts();
        uri.authority = Some(socket.to_string().try_into()?);
        Ok(Uri::from_parts(uri)?)
    }

    /// Acquire a lock on a socket address.
    async fn acquire(&self, sockets: &[SocketAddr]) -> LockGuard<SocketAddr> {
        // Cycle through the socket addresses until we acquire a lock.
        let mut addrs = sockets.into_iter().cycle();
        loop {
            let addr = addrs.next().unwrap();
            if let Some(lock) = self.dispatcher.try_acquire(*addr).await {
                return lock;
            }
        }
    }

    async fn forward_req(&self, lock: &LockGuard<SocketAddr>, req: Request<Body>) -> Result<Response<Body>> {
        // Get the socket address of the pod from the lock.
        let socket = lock.key();
        // Convert the socket address to a URL.
        let pod_url = self.socket_to_uri(socket)?;
        // Convert the request to a request to the pod.
        let pod_req = convert_request(&pod_url, req)?;
        // Send the request to the pod and wait for the response.
        let pod_res = self.client.request(pod_req).await?;
        Ok(pod_res)
    }

    /// Send a request to a worker.
    pub async fn send(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        // Get the socket addresses of the pods backing the service.
        let sockets = self.get_socket_addrs()?;
        // Acquire a lock on any available pod.
        let lock = self.acquire(&sockets).await;
        let pod_res = match self.forward_req(&lock, req).await {
            Ok(res) => res,
            Err(err) => {
                // If the request failed, release the lock on the pod.
                lock.release().await;
                // Return the error to the client.
                return Err(err);
            }
        };
        // Release the lock on the pod.
        lock.release().await;
        // Return the response to the client.
        Ok(pod_res)
    }
}
