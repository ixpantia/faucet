use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The directory containing the plumber files.
    #[arg(short, long, default_value = ".")]
    pub dir: std::path::PathBuf,

    /// The host to bind to.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// The port to bind to.
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

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
