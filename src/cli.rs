use std::path::{Path, PathBuf};

use clap::Parser;

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
    Auto,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum Strategy {
    /// Sends requests to workers in a round-robin fashion.
    RoundRobin,
    /// Hashes the IP address of the client to determine which worker to send the request to.
    IpHash,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum IpFrom {
    Client,
    XForwardedFor,
    XRealIp,
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
#[derive(Parser, Debug)]
#[command(author, version, verbatim_doc_comment)]
pub struct Args {
    /// The host to bind to.
    #[arg(long, env = "FAUCET_HOST", default_value = "127.0.0.1:3838")]
    host: String,

    /// The number of threads to use to handle requests.
    #[arg(short, long, env = "FAUCET_WORKERS", default_value_t = num_cpus::get())]
    workers: usize,

    /// The load balancing strategy to use.
    #[arg(short, long, env = "FAUCET_STRATEGY", default_value = "round-robin")]
    strategy: Strategy,

    /// The type of workers to spawn.
    #[arg(short, long, env = "FAUCET_TYPE", default_value = "auto")]
    type_: ServerType,

    /// The directory to spawn workers in.
    /// Defaults to the current directory.
    #[arg(short, long, env = "FAUCET_DIR", default_value = ".")]
    dir: PathBuf,

    /// The IP address to extract from.
    /// Defaults to client address.
    #[arg(short, long, env = "FAUCET_IP_FROM", default_value = "client")]
    ip_from: IpFrom,
}

impl Args {
    pub fn parse() -> Self {
        Self::parse_from(std::env::args_os())
    }
    pub fn strategy(&self) -> load_balancing::Strategy {
        use Strategy::*;
        match self.strategy {
            RoundRobin => load_balancing::Strategy::RoundRobin,
            IpHash => load_balancing::Strategy::IpHash,
        }
    }
    pub fn workers(&self) -> usize {
        self.workers
    }
    pub fn server_type(&self) -> WorkerType {
        match self.type_ {
            ServerType::Plumber => WorkerType::Plumber,
            ServerType::Shiny => WorkerType::Shiny,
            ServerType::Auto => {
                if is_plumber(&self.dir) {
                    WorkerType::Plumber
                } else if is_shiny(&self.dir) {
                    WorkerType::Shiny
                } else {
                    panic!("Could not determine worker type. Please specify with --type.");
                }
            }
        }
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn dir(&self) -> &Path {
        &self.dir
    }
    pub fn ip_extractor(&self) -> load_balancing::IpExtractor {
        match self.ip_from {
            IpFrom::Client => load_balancing::IpExtractor::ClientAddr,
            IpFrom::XForwardedFor => load_balancing::IpExtractor::XForwardedFor,
            IpFrom::XRealIp => load_balancing::IpExtractor::XRealIp,
        }
    }
}
