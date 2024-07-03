use crate::{
    error::{FaucetError, FaucetResult},
    networking::get_available_sockets,
    server::FaucetServerConfig,
};
use std::{ffi::OsStr, net::SocketAddr, path::Path, sync::atomic::AtomicBool, time::Duration};
use tokio::{process::Child, task::JoinHandle};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum WorkerType {
    Plumber,
    Shiny,
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
    pub(crate) workdir: &'static Path,
    pub(crate) addr: SocketAddr,
    pub(crate) target: &'static str,
    pub(crate) worker_id: usize,
    pub(crate) is_online: &'static AtomicBool,
}

impl WorkerConfig {
    fn new(worker_id: usize, addr: SocketAddr, server_config: FaucetServerConfig) -> Self {
        Self {
            addr,
            worker_id,
            is_online: Box::leak(Box::new(AtomicBool::new(false))),
            workdir: server_config.workdir,
            target: Box::leak(format!("Worker::{}", worker_id).into_boxed_str()),
            app_dir: server_config.app_dir,
            wtype: server_config.server_type,
            rscript: server_config.rscript,
        }
    }
    #[allow(dead_code)]
    pub fn dummy(target: &'static str, addr: &str, online: bool) -> WorkerConfig {
        WorkerConfig {
            target,
            is_online: Box::leak(Box::new(AtomicBool::new(online))),
            addr: addr.parse().unwrap(),
            app_dir: None,
            rscript: OsStr::new(""),
            wtype: crate::client::worker::WorkerType::Shiny,
            worker_id: 1,
            workdir: Path::new("."),
        }
    }
}

fn spawn_child_rscript_process(
    config: WorkerConfig,
    command: impl AsRef<str>,
) -> FaucetResult<Child> {
    tokio::process::Command::new(config.rscript)
        // Set the current directory to the directory containing the entrypoint
        .current_dir(config.workdir)
        .arg("-e")
        .arg(command.as_ref())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("FAUCET_WORKER_ID", config.worker_id.to_string())
        // This is needed to make sure the child process is killed when the parent is dropped
        .kill_on_drop(true)
        .spawn()
        .map_err(Into::into)
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

impl WorkerConfig {
    fn spawn_process(self, config: WorkerConfig) -> Child {
        let child_result = match self.wtype {
            WorkerType::Plumber => spawn_plumber_worker(config),
            WorkerType::Shiny => spawn_shiny_worker(config),
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

struct Worker {
    /// Whether the worker should be stopped
    _worker_task: JoinHandle<FaucetResult<()>>,
    /// The address of the worker's socket.
    config: WorkerConfig,
}

async fn check_if_online(addr: SocketAddr) -> bool {
    let stream = tokio::net::TcpStream::connect(addr).await;
    stream.is_ok()
}

const RECHECK_INTERVAL: Duration = Duration::from_millis(250);

fn spawn_worker_task(config: WorkerConfig) -> JoinHandle<FaucetResult<()>> {
    tokio::spawn(async move {
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

            let status = child.wait().await?;
            config
                .is_online
                .store(false, std::sync::atomic::Ordering::SeqCst);
            log::error!(target: "faucet", "{target}'s process ({}) exited with status {}", pid, status, target = config.target);
        }
    })
}

impl Worker {
    pub fn from_config(config: WorkerConfig) -> FaucetResult<Self> {
        let worker_task = spawn_worker_task(config);
        Ok(Self {
            _worker_task: worker_task,
            config,
        })
    }
}

pub(crate) struct Workers {
    workers: Box<[Worker]>,
}

impl Workers {
    pub(crate) async fn new(server_config: FaucetServerConfig) -> FaucetResult<Self> {
        let workers = get_available_sockets(server_config.n_workers.get())
            .await
            .enumerate()
            .map(|(id, socket_addr)| WorkerConfig::new(id + 1, socket_addr, server_config))
            .map(Worker::from_config)
            .collect::<FaucetResult<Box<[Worker]>>>()?;
        Ok(Self { workers })
    }
    pub(crate) fn get_workers_config(&self) -> Vec<WorkerConfig> {
        self.workers.iter().map(|w| w.config).collect()
    }
}
