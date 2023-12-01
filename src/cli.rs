use std::path::{Path, PathBuf};

use clap::Parser;

use crate::{load_balancing, worker::WorkerType};

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum ServerType {
    Plumber,
    Shiny,
    Auto,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum Strategy {
    RoundRobin,
    RoundRobinIpHash,
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
#[command(author, version, about, long_about = None, verbatim_doc_comment)]
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
}

impl Args {
    pub fn parse() -> Self {
        Self::parse_from(std::env::args_os())
    }
    pub fn strategy(&self) -> load_balancing::Strategy {
        use Strategy::*;
        match self.strategy {
            RoundRobin => load_balancing::Strategy::RoundRobinSimple,
            RoundRobinIpHash => load_balancing::Strategy::RoundRobinIpHash,
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
                if self.dir.join("plumber.R").exists() {
                    WorkerType::Plumber
                } else {
                    WorkerType::Shiny
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
}
