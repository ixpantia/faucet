use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The host to bind to.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// The port to bind to.
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// The number of threads to use to handle requests.
    #[arg(short, long, default_value_t = num_cpus::get())]
    pub threads: usize,

    /// To use the on-prem backend or the k8 backend.
    #[command(subcommand)]
    pub backend: Backend,
}

#[derive(Subcommand, Debug)]
pub enum Backend {
    Local(OnPremArgs),
    K8s(K8Args),
}

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
pub struct OnPremArgs {
    /// The directory containing the plumber files.
    #[arg(short, long, default_value = ".")]
    pub dir: std::path::PathBuf,

    /// The port to start the child processes on.
    ///
    /// NOTE: The child process will be started on this port and the next n-1 ports.
    #[arg(short, long, default_value_t = 8000)]
    pub child_port: u16,

    /// The number of child processes to start and use.
    ///
    /// NOTE: If not specified, the number of child processes will be equal to the number of CPUs.
    #[arg(short, long, default_value_t = num_cpus::get())]
    pub workers: usize,
}

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
pub struct K8Args {
    /// URL (with Port) of the Kubernetes Headless Service specifying the Plumber pods.
    ///
    /// Example: http://plumber:8080
    #[arg(long)]
    pub service_url: hyper::Uri,
}
