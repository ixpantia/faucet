// Modules
mod cli;
mod k8;
mod map_mutex;
mod onprem;
mod plumber_dispatcher;
mod prelude;

// Imports
use actix_web::{middleware::Logger, web, App, HttpServer, Responder};
use clap::Parser;
use cli::Backend;
use k8::K8PlumberDispatcher;
use onprem::OnPremPlumberDispatcher;
use plumber_dispatcher::PlumberDispatcher;

async fn redirect(
    req: actix_web::HttpRequest,
    payload: web::Bytes,
    dispatcher: web::Data<PlumberDispatcher>,
) -> impl Responder {
    dispatcher.send(req, payload).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
        Backend::K8(args) => K8PlumberDispatcher::new(args.host).into(),
    };

    // Wrap dispatcher in web::Data to allow it to be shared between threads
    let dispatcher = web::Data::new(dispatcher);

    // Start server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default().log_target("Faucet"))
            .default_service(web::route().to(redirect))
            .app_data(dispatcher.clone())
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await
}
