use clap::Parser;
use faucet_server::cli::{Args, Commands};
use faucet_server::error::FaucetResult;
use faucet_server::server::logger::build_logger;
use faucet_server::server::{FaucetServerBuilder, RouterConfig};
use faucet_server::telemetry::TelemetryManager;
use faucet_server::{cli::Shutdown, shutdown};

#[tokio::main]
pub async fn main() -> FaucetResult<()> {
    dotenv::from_filename(".Renviron").ok();
    dotenv::from_filename(".env").ok();

    let cli_args = Args::parse();

    let shutdown_signal = match cli_args.shutdown {
        Shutdown::Immediate => shutdown::immediate(),
        Shutdown::Graceful => shutdown::graceful(),
    };

    let telemetry = cli_args.pg_con_string.map(|pg_con| {
        match TelemetryManager::start(
            &cli_args.telemetry_namespace,
            cli_args.telemetry_version.as_deref(),
            &pg_con,
        ) {
            Ok(telemetry) => telemetry,
            Err(e) => {
                eprintln!("Unable to start telemetry manager: {e}");
                std::process::exit(1);
            }
        }
    });

    let log_thread_handle = build_logger(
        cli_args
            .log_file
            .as_ref()
            .map_or(faucet_server::server::logger::Target::Stderr, |file| {
                faucet_server::server::logger::Target::File(file.to_path_buf())
            }),
        cli_args.max_log_file_size,
        shutdown_signal,
    );

    match cli_args.command {
        Commands::Start(start_args) => {
            log::info!(target: "faucet", "Building the faucet server...");

            FaucetServerBuilder::new()
                .strategy(Some(start_args.strategy.into()))
                .workers(start_args.workers)
                .server_type(start_args.server_type())
                .extractor(cli_args.ip_from.into())
                .bind(cli_args.host.parse()?)
                .workdir(start_args.dir)
                .rscript(cli_args.rscript)
                .app_dir(start_args.app_dir)
                .quarto(cli_args.quarto)
                .qmd(start_args.qmd)
                .telemetry(telemetry.as_ref())
                .max_rps(start_args.max_rps)
                .build()?
                .run(shutdown_signal)
                .await?;
        }
        Commands::Router(router_args) => {
            let config: RouterConfig =
                toml::from_str(&std::fs::read_to_string(router_args.conf).unwrap()).unwrap();

            config
                .run(
                    cli_args.rscript,
                    cli_args.quarto,
                    cli_args.ip_from.into(),
                    cli_args.host.parse()?,
                    shutdown_signal,
                    telemetry.as_ref(),
                )
                .await?;
        }
    }

    if let Some(telemetry) = telemetry {
        log::debug!("Waiting to stop DB writes");
        drop(telemetry.sender);
        let _ = telemetry.http_events_join_handle.await;
    }

    if let Some(handle) = log_thread_handle {
        let _ = handle.await;
    }

    Ok(())
}
