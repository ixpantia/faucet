use faucet_server::cli::Args;
use faucet_server::error::FaucetResult;
use faucet_server::server::FaucetServerBuilder;

#[tokio::main]
pub async fn main() -> FaucetResult<()> {
    ctrlc::set_handler(|| {
        log::info!(target: "faucet", "Ctrl-C received, shutting down...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    env_logger::init_from_env(env_logger::Env::new().filter_or("FAUCET_LOG", "info"));
    let cli_args = Args::parse();

    log::info!(target: "faucet", "Building the faucet server...");

    FaucetServerBuilder::new()
        .strategy(cli_args.strategy())
        .workers(cli_args.workers())
        .server_type(cli_args.server_type())
        .extractor(cli_args.ip_extractor())
        .bind(cli_args.host().parse()?)
        .workdir(cli_args.dir())
        .rscript(cli_args.rscript())
        .build()?
        .run()
        .await?;

    Ok(())
}
