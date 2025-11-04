use crate::{
    error::{FaucetError, FaucetResult},
    leak,
    networking::get_available_socket,
    server::{
        logging::{parse_faucet_event, FaucetEventResult},
        FaucetServerConfig,
    },
    shutdown::ShutdownSignal,
    telemetry::send_log_event,
};
use std::{
    ffi::OsStr,
    net::SocketAddr,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::{
    process::Child,
    sync::{Mutex, Notify},
    task::JoinHandle,
};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Deserialize)]
pub enum WorkerType {
    #[serde(alias = "plumber", alias = "Plumber")]
    Plumber,
    #[serde(alias = "shiny", alias = "Shiny")]
    Shiny,
    #[serde(alias = "quarto-shiny", alias = "QuartoShiny", alias = "quarto_shiny")]
    QuartoShiny,
    #[serde(alias = "fast-api", alias = "FastAPI")]
    FastAPI,
    #[cfg(test)]
    Dummy,
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
                match parse_faucet_event(&line) {
                    FaucetEventResult::Output(line) => log::warn!(target: target, "{line}"),
                    FaucetEventResult::Event(e) => {
                        send_log_event(e);
                    }
                    FaucetEventResult::EventError(e) => {
                        log::error!(target: target, "{e:?}")
                    }
                }
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
pub struct WorkerConfig {
    pub wtype: WorkerType,
    pub app_dir: Option<&'static str>,
    pub rscript: &'static OsStr,
    pub uv: &'static OsStr,
    pub quarto: &'static OsStr,
    pub workdir: &'static Path,
    pub addr: SocketAddr,
    pub target: &'static str,
    pub worker_id: usize,
    pub worker_route: Option<&'static str>,
    pub is_online: &'static AtomicBool,
    pub qmd: Option<&'static Path>,
    pub handle: &'static Mutex<Option<JoinHandle<FaucetResult<()>>>>,
    pub shutdown: &'static ShutdownSignal,
    pub idle_stop: &'static Notify,
}

impl WorkerConfig {
    fn new(
        worker_id: usize,
        addr: SocketAddr,
        server_config: &FaucetServerConfig,
        shutdown: &'static ShutdownSignal,
    ) -> Self {
        Self {
            addr,
            worker_id,
            is_online: leak!(AtomicBool::new(false)),
            workdir: server_config.workdir,
            worker_route: server_config.route,
            target: leak!(format!("Worker::{}", worker_id)),
            app_dir: server_config.app_dir,
            wtype: server_config.server_type,
            rscript: server_config.rscript,
            quarto: server_config.quarto,
            qmd: server_config.qmd,
            uv: server_config.uv,
            handle: leak!(Mutex::new(None)),
            shutdown,
            idle_stop: leak!(Notify::new()),
        }
    }
    #[allow(dead_code)]
    #[cfg(test)]
    pub fn dummy(target: &'static str, addr: &str, online: bool) -> WorkerConfig {
        WorkerConfig {
            target,
            is_online: leak!(AtomicBool::new(online)),
            addr: addr.parse().unwrap(),
            app_dir: None,
            worker_route: None,
            rscript: OsStr::new(""),
            wtype: WorkerType::Dummy,
            worker_id: 1,
            quarto: OsStr::new(""),
            uv: OsStr::new(""),
            workdir: Path::new("."),
            qmd: None,
            handle: leak!(Mutex::new(None)),
            shutdown: leak!(ShutdownSignal::new()),
            idle_stop: leak!(Notify::new()),
        }
    }
}

fn spawn_child_fastapi_server(config: &WorkerConfig) -> FaucetResult<Child> {
    let mut cmd = tokio::process::Command::new(config.uv);

    // Set the current directory to the directory containing the entrypoint
    cmd.current_dir(config.workdir)
        .args(["run", "fastapi", "run", "--workers", "1"])
        .arg("--port")
        .arg(config.addr.port().to_string())
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

fn spawn_child_rscript_process(
    config: &WorkerConfig,
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

fn spawn_plumber_worker(config: &WorkerConfig) -> FaucetResult<Child> {
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

fn spawn_shiny_worker(config: &WorkerConfig) -> FaucetResult<Child> {
    let command = format!(
        r###"
        options("shiny.port" = {port})
        options(shiny.http.response.filter = function(...) {{
          response <- list(...)[[length(list(...))]]
          if (response$status < 200 || response$status > 300) return(response)
          if ('file' %in% names(response$content)) return(response)
          if (!grepl("^text/html", response$content_type, perl = T)) return(response)
          if (is.raw(response$content)) response$content <- rawToChar(response$content)
          response$content <- sub("</head>", '<script src="__faucet__/reconnect.js"></script></head>', response$content, ignore.case = T)
          return(response)
        }})
        shiny::runApp("{app_dir}")
        "###,
        port = config.addr.port(),
        app_dir = config.app_dir.unwrap_or(".")
    );
    let child = spawn_child_rscript_process(config, command)?;

    log_stdio(child, config.target)
}

fn spawn_quarto_shiny_worker(config: &WorkerConfig) -> FaucetResult<Child> {
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
    fn spawn_process(&self) -> FaucetResult<Child> {
        let child_result = match self.wtype {
            WorkerType::Plumber => spawn_plumber_worker(self),
            WorkerType::Shiny => spawn_shiny_worker(self),
            WorkerType::QuartoShiny => spawn_quarto_shiny_worker(self),
            WorkerType::FastAPI => spawn_child_fastapi_server(self),
            #[cfg(test)]
            WorkerType::Dummy => unreachable!(
                "WorkerType::Dummy should be handled in spawn_worker_task and not reach spawn_process"
            ),
        };

        match child_result {
            Ok(child) => Ok(child),
            Err(e) => {
                log::error!(target: "faucet", "Failed to invoke R for {target}: {e}", target = self.target);
                Err(e)
            }
        }
    }
    pub async fn wait_until_done(&self) {
        if let Some(handle) = self.handle.lock().await.take() {
            log::debug!("Waiting for process to be finished");
            match handle.await {
                Ok(Ok(_)) => {
                    log::debug!("Task ended successfully!")
                }
                Ok(Err(e)) => {
                    panic!("Worker task for target '{}' failed: {:?}", self.target, e);
                }
                Err(e) => {
                    panic!(
                        "Worker task for target '{}' panicked or was cancelled: {:?}",
                        self.target, e
                    );
                }
            }
        }
    }
    pub async fn spawn_worker_task(&'static self) {
        let mut handle = self.handle.lock().await;

        if let Some(handle) = handle.as_ref() {
            if !handle.is_finished() {
                log::warn!(target: "faucet", "Worker task for {target} is already running, skipping spawn", target = self.target);
                return;
            }
        }

        *handle = Some(tokio::spawn(async move {
            #[cfg(test)]
            if self.wtype == WorkerType::Dummy {
                log::debug!(
                    target: "faucet",
                    "Worker {target} is type Dummy, skipping real process spawn.",
                    target = self.target
                );
                return FaucetResult::Ok(());
            }

            'outer: loop {
                let mut child = match self.spawn_process() {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!(
                            target: "faucet",
                            "Worker task for {target} failed to spawn initial process: {e}",
                            target = self.target
                        );
                        return Err(e);
                    }
                };

                let pid = match child.id() {
                    Some(id) => id,
                    None => {
                        let err_msg = format!(
                            "Spawned process for {target} has no PID",
                            target = self.target
                        );
                        log::error!(target: "faucet", "{err_msg}");
                        return Err(FaucetError::Unknown(err_msg));
                    }
                };

                // We will run this loop asynchrnously on this same thread.
                // We will use this to wait for either the stop signal
                // or the child exiting
                let child_loop = async {
                    log::info!(target: "faucet", "Starting process {pid} for {target} on port {port}", port = self.addr.port(), target = self.target);
                    loop {
                        // Try to connect to the socket
                        let check_status = check_if_online(self.addr).await;
                        // If it's online, we can break out of the loop and start serving connections
                        if check_status {
                            log::info!(target: "faucet", "{target} is online and ready to serve connections at {route}", target = self.target, route = self.worker_route.unwrap_or("/"));
                            self.is_online.store(check_status, Ordering::SeqCst);
                            break;
                        }
                        // If it's not online but the child process has exited, we should break out of the loop
                        // and restart the process
                        if child.try_wait()?.is_some() {
                            break;
                        }

                        tokio::time::sleep(RECHECK_INTERVAL).await;
                    }
                    FaucetResult::Ok(child.wait().await?)
                };
                tokio::select! {
                    // If we receive a stop signal that means we will stop the outer loop
                    // and kill the process
                    _ = self.shutdown.wait() => {
                        let _ = child.kill().await;
                        log::info!(target: "faucet", "{target}'s process ({pid}) killed for shutdown", target = self.target);
                        break 'outer;
                    },
                    _ = self.idle_stop.notified() => {
                        self.is_online.store(false, std::sync::atomic::Ordering::SeqCst);
                        let _ = child.kill().await;
                        log::info!(target: "faucet", "{target}'s process ({pid}) killed for idle stop", target = self.target);
                        break 'outer;
                    },
                    // If our child loop stops that means the process crashed. We will restart it
                    status = child_loop => {
                       self
                            .is_online
                            .store(false, std::sync::atomic::Ordering::SeqCst);
                        log::error!(target: "faucet", "{target}'s process ({}) exited with status {}", pid, status?, target = self.target);
                        continue 'outer;
                    }
                }
            }
            log::debug!("{target}'s process has ended.", target = self.target);
            FaucetResult::Ok(())
        }));
    }
}

async fn check_if_online(addr: SocketAddr) -> bool {
    let stream = tokio::net::TcpStream::connect(addr).await;
    stream.is_ok()
}

const RECHECK_INTERVAL: Duration = Duration::from_millis(250);

pub struct WorkerConfigs {
    pub workers: Box<[&'static WorkerConfig]>,
}

const TRIES: usize = 20;

impl WorkerConfigs {
    pub(crate) async fn new(
        server_config: FaucetServerConfig,
        shutdown: &'static ShutdownSignal,
    ) -> FaucetResult<Self> {
        let mut workers =
            Vec::<&'static WorkerConfig>::with_capacity(server_config.n_workers.get());

        for id in 0..server_config.n_workers.get() {
            // Probably hacky but it works. I need to guarantee that ports are never
            // reused
            let socket_addr = 'find_socket: loop {
                let addr_candidate = get_available_socket(TRIES).await?;
                // Check if another worker has already reserved this port
                if workers.iter().any(|w| w.addr == addr_candidate) {
                    continue 'find_socket;
                }
                break 'find_socket addr_candidate;
            };

            let config = leak!(WorkerConfig::new(
                id + 1,
                socket_addr,
                &server_config,
                shutdown
            )) as &'static WorkerConfig;
            workers.push(config);
        }

        let workers = workers.into_boxed_slice();

        Ok(Self { workers })
    }
}
