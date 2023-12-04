use faucet_server::cli::Args;
use faucet_server::error::FaucetResult;
use faucet_server::server::FaucetServer;

#[tokio::main]
pub async fn main() -> FaucetResult<()> {
    env_logger::init_from_env(env_logger::Env::new().filter_or("FAUCET_LOG", "info"));
    let cli_args = Args::parse();

    log::info!(target: "faucet", "Starting Faucet!");

    FaucetServer::new()
        .strategy(cli_args.strategy())
        .workers(cli_args.workers())
        .server_type(cli_args.server_type())
        .extractor(cli_args.ip_extractor())
        .bind(cli_args.host().parse()?)
        .workdir(cli_args.dir())
        .run()
        .await?;
    Ok(())
}
