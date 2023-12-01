use faucet::cli::Args;
use faucet::error::FaucetResult;
use faucet::server::FaucetServer;

#[tokio::main]
pub async fn main() -> FaucetResult<()> {
    env_logger::init_from_env(env_logger::Env::new().filter_or("FAUCET_LOG", "info"));
    let cli_args = Args::parse();

    log::info!(target: "faucet", "Starting Faucet!");

    FaucetServer::new()
        .strategy(cli_args.strategy())
        .workers(cli_args.workers())
        .server_type(cli_args.server_type())
        .bind(cli_args.host().parse()?)
        .workdir(cli_args.dir())
        .run()
        .await?;
    Ok(())
}
