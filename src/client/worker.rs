use crate::{
    error::{FaucetError, FaucetResult},
    leak,
    networking::get_available_sockets,
    server::FaucetServerConfig,
};
use std::{
    ffi::OsStr,
    net::SocketAddr,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{process::Child, sync::Mutex, task::JoinHandle};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Deserialize)]
#[serde(rename = "snake_case")]
pub enum WorkerType {
    Plumber,
    Shiny,
    QuartoShiny,
}

fn log_stdio(mut child: Child, target: &'static str) -> FaucetResult<Child> {
    let pid = child.id().expect("Failed to get plumber worker PID");

    let mut stdout = FramedRead::new(
        child.stdout.take().ok_or(FaucetError::Unknown(format!(
            "Unable to take stdout from PID {pid}"
        )))?,
        LinesCodec::new(),
    );

    let mut stderr = FramedRead::new(
        child.stderr.take().ok_or(FaucetError::Unknown(format!(
            "Unable to take stderr from PID {pid}"
        )))?,
        LinesCodec::new(),
    );

    tokio::spawn(async move {
        while let Some(line) = stderr.next().await {
            if let Ok(line) = line {
                log::warn!(target: target, "{line}");
            }
        }
    });

    tokio::spawn(async move {
        while let Some(line) = stdout.next().await {
            if let Ok(line) = line {
                log::info!(target: target, "{line}");
            }
        }
    });

    Ok(child)
}

#[derive(Copy, Clone)]
pub(crate) struct WorkerConfig {
    pub(crate) wtype: WorkerType,
    pub(crate) app_dir: Option<&'static str>,
    pub(crate) rscript: &'static OsStr,
    pub(crate) quarto: &'static OsStr,
    pub(crate) workdir: &'static Path,
    pub(crate) addr: SocketAddr,
    pub(crate) target: &'static str,
    pub(crate) worker_id: usize,
    pub(crate) is_online: &'static AtomicBool,
    pub(crate) qmd: Option<&'static Path>,
}

impl WorkerConfig {
    fn new(
        worker_id: usize,
        addr: SocketAddr,
        server_config: FaucetServerConfig,
        target_prefix: &str,
    ) -> Self {
        Self {
            addr,
            worker_id,
            is_online: leak!(AtomicBool::new(false)),
            workdir: server_config.workdir,
            target: leak!(format!("{}Worker::{}", target_prefix, worker_id)),
            app_dir: server_config.app_dir,
            wtype: server_config.server_type,
            rscript: server_config.rscript,
            quarto: server_config.quarto,
            qmd: server_config.qmd,
        }
    }
    #[allow(dead_code)]
    pub fn dummy(target: &'static str, addr: &str, online: bool) -> WorkerConfig {
        WorkerConfig {
            target,
            is_online: leak!(AtomicBool::new(online)),
            addr: addr.parse().unwrap(),
            app_dir: None,
            rscript: OsStr::new(""),
            wtype: crate::client::worker::WorkerType::Shiny,
            worker_id: 1,
            quarto: OsStr::new(""),
            workdir: Path::new("."),
            qmd: None,
        }
    }
}

fn spawn_child_rscript_process(
    config: WorkerConfig,
    command: impl AsRef<str>,
) -> FaucetResult<Child> {
    let mut cmd = tokio::process::Command::new(config.rscript);

    // Set the current directory to the directory containing the entrypoint
    cmd.current_dir(config.workdir)
        .arg("-e")
        .arg(command.as_ref())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("FAUCET_WORKER_ID", config.worker_id.to_string())
        // This is needed to make sure the child process is killed when the parent is dropped
        .kill_on_drop(true);

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            // Create a new process group for the child process
            nix::libc::setpgid(0, 0);
            Ok(())
        });
    }

    cmd.spawn().map_err(Into::into)
}

fn spawn_plumber_worker(config: WorkerConfig) -> FaucetResult<Child> {
    let command = format!(
        r#"
        options("plumber.port" = {port})
        plumber::pr_run(plumber::plumb())
        "#,
        port = config.addr.port()
    );
    let child = spawn_child_rscript_process(config, command)?;

    log_stdio(child, config.target)
}

fn spawn_shiny_worker(config: WorkerConfig) -> FaucetResult<Child> {
    let command = format!(
        r#"
        options("shiny.port" = {port})
        shiny::runApp("{app_dir}")
        "#,
        port = config.addr.port(),
        app_dir = config.app_dir.unwrap_or(".")
    );
    let child = spawn_child_rscript_process(config, command)?;

    log_stdio(child, config.target)
}

fn spawn_quarto_shiny_worker(config: WorkerConfig) -> FaucetResult<Child> {
    let mut cmd = tokio::process::Command::new(config.quarto);
    // Set the current directory to the directory containing the entrypoint
    cmd.current_dir(config.workdir)
        .arg("serve")
        .args(["--port", config.addr.port().to_string().as_str()])
        .arg(config.qmd.ok_or(FaucetError::MissingArgument("qmd"))?)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("FAUCET_WORKER_ID", config.worker_id.to_string())
        // This is needed to make sure the child process is killed when the parent is dropped
        .kill_on_drop(true);

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            // Create a new process group for the child process
            nix::libc::setpgid(0, 0);
            Ok(())
        });
    }

    let child = cmd.spawn()?;

    log_stdio(child, config.target)
}

impl WorkerConfig {
    fn spawn_process(self, config: WorkerConfig) -> Child {
        let child_result = match self.wtype {
            WorkerType::Plumber => spawn_plumber_worker(config),
            WorkerType::Shiny => spawn_shiny_worker(config),
            WorkerType::QuartoShiny => spawn_quarto_shiny_worker(config),
        };
        match child_result {
            Ok(child) => child,
            Err(e) => {
                log::error!(target: "faucet", "Failed to invoke R for {target}: {e}", target = config.target);
                log::error!(target: "faucet", "Exiting...");
                std::process::exit(1);
            }
        }
    }
}

pub struct Worker {
    /// Whether the worker should be stopped
    pub child: WorkerChild,
    /// The address of the worker's socket.
    pub config: WorkerConfig,
}

async fn check_if_online(addr: SocketAddr) -> bool {
    let stream = tokio::net::TcpStream::connect(addr).await;
    stream.is_ok()
}

const RECHECK_INTERVAL: Duration = Duration::from_millis(250);

pub struct WorkerChild {
    handle: JoinHandle<FaucetResult<()>>,
    stopper: tokio::sync::mpsc::Sender<()>,
}

impl WorkerChild {
    pub fn kill(&self) {
        let _ = self.stopper.try_send(());
    }
}

fn spawn_worker_task(config: WorkerConfig) -> WorkerChild {
    let (stopper, mut rx) = tokio::sync::mpsc::channel(1);
    let handle = tokio::spawn(async move {
        loop {
            let mut child = config.spawn_process(config);
            let pid = child.id().expect("Failed to get plumber worker PID");
            log::info!(target: "faucet", "Starting process {pid} for {target} on port {port}", port = config.addr.port(), target = config.target);
            loop {
                // Try to connect to the socket
                let check_status = check_if_online(config.addr).await;
                // If it's online, we can break out of the loop and start serving connections
                if check_status {
                    log::info!(target: "faucet", "{target} is online and ready to serve connections", target = config.target);
                    config
                        .is_online
                        .store(check_status, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
                // If it's not online but the child process has exited, we should break out of the loop
                // and restart the process
                if child.try_wait()?.is_some() {
                    break;
                }

                tokio::time::sleep(RECHECK_INTERVAL).await;
            }

            tokio::select! {
                _ = child.wait() => (),
                _ = rx.recv() => return FaucetResult::Ok(()),
            }
            let status = child.wait().await?;
            config
                .is_online
                .store(false, std::sync::atomic::Ordering::SeqCst);
            log::error!(target: "faucet", "{target}'s process ({}) exited with status {}", pid, status, target = config.target);
        }
    });
    WorkerChild { handle, stopper }
}

impl Worker {
    pub fn from_config(config: WorkerConfig) -> FaucetResult<Self> {
        let child = spawn_worker_task(config);
        Ok(Self { child, config })
    }
}

pub(crate) struct Workers {
    pub workers: Box<[Worker]>,
}

impl Workers {
    pub(crate) async fn new(
        server_config: FaucetServerConfig,
        target_prefix: &str,
    ) -> FaucetResult<Self> {
        let workers = get_available_sockets(server_config.n_workers.get())
            .await
            .enumerate()
            .map(|(id, socket_addr)| {
                WorkerConfig::new(id + 1, socket_addr, server_config, target_prefix)
            })
            .map(Worker::from_config)
            .collect::<FaucetResult<Box<[Worker]>>>()?;
        Ok(Self { workers })
    }
    pub(crate) fn get_workers_config(&self) -> Vec<WorkerConfig> {
        self.workers.iter().map(|w| w.config).collect()
    }
}
