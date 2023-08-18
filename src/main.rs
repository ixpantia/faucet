// Modules
mod cli;
mod workers;

// Imports
use actix_web::{middleware::Logger, web, App, HttpServer, Responder};
use clap::Parser;
use workers::PlumberDispatcher;

async fn redirect(
    req: actix_web::HttpRequest,
    dispatcher: web::Data<PlumberDispatcher>,
) -> impl Responder {
    dispatcher.send(req).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    // Parse command line arguments
    let args = cli::Args::parse();

    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Initialize dispatcher
    let dispatcher = PlumberDispatcher::builder(args.dir)
        .base_port(args.child_port)
        .n_workers(args.workers)
        .build()
        .await;

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
    .workers(args.workers)
    .run()
    .await
}
