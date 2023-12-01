use crate::error::{FaucetError, FaucetResult};
use std::{net::SocketAddr, path::Path};
use tokio::process::Child;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum WorkerType {
    Plumber,
    Shiny,
}

struct Worker {
    /// The child process running the plumber worker.
    _child: Child,
    /// The address of the worker's socket.
    socket_addr: SocketAddr,
}

fn log_stdio(mut child: Child) -> FaucetResult<Child> {
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
        let target = format!("Worker::{}", pid);
        while let Some(line) = stderr.next().await {
            if let Ok(line) = line {
                log::warn!(target: &target, "{line}");
            }
        }
    });

    tokio::spawn(async move {
        let target = format!("Worker::{}", pid);
        while let Some(line) = stdout.next().await {
            if let Ok(line) = line {
                log::info!(target: &target, "{line}");
            }
        }
    });

    Ok(child)
}

fn spawn_plumber_worker(workdir: impl AsRef<Path>, port: u16) -> FaucetResult<Child> {
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

    log_stdio(child)
}

fn spawn_shiny_worker(workdir: impl AsRef<Path>, port: u16) -> FaucetResult<Child> {
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

    log_stdio(child)
}

fn get_available_socket() -> FaucetResult<SocketAddr> {
    use std::net::TcpListener;
    TcpListener::bind("127.0.0.1:0")?
        .local_addr()
        .map_err(Into::into)
}

impl Worker {
    pub fn new(worker_type: WorkerType, workdir: impl AsRef<Path>) -> FaucetResult<Self> {
        use WorkerType::*;
        let addr = get_available_socket()?;
        let child = match worker_type {
            Plumber => spawn_plumber_worker(workdir, addr.port())?,
            Shiny => spawn_shiny_worker(workdir, addr.port())?,
        };
        Ok(Self {
            _child: child,
            socket_addr: addr,
        })
    }
}

pub(crate) struct Workers {
    workers: Vec<Worker>,
    worker_type: WorkerType,
    workdir: Box<Path>,
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
        for _ in 0..n {
            self.workers
                .push(Worker::new(self.worker_type, &self.workdir)?);
        }
        Ok(())
    }
    pub(crate) fn get_socket_addrs(&self) -> Vec<SocketAddr> {
        self.workers
            .iter()
            .map(|w| w.socket_addr)
            .collect::<Vec<_>>()
    }
}
