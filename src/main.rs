use clap::Parser;
use faucet_server::cli::{Args, Commands};
use faucet_server::error::FaucetResult;
use faucet_server::server::logger::build_logger;
use faucet_server::server::{FaucetServerBuilder, RouterConfig};
use faucet_server::{cli::Shutdown, shutdown};

#[tokio::main]
pub async fn main() -> FaucetResult<()> {
    let cli_args = Args::parse();

    let signal = match cli_args.shutdown {
        Shutdown::Immediate => shutdown::immediate(),
        Shutdown::Graceful => shutdown::graceful(),
    };

    match cli_args.command {
        Commands::Start(start_args) => {
            build_logger(
                start_args
                    .log_file
                    .as_ref()
                    .map_or(faucet_server::server::logger::Target::Stderr, |file| {
                        faucet_server::server::logger::Target::File(file.to_path_buf())
                    }),
            );

            log::info!(target: "faucet", "Building the faucet server...");

            FaucetServerBuilder::new()
                .strategy(Some(start_args.strategy.into()))
                .workers(start_args.workers)
                .server_type(start_args.server_type())
                .extractor(start_args.ip_from.into())
                .bind(start_args.host.parse()?)
                .workdir(start_args.dir)
                .rscript(start_args.rscript)
                .app_dir(start_args.app_dir)
                .quarto(start_args.quarto)
                .qmd(start_args.qmd)
                .build()?
                .run(signal)
                .await?;
        }
        Commands::Router(router_args) => {
            build_logger(
                router_args
                    .log_file
                    .as_ref()
                    .map_or(faucet_server::server::logger::Target::Stderr, |file| {
                        faucet_server::server::logger::Target::File(file.to_path_buf())
                    }),
            );

            let config: RouterConfig =
                toml::from_str(&std::fs::read_to_string(router_args.conf).unwrap()).unwrap();

            config
                .run(
                    router_args.rscript,
                    router_args.quarto,
                    router_args.ip_from.into(),
                    router_args.host.parse()?,
                    signal,
                )
                .await?;
        }
    }

    Ok(())
}
