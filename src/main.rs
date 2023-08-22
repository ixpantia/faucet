// Modules
mod cli;
mod k8;
mod map_mutex;
mod onprem;
mod plumber_dispatcher;
mod prelude;

// Imports
use clap::Parser;
use cli::Backend;
use k8::K8PlumberDispatcher;
use onprem::OnPremPlumberDispatcher;
use plumber_dispatcher::PlumberDispatcher;

use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server};
use std::sync::Arc;
use hyper::service::{make_service_fn, service_fn};

async fn hyper_redirect(
    req: Request<Body>,
    dispatcher: Arc<PlumberDispatcher>,
) -> Result<Response<Body>, Infallible> {
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
        Backend::K8s(args) => K8PlumberDispatcher::new(args.service_url).into(),
    };

    // Wrap dispatcher in web::Data to allow it to be shared between threads
    let dispatcher = Arc::new(dispatcher);

    let addr = args.host.parse::<SocketAddr>().expect("Invalid host");

    let make_svc = make_service_fn(move |_conn| {
        let dispatcher = dispatcher.clone();
        async move{
            Ok::<_, Infallible>(service_fn(move |req| {
                let dispatcher = dispatcher.clone();
                hyper_redirect(req, dispatcher)
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
