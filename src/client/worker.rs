use crate::{
    error::{FaucetError, FaucetResult},
    networking::get_available_sockets,
};
use std::{
    ffi::OsStr,
    net::SocketAddr,
    num::NonZeroUsize,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
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

fn spawn_child_rscript_process(
    rscript: impl AsRef<OsStr>,
    workdir: impl AsRef<Path>,
    command: impl AsRef<str>,
    worker_id: usize,
) -> FaucetResult<Child> {
    tokio::process::Command::new(rscript)
        // Set the current directory to the directory containing the entrypoint
        .current_dir(workdir)
        .arg("-e")
        .arg(command.as_ref())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("FAUCET_WORKER_ID", worker_id.to_string())
        // This is needed to make sure the child process is killed when the parent is dropped
        .kill_on_drop(true)
        .spawn()
        .map_err(Into::into)
}

fn spawn_plumber_worker(
    rscript: impl AsRef<OsStr>,
    workdir: impl AsRef<Path>,
    port: u16,
    target: &'static str,
    worker_id: usize,
) -> FaucetResult<Child> {
    let command = format!(
        r#"
        options("plumber.port" = {port})
        plumber::pr_run(plumber::plumb())
        "#,
    );
    let child = spawn_child_rscript_process(rscript, workdir, command, worker_id)?;

    log_stdio(child, target)
}

fn spawn_shiny_worker(
    rscript: impl AsRef<OsStr>,
    workdir: impl AsRef<Path>,
    port: u16,
    target: &'static str,
    worker_id: usize,
) -> FaucetResult<Child> {
    let command = format!(
        r#"
        options("shiny.port" = {port})
        shiny::runApp()
        "#,
    );
    let child = spawn_child_rscript_process(rscript, workdir, command, worker_id)?;

    log_stdio(child, target)
}

impl WorkerType {
    fn spawn_process(
        self,
        rscript: impl AsRef<OsStr>,
        workdir: impl AsRef<Path>,
        port: u16,
        target: &'static str,
        worker_id: usize,
    ) -> Child {
        let child_result = match self {
            WorkerType::Plumber => spawn_plumber_worker(rscript, workdir, port, target, worker_id),
            WorkerType::Shiny => spawn_shiny_worker(rscript, workdir, port, target, worker_id),
        };
        match child_result {
            Ok(child) => child,
            Err(e) => {
                log::error!(target: "faucet", "Failed to invoke R for {target}: {e}");
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
    socket_addr: SocketAddr,
    /// Atomic boolean with the current state of the worker
    is_online: Arc<AtomicBool>,
    /// Target of the worker
    target: &'static str,
}

async fn check_if_online(addr: SocketAddr) -> bool {
    let stream = tokio::net::TcpStream::connect(addr).await;
    stream.is_ok()
}

const RECHECK_INTERVAL: Duration = Duration::from_millis(250);

fn spawn_worker_task(
    rscript: Arc<OsStr>,
    addr: SocketAddr,
    worker_type: WorkerType,
    workdir: Arc<Path>,
    is_online: Arc<AtomicBool>,
    target: &'static str,
    id: usize,
) -> JoinHandle<FaucetResult<()>> {
    tokio::spawn(async move {
        let port = addr.port();
        loop {
            let mut child = worker_type.spawn_process(&rscript, &workdir, port, target, id);
            let pid = child.id().expect("Failed to get plumber worker PID");
            log::info!(target: "faucet", "Starting process {pid} for {target} on port {port}");
            loop {
                // Try to connect to the socket
                let check_status = check_if_online(addr).await;
                // If it's online, we can break out of the loop and start serving connections
                if check_status {
                    log::info!(target: "faucet", "{target} is online and ready to serve connections");
                    is_online.store(check_status, std::sync::atomic::Ordering::SeqCst);
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
            is_online.store(false, std::sync::atomic::Ordering::SeqCst);
            log::error!(target: "faucet", "{target}'s process ({}) exited with status {}", pid, status);
        }
    })
}

impl Worker {
    pub async fn new(
        socket_addr: SocketAddr,
        rscript: Arc<OsStr>,
        worker_type: WorkerType,
        workdir: Arc<Path>,
        id: usize,
    ) -> FaucetResult<Self> {
        let target = Box::leak(format!("Worker::{}", id).into_boxed_str());
        let is_online = Arc::new(AtomicBool::new(false));
        let worker_task = spawn_worker_task(
            rscript,
            socket_addr,
            worker_type,
            workdir,
            is_online.clone(),
            target,
            id,
        );
        Ok(Self {
            _worker_task: worker_task,
            is_online,
            socket_addr,
            target,
        })
    }
    pub fn state(&self) -> WorkerState {
        WorkerState {
            target: self.target,
            is_online: Arc::clone(&self.is_online),
            socket_addr: self.socket_addr,
        }
    }
}

#[derive(Clone)]
pub(crate) struct WorkerState {
    pub(super) target: &'static str,
    pub(super) is_online: Arc<AtomicBool>,
    pub(super) socket_addr: SocketAddr,
}

impl WorkerState {
    pub fn target(&self) -> &'static str {
        self.target
    }
    pub fn is_online(&self) -> bool {
        self.is_online.load(std::sync::atomic::Ordering::SeqCst)
    }
    pub fn socket_addr(&self) -> SocketAddr {
        self.socket_addr
    }
}

pub(crate) struct Workers {
    workers: Vec<Worker>,
    worker_type: WorkerType,
    workdir: Arc<Path>,
    rscript: Arc<OsStr>,
}

impl Workers {
    pub(crate) fn new(worker_type: WorkerType, workdir: Arc<Path>, rscript: Arc<OsStr>) -> Self {
        Self {
            workers: Vec::new(),
            worker_type,
            workdir,
            rscript,
        }
    }
    pub(crate) async fn spawn(&mut self, n: NonZeroUsize) -> FaucetResult<()> {
        let socket_addrs = get_available_sockets(n.get()).await;
        for (id, socket_addr) in socket_addrs.enumerate() {
            self.workers.push(
                Worker::new(
                    socket_addr,
                    Arc::clone(&self.rscript),
                    self.worker_type,
                    Arc::clone(&self.workdir),
                    id + 1,
                )
                .await?,
            );
        }
        Ok(())
    }
    pub(crate) fn get_workers_state(&self) -> Vec<WorkerState> {
        self.workers.iter().map(|w| w.state()).collect()
    }
}
