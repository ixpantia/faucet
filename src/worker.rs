use crate::error::{FaucetError, FaucetResult};
use std::{
    net::SocketAddr,
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

fn log_stdio(mut child: Child, id: usize) -> FaucetResult<Child> {
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
        let target = format!("Worker::{}", id);
        while let Some(line) = stderr.next().await {
            if let Ok(line) = line {
                log::warn!(target: &target, "{line}");
            }
        }
    });

    tokio::spawn(async move {
        let target = format!("Worker::{}", id);
        while let Some(line) = stdout.next().await {
            if let Ok(line) = line {
                log::info!(target: &target, "{line}");
            }
        }
    });

    Ok(child)
}

fn spawn_plumber_worker(workdir: impl AsRef<Path>, port: u16, id: usize) -> FaucetResult<Child> {
    let command = format!(
        r#"
        options("plumber.port" = {port})
        plumber::pr_run(plumber::pr("plumber.R"))
        "#,
    );
    let child = tokio::process::Command::new("Rscript")
        // Set the current directory to the directory containing the entrypoint
        .current_dir(workdir)
        .arg("-e")
        .arg(command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // Set the port environment variable `PORT` to the port we want to use
        // This is needed to make sure the child process is killed when the parent is dropped
        .kill_on_drop(true)
        .spawn()?;

    log_stdio(child, id)
}

fn spawn_shiny_worker(workdir: impl AsRef<Path>, port: u16, id: usize) -> FaucetResult<Child> {
    let command = format!(
        r#"
        options("shiny.port" = {port})
        shiny::runApp()
        "#,
    );
    let child = tokio::process::Command::new("Rscript")
        // Set the current directory to the directory containing the entrypoint
        .current_dir(workdir)
        .arg("-e")
        .arg(command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // Set the port environment variable `PORT` to the port we want to use
        // This is needed to make sure the child process is killed when the parent is dropped
        .kill_on_drop(true)
        .spawn()?;

    log_stdio(child, id)
}

impl WorkerType {
    fn spawn_process(self, workdir: impl AsRef<Path>, port: u16, id: usize) -> FaucetResult<Child> {
        match self {
            WorkerType::Plumber => spawn_plumber_worker(workdir, port, id),
            WorkerType::Shiny => spawn_shiny_worker(workdir, port, id),
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
    /// Id of the worker
    id: usize,
}

fn get_available_socket() -> FaucetResult<SocketAddr> {
    use std::net::TcpListener;
    TcpListener::bind("127.0.0.1:0")?
        .local_addr()
        .map_err(Into::into)
}

async fn check_if_online(addr: SocketAddr) -> bool {
    let stream = tokio::net::TcpStream::connect(addr).await;
    stream.is_ok()
}

const RECHECK_INTERVAL: Duration = Duration::from_millis(50);

fn spawn_worker_task(
    addr: SocketAddr,
    worker_type: WorkerType,
    workdir: Arc<Path>,
    is_online: Arc<AtomicBool>,
    id: usize,
) -> JoinHandle<FaucetResult<()>> {
    tokio::spawn(async move {
        loop {
            let mut child = worker_type.spawn_process(workdir.clone(), addr.port(), id)?;
            let pid = child.id().expect("Failed to get plumber worker PID");
            log::info!(target: "faucet", "Starting process {pid} for Worker::{id}");
            loop {
                // Try to connect to the socket
                let check_status = check_if_online(addr).await;
                if check_status || child.try_wait()?.is_some() {
                    log::info!(target: "faucet", "Worker::{} is online and ready to serve connections", id);
                    is_online.store(check_status, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
                tokio::time::sleep(RECHECK_INTERVAL).await;
            }

            let status = child.wait().await?;
            is_online.store(false, std::sync::atomic::Ordering::SeqCst);
            log::error!(target: "faucet", "Worker::{}'s process ({}) exited with status {}", id, pid, status);
        }
    })
}

impl Worker {
    pub fn new(worker_type: WorkerType, workdir: Arc<Path>, id: usize) -> FaucetResult<Self> {
        let socket_addr = get_available_socket()?;
        let is_online = Arc::new(AtomicBool::new(false));
        let worker_task = spawn_worker_task(
            socket_addr,
            worker_type,
            workdir.clone(),
            is_online.clone(),
            id,
        );
        Ok(Self {
            _worker_task: worker_task,
            is_online,
            socket_addr,
            id,
        })
    }
    pub fn state(&self) -> WorkerState {
        WorkerState {
            id: self.id,
            is_online: Arc::clone(&self.is_online),
            socket_addr: self.socket_addr,
        }
    }
}

#[derive(Clone)]
pub(crate) struct WorkerState {
    id: usize,
    is_online: Arc<AtomicBool>,
    socket_addr: SocketAddr,
}

impl WorkerState {
    pub fn id(&self) -> usize {
        self.id
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
}

impl Workers {
    pub(crate) fn new(worker_type: WorkerType, workdir: impl AsRef<Path>) -> Self {
        let workdir = workdir.as_ref();
        Self {
            workers: Vec::new(),
            worker_type,
            workdir: workdir.into(),
        }
    }
    pub(crate) fn spawn(&mut self, n: usize) -> FaucetResult<()> {
        for id in 0..n {
            self.workers
                .push(Worker::new(self.worker_type, self.workdir.clone(), id + 1)?);
        }
        Ok(())
    }
    pub(crate) fn get_workers_state(&self) -> Vec<WorkerState> {
        self.workers.iter().map(|w| w.state()).collect()
    }
}
