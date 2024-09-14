use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};

use crate::client::{load_balancing, worker::WorkerType};

fn is_plumber(dir: &Path) -> bool {
    let plumber = dir.join("plumber.R");
    let plumber_entrypoint = dir.join("entrypoint.R");
    plumber.exists() || plumber_entrypoint.exists()
}

fn is_shiny(dir: &Path) -> bool {
    let shiny_app = dir.join("app.R");
    let shiny_ui = dir.join("ui.R");
    let shiny_server = dir.join("server.R");
    shiny_app.exists() || (shiny_ui.exists() && shiny_server.exists())
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum ServerType {
    Plumber,
    Shiny,
    QuartoShiny,
    Auto,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Strategy {
    /// Sends requests to workers in a round-robin fashion.
    RoundRobin,
    /// Hashes the IP address of the client to determine which worker to send the request to.
    IpHash,
}

impl From<Strategy> for load_balancing::Strategy {
    fn from(value: Strategy) -> Self {
        match value {
            Strategy::RoundRobin => load_balancing::Strategy::RoundRobin,
            Strategy::IpHash => load_balancing::Strategy::IpHash,
        }
    }
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum IpFrom {
    Client,
    XForwardedFor,
    XRealIp,
}

impl From<IpFrom> for load_balancing::IpExtractor {
    fn from(value: IpFrom) -> Self {
        match value {
            IpFrom::Client => load_balancing::IpExtractor::ClientAddr,
            IpFrom::XForwardedFor => load_balancing::IpExtractor::XForwardedFor,
            IpFrom::XRealIp => load_balancing::IpExtractor::XRealIp,
        }
    }
}

#[derive(Parser, Debug)]
pub struct StartArgs {
    /// The host to bind to.
    #[arg(long, env = "FAUCET_HOST", default_value = "127.0.0.1:3838")]
    pub host: String,

    /// The number of threads to use to handle requests.
    #[arg(short, long, env = "FAUCET_WORKERS", default_value_t = num_cpus::get())]
    pub workers: usize,

    /// The load balancing strategy to use.
    #[arg(short, long, env = "FAUCET_STRATEGY", default_value = "round-robin")]
    pub strategy: Strategy,

    /// The type of workers to spawn.
    #[arg(short, long, env = "FAUCET_TYPE", default_value = "auto")]
    type_: ServerType,

    /// The directory to spawn workers in.
    /// Defaults to the current directory.
    #[arg(short, long, env = "FAUCET_DIR", default_value = ".")]
    pub dir: PathBuf,

    /// The IP address to extract from.
    /// Defaults to client address.
    #[arg(short, long, env = "FAUCET_IP_FROM", default_value = "client")]
    pub ip_from: IpFrom,

    /// Command, path, or executable to run Rscript.
    #[arg(long, short, env = "FAUCET_RSCRIPT", default_value = "Rscript")]
    pub rscript: OsString,

    /// Command, path, or executable to run quarto.
    #[arg(long, env = "FAUCET_QUARTO", default_value = "quarto")]
    pub quarto: OsString,

    /// Argument passed on to `appDir` when running Shiny.
    #[arg(long, short, env = "FAUCET_APP_DIR", default_value = None)]
    pub app_dir: Option<String>,

    /// Quarto Shiny file path.
    #[arg(long, short, env = "FAUCET_QMD", default_value = None)]
    pub qmd: Option<PathBuf>,

    /// Save logs to a file. Will disable colors!
    #[arg(long, short, env = "FAUCET_LOG_FILE", default_value = None)]
    pub log_file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct RouterArgs {
    /// The host to bind to.
    #[arg(long, env = "FAUCET_HOST", default_value = "127.0.0.1:3838")]
    pub host: String,

    /// The IP address to extract from.
    /// Defaults to client address.
    #[arg(short, long, env = "FAUCET_IP_FROM", default_value = "client")]
    pub ip_from: IpFrom,

    /// Command, path, or executable to run Rscript.
    #[arg(long, short, env = "FAUCET_RSCRIPT", default_value = "Rscript")]
    pub rscript: OsString,

    /// Command, path, or executable to run quarto.
    #[arg(long, short, env = "FAUCET_QUARTO", default_value = "quarto")]
    pub quarto: OsString,

    /// Save logs to a file. Will disable colors!
    #[arg(long, short, env = "FAUCET_LOG_FILE", default_value = None)]
    pub log_file: Option<PathBuf>,

    /// Router config file.
    #[arg(
        long,
        short,
        env = "FAUCET_ROUTER_CONF",
        default_value = "./frouter.toml"
    )]
    pub conf: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start a simple faucet server.
    #[command(name = "start")]
    Start(StartArgs),
    /// Runs faucet in "router" mode. (Experimental)
    #[command(name = "router")]
    Router(RouterArgs),
}

///
/// ███████╗ █████╗ ██╗   ██╗ ██████╗███████╗████████╗
/// ██╔════╝██╔══██╗██║   ██║██╔════╝██╔════╝╚══██╔══╝
/// █████╗  ███████║██║   ██║██║     █████╗     ██║
/// ██╔══╝  ██╔══██║██║   ██║██║     ██╔══╝     ██║
/// ██║     ██║  ██║╚██████╔╝╚██████╗███████╗   ██║
/// ╚═╝     ╚═╝  ╚═╝ ╚═════╝  ╚═════╝╚══════╝   ╚═╝
/// Fast, async, and concurrent data applications.
///
#[derive(Parser)]
#[command(author, version, verbatim_doc_comment)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

impl StartArgs {
    pub fn server_type(&self) -> WorkerType {
        match self.type_ {
            ServerType::Plumber => WorkerType::Plumber,
            ServerType::Shiny => WorkerType::Shiny,
            ServerType::QuartoShiny => WorkerType::QuartoShiny,
            ServerType::Auto => {
                if is_plumber(&self.dir) {
                    WorkerType::Plumber
                } else if is_shiny(&self.dir) {
                    WorkerType::Shiny
                } else {
                    log::error!(target: "faucet", "Could not determine worker type. Please specify with --type.");
                    std::process::exit(1);
                }
            }
        }
    }
}
