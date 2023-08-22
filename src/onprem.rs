use crate::map_mutex::{Dispatcher, LockGuard};
use crate::prelude::{convert_request};

use hyper::{Body, Request, Response, Uri, client::HttpConnector};
use tokio::process::Child;

const PORT_ENV_VAR: &str = "FAUCET_PORT";

struct PlumberWorker {
    /// The child process running the plumber worker.
    child: Child,
    /// The url of the worker.
    uri: Uri,
    /// The port the worker is listening on.
    port: u16,
}

/// Warns the user that the worker is being killed.
impl Drop for PlumberWorker {
    fn drop(&mut self) {
        let url = &self.uri;
        let id = match self.child.id() {
            Some(id) => id,
            None => return,
        };
        log::warn!(target: "PlumberWorker", "Killing worker with PID {id} listening on {url}");
    }
}

impl PlumberWorker {
    /// Spawns a child process with the given directory and port.
    fn spawn_child_process(dir: &std::path::PathBuf, port: u16) -> Result<Child, std::io::Error> {
        let child = tokio::process::Command::new("Rscript")
            // Set the current directory to the directory containing the entrypoint
            .current_dir(dir)
            .arg("entrypoint.R")
            // Set the port environment variable `PORT` to the port we want to use
            .env(PORT_ENV_VAR, port.to_string())
            // This is needed to make sure the child process is killed when the parent is dropped
            .kill_on_drop(true)
            .spawn();
        // If the child process was spawned successfully, log the port and pid
        // This is useful for debugging.
        if let Ok(child) = &child {
            if let Some(id) = child.id() {
                log::info!(target: "PlumberWorker", "Started worker with PID {id} listening on port {port}");
            }
        }
        child
    }

    /// Returns the url for the worker with the given id.
    fn build_uri(port: u16) -> Uri {
        Uri::try_from(&format!("http://127.0.0.1:{}", port)).expect("failed to parse url")
    }

    async fn new(dir: &std::path::PathBuf, id: usize, base_port: u16) -> Self {
        let port = base_port + id as u16;
        let child = Self::spawn_child_process(dir, port).expect("Failed to spawn child process");
        let uri = Self::build_uri(port);
        Self { child, port, uri }
    }

    /// Get the id of the worker.
    fn get_id(&self) -> PlumberWorkerId {
        PlumberWorkerId { port: self.port }
    }

    /// Check if the worker has the given id.
    fn matches(&self, id: PlumberWorkerId) -> bool {
        self.get_id() == id
    }

    /// Get the url of the worker.
    fn get_url(&self) -> &Uri {
        &self.uri
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Used to identify a worker and acquire a lock on it.
struct PlumberWorkerId {
    port: u16,
}

/// A dispatcher for plumber workers.
pub struct OnPremPlumberDispatcher {
    workers: Vec<PlumberWorker>,
    client: hyper::Client<HttpConnector>,
    dispatcher: Dispatcher<PlumberWorkerId>,
}

impl OnPremPlumberDispatcher {
    pub async fn new(dir: std::path::PathBuf, base_port: u16, n_workers: usize) -> Self {
        // Create a new client that will recycle TCP connections.
        let client = hyper::Client::new();
        // Create a vector of workers and initialize them.
        let mut workers = Vec::with_capacity(n_workers);
        for i in 0..n_workers {
            workers.push(PlumberWorker::new(&dir, i, base_port).await)
        }
        Self {
            client,
            workers,
            dispatcher: Dispatcher::new(),
        }
    }

    /// Find the worker with the given id.
    fn find_worker(&self, id: PlumberWorkerId) -> Option<&PlumberWorker> {
        self.workers.iter().find(|w| w.matches(id))
    }

    /// Get the ids of the workers.
    fn get_worker_ids(&self) -> Vec<PlumberWorkerId> {
        self.workers.iter().map(|w| w.get_id()).collect()
    }

    async fn acquire(&self) -> LockGuard<PlumberWorkerId> {
        // Cycle through the workers until we find one that is available.
        let mut workers = self.get_worker_ids().into_iter().cycle();
        loop {
            let worker = workers.next().unwrap();
            if let Some(lock) = self.dispatcher.try_acquire(worker).await {
                return lock;
            }
        }
    }

    /// Send a request to a worker.
    pub async fn send(
        &self,
        req: Request<Body>,
    ) -> Response<Body> {
        // Acquire a lock on a worker.
        let lock = self.acquire().await;
        // Get the id of the worker.
        let worker_id = lock.key();
        // Find the worker with the given id.
        let worker = self.find_worker(worker_id).expect("failed to find worker");
        // Get the url of the worker.
        let pod_url = worker.get_url();
        // Convert the request to a worker request.
        let pod_req = convert_request(pod_url, req);
        // Send the request to the worker.
        let pod_res = self.client.request(pod_req).await.expect("failed to send request");
        // Release the lock on the worker.
        lock.release().await;
        // Return the response.
        pod_res
    }
}
