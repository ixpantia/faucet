use tokio::process::Child;
use tokio::sync::{Mutex, MutexGuard};

const PORT_ENV_VAR: &str = "FAUCET_PORT";

pub struct PlumberDispatcherBuilder {
    dir: std::path::PathBuf,
    base_port: u16,
    n_workers: usize,
}

impl PlumberDispatcherBuilder {
    pub fn new(dir: std::path::PathBuf) -> Self {
        Self {
            dir,
            base_port: 8000,
            n_workers: 1,
        }
    }
    pub fn base_port(mut self, base_port: u16) -> Self {
        self.base_port = base_port;
        self
    }
    pub fn n_workers(mut self, n_workers: usize) -> Self {
        self.n_workers = n_workers;
        self
    }
    pub async fn build(self) -> PlumberDispatcher {
        PlumberDispatcher::new(self.dir, self.base_port, self.n_workers).await
    }
}

struct PlumberWorker {
    client: reqwest::Client,
    _child: Child,
    url: url::Url,
}

impl PlumberWorker {
    async fn new(
        dir: &std::path::PathBuf,
        id: usize,
        base_port: u16,
        client: reqwest::Client,
    ) -> Self {
        let port = Self::get_port(base_port, id);
        let child = Self::spawn_child_process(dir, port).expect("Failed to spawn child process");
        let url = Self::get_url(port);
        Self {
            client,
            _child: child,
            url,
        }
    }

    /// Takes a request and builds a new uri to send to the worker.
    /// The uri is built by taking the path and query string from the request
    /// and appending it to the worker's url.
    fn build_uri(&self, req: &actix_web::HttpRequest) -> url::Url {
        let mut url = self.url.clone();
        url.set_path(req.path());
        url.set_query(Some(req.query_string()));
        url
    }

    /// Converts a reqwest::Response into an actix_web::HttpResponse
    /// This is done by copying the status code and headers from the response
    /// and then reading the body into a byte array.
    async fn convert_response(res: reqwest::Response) -> actix_web::HttpResponse {
        let mut builder = actix_web::HttpResponseBuilder::new(res.status());
        // We copy every header from the response into the builder
        for (key, value) in res.headers() {
            builder.append_header((key, value));
        }
        // We read the body into a byte array and then set the body of the builder
        builder.body(res.bytes().await.expect("failed to read body into bytes"))
    }

    /// Converts an actix_web::HttpRequest into a reqwest::Request
    /// This is done by copying the method and uri from the request
    /// and then building a new reqwest::Request.
    async fn convert_request(&self, req: actix_web::HttpRequest) -> reqwest::Request {
        self.client
            .request(req.method().clone(), self.build_uri(&req))
            .build()
            .expect("failed to build request")
    }

    /// Returns the port for the worker with the given id.
    fn get_port(base: u16, id: usize) -> u16 {
        base + (id as u16)
    }

    /// Returns the url for the worker with the given id.
    fn get_url(port: u16) -> url::Url {
        url::Url::parse(&format!("http://127.0.0.1:{}", port)).expect("failed to parse url")
    }

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

    /// Sends a request to the worker and returns the response.
    async fn send(&self, req: actix_web::HttpRequest) -> actix_web::HttpResponse {
        // Convert the request into a reqwest::Request
        let request = self.convert_request(req).await;
        // Execute the request and convert the response into an actix_web::HttpResponse
        let res = self
            .client
            .execute(request)
            .await
            .expect("failed to execute request");
        Self::convert_response(res).await
    }
}

/// Warns the user that the worker is being killed.
impl Drop for PlumberWorker {
    fn drop(&mut self) {
        let url = &self.url;
        let id = match self._child.id() {
            Some(id) => id,
            None => return,
        };
        log::warn!(target: "PlumberWorker", "Killing worker with PID {id} listening on {url}");
    }
}

/// A dispatcher for plumber workers. This dispatcher will send requests to the workers
/// that do not have any requests currently being processed.
///
/// It utilizes a lock to ensure that only one worker is processing a request at a time.
pub struct PlumberDispatcher {
    workers: Vec<Mutex<PlumberWorker>>,
}

impl PlumberDispatcher {
    pub async fn new(dir: std::path::PathBuf, base_port: u16, n_workers: usize) -> Self {
        // Create a new client that will recycle TCP connections.
        let client = reqwest::Client::new();
        // Create a vector of workers and initialize them.
        let mut workers = Vec::with_capacity(n_workers as usize);
        for i in 0..n_workers {
            workers.push(Mutex::new(
                PlumberWorker::new(&dir, i, base_port, client.clone()).await,
            ))
        }
        Self { workers }
    }
    /// Creates a new PlumberDispatcherBuilder.
    pub fn builder(dir: std::path::PathBuf) -> PlumberDispatcherBuilder {
        PlumberDispatcherBuilder::new(dir)
    }
    async fn acquire_worker(&self) -> MutexGuard<'_, PlumberWorker> {
        // The `cycle` method creates an iterator that repeats the elements of the vector
        // therefore we never have to worry about running out of workers.
        let mut workers = self.workers.iter().cycle();
        loop {
            // We can panic here because we know that there is always a next worker.
            let worker = workers.next().expect("There is always a next worker");
            match worker.try_lock() {
                Ok(worker) => return worker,
                Err(_) => continue,
            };
        }
    }
    /// Acquires a worker and sends the request to it.
    pub async fn send(&self, req: actix_web::HttpRequest) -> actix_web::HttpResponse {
        let worker = self.acquire_worker().await;
        worker.send(req).await
    }
}
