// Modules
mod cli;
mod k8s;
mod map_mutex;
mod onprem;
mod plumber_dispatcher;
mod prelude;

// Imports
use clap::Parser;
use cli::Backend;
use k8s::K8sPlumberDispatcher;
use onprem::OnPremPlumberDispatcher;
use plumber_dispatcher::PlumberDispatcher;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Semaphore;

async fn hyper_redirect(
    req: Request<Body>,
    dispatcher: Arc<PlumberDispatcher>,
    semaphore: Arc<Semaphore>,
) -> Result<Response<Body>, Infallible> {
    let _permit = semaphore.acquire().await.expect("Semaphore error");
    Ok(dispatcher.send(req).await)
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = cli::Args::parse();

    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Initialize dispatcher
    let dispatcher: PlumberDispatcher = match args.backend {
        Backend::Local(args) => {
            OnPremPlumberDispatcher::new(args.dir, args.child_port, args.workers)
                .await
                .into()
        }
        Backend::K8s(args) => K8sPlumberDispatcher::new(args.service_url).into(),
    };

    // Create a semaphore to limit the number of concurrent requests trying to
    // access the dispatcher
    let semaphore = Arc::new(Semaphore::new(args.threads));

    // Wrap dispatcher in web::Data to allow it to be shared between threads
    let dispatcher = Arc::new(dispatcher);

    let addr = args.host.parse::<SocketAddr>().expect("Invalid host");

    let make_svc = make_service_fn(move |_conn| {
        let dispatcher = dispatcher.clone();
        let semaphore = semaphore.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let dispatcher = dispatcher.clone();
                let semaphore = semaphore.clone();
                hyper_redirect(req, dispatcher, semaphore)
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
