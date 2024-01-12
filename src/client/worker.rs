use crate::{
    error::{FaucetError, FaucetResult},
    networking::get_available_socket,
};
use std::{
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
    rscript: impl AsRef<Path>,
    workdir: impl AsRef<Path>,
    command: impl AsRef<str>,
) -> FaucetResult<Child> {
    tokio::process::Command::new(rscript.as_ref())
        // Set the current directory to the directory containing the entrypoint
        .current_dir(workdir)
        .arg("-e")
        .arg(command.as_ref())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // Set the port environment variable `PORT` to the port we want to use
        // This is needed to make sure the child process is killed when the parent is dropped
        .kill_on_drop(true)
        .spawn()
        .map_err(Into::into)
}

fn spawn_plumber_worker(
    rscript: impl AsRef<Path>,
    workdir: impl AsRef<Path>,
    port: u16,
    target: &'static str,
) -> FaucetResult<Child> {
    let command = format!(
        r#"
        options("plumber.port" = {port})
        plumber::pr_run(plumber::plumb())
        "#,
    );
    let child = spawn_child_rscript_process(rscript, workdir, command)?;

    log_stdio(child, target)
}

fn spawn_shiny_worker(
    rscript: impl AsRef<Path>,
    workdir: impl AsRef<Path>,
    port: u16,
    target: &'static str,
) -> FaucetResult<Child> {
    let command = format!(
        r#"
        options("shiny.port" = {port})
        shiny::runApp()
        "#,
    );
    let child = spawn_child_rscript_process(rscript, workdir, command)?;

    log_stdio(child, target)
}

impl WorkerType {
    fn spawn_process(
        self,
        rscript: impl AsRef<Path>,
        workdir: impl AsRef<Path>,
        port: u16,
        target: &'static str,
    ) -> FaucetResult<Child> {
        match self {
            WorkerType::Plumber => spawn_plumber_worker(rscript, workdir, port, target),
            WorkerType::Shiny => spawn_shiny_worker(rscript, workdir, port, target),
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

const RECHECK_INTERVAL: Duration = Duration::from_millis(10);

fn spawn_worker_task(
    rscript: Arc<Path>,
    addr: SocketAddr,
    worker_type: WorkerType,
    workdir: Arc<Path>,
    is_online: Arc<AtomicBool>,
    target: &'static str,
) -> JoinHandle<FaucetResult<()>> {
    tokio::spawn(async move {
        loop {
            let mut child = worker_type.spawn_process(
                rscript.as_ref(),
                workdir.as_ref(),
                addr.port(),
                target,
            )?;
            let pid = child.id().expect("Failed to get plumber worker PID");
            log::info!(target: "faucet", "Starting process {pid} for {target}");
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
        rscript: Arc<Path>,
        worker_type: WorkerType,
        workdir: Arc<Path>,
        id: usize,
    ) -> FaucetResult<Self> {
        let target = Box::leak(format!("Worker::{}", id).into_boxed_str());
        let socket_addr = get_available_socket().await?;
        let is_online = Arc::new(AtomicBool::new(false));
        let worker_task = spawn_worker_task(
            rscript,
            socket_addr,
            worker_type,
            workdir,
            is_online.clone(),
            target,
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
    rscript: Arc<Path>,
}

impl Workers {
    pub(crate) fn new(worker_type: WorkerType, workdir: Arc<Path>, rscript: Arc<Path>) -> Self {
        Self {
            workers: Vec::new(),
            worker_type,
            workdir,
            rscript,
        }
    }
    pub(crate) async fn spawn(&mut self, n: NonZeroUsize) -> FaucetResult<()> {
        for id in 0..(n.get()) {
            self.workers.push(
                Worker::new(
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
