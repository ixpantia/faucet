use clap::Parser;
use faucet_server::cli::{Args, Commands};
use faucet_server::error::FaucetResult;
use faucet_server::server::{FaucetServerBuilder, RouterConfig};

#[tokio::main]
pub async fn main() -> FaucetResult<()> {
    ctrlc::set_handler(|| {
        log::info!(target: "faucet", "Ctrl-C received, shutting down...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    env_logger::init_from_env(env_logger::Env::new().filter_or("FAUCET_LOG", "info"));
    let cli_args = Args::parse();
    match cli_args.command {
        Commands::Start(start_args) => {
            log::info!(target: "faucet", "Building the faucet server...");

            FaucetServerBuilder::new()
                .strategy(start_args.strategy())
                .workers(start_args.workers())
                .server_type(start_args.server_type())
                .extractor(start_args.ip_extractor())
                .bind(start_args.host().parse()?)
                .workdir(start_args.dir())
                .rscript(start_args.rscript())
                .app_dir(start_args.app_dir())
                .build()?
                .run()
                .await?;
        }
        Commands::Router => {
            let config: RouterConfig =
                toml::from_str(&std::fs::read_to_string("router.toml").unwrap()).unwrap();

            config.run().await?;
        }
    }

    Ok(())
}
