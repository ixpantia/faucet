use hyper::{http::HeaderValue, Method, Request, Response, Uri, Version};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::onion::{Layer, Service};
use crate::{server::service::State, telemetry::send_http_event};
use std::{net::IpAddr, time};

pub mod logger {
    use std::{io::BufWriter, io::Write, path::PathBuf};

    use hyper::body::Bytes;
    use tokio::task::JoinHandle;

    use crate::shutdown::ShutdownSignal;

    pub enum Target {
        Stderr,
        File(PathBuf),
    }

    struct LogFileWriter {
        sender: tokio::sync::mpsc::Sender<Bytes>,
    }

    impl std::io::Write for LogFileWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let _ = self.sender.try_send(Bytes::copy_from_slice(buf));
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn start_log_writer_thread(
        path: PathBuf,
        max_file_size: Option<u64>,
        shutdown: &'static ShutdownSignal,
    ) -> (LogFileWriter, JoinHandle<()>) {
        let max_file_size = max_file_size.unwrap_or(u64::MAX);
        let mut current_file_size = match std::fs::metadata(&path) {
            Ok(md) => md.len(),
            Err(_) => 0,
        };
        let file = std::fs::File::options()
            .create(true)
            .append(true)
            .truncate(false)
            .open(&path)
            .expect("Unable to open or create log file");

        // Create a file path to a backup of the previous logs with MAX file size
        let mut copy_path = path.clone();
        copy_path.as_mut_os_string().push(".bak");

        let mut writer = BufWriter::new(file);
        let mut stderr = BufWriter::new(std::io::stderr());
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Bytes>(1000);
        let writer_thread = tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    bytes = receiver.recv() => {
                        match bytes {
                            Some(bytes) => {
                                if let Err(e) = stderr.write_all(bytes.as_ref()) {
                                    eprintln!("Unable to write to stderr: {e}");
                                };

                                if let Err(e) = writer.write_all(bytes.as_ref()) {
                                    eprintln!("Unable to write to {path:?}: {e}");
                                };

                                current_file_size += bytes.len() as u64;
                                if current_file_size > max_file_size {
                                    // Flush the writer
                                    let _ = writer.flush();
                                    let file = writer.get_mut();

                                    // Copy the current file to the backup
                                    if let Err(e) = std::fs::copy(&path, &copy_path) {
                                        log::error!("Unable to copy logs to backup file: {e}");
                                    }

                                    // Truncate the logs file
                                    if let Err(e) = file.set_len(0) {
                                        log::error!("Unable to truncate logs file: {e}");
                                    }

                                    current_file_size = 0;
                                }
                            },
                            None => break
                        }
                    },
                    _ = shutdown.wait() => break
                }
            }
            let _ = writer.flush();
            let _ = stderr.flush();
        });
        (LogFileWriter { sender }, writer_thread)
    }

    pub fn build_logger(
        target: Target,
        max_file_size: Option<u64>,
        shutdown: &'static ShutdownSignal,
    ) -> Option<JoinHandle<()>> {
        let (target, handle) = match target {
            Target::File(path) => {
                let (writer, handle) = start_log_writer_thread(path, max_file_size, shutdown);
                (env_logger::Target::Pipe(Box::new(writer)), Some(handle))
            }
            Target::Stderr => (env_logger::Target::Stderr, None),
        };

        let mut env_builder = env_logger::Builder::new();
        env_builder
            .parse_env(env_logger::Env::new().filter_or("FAUCET_LOG", "info"))
            .target(target)
            .init();

        handle
    }
}

#[derive(Clone, Copy)]
pub struct StateData {
    pub uuid: uuid::Uuid,
    pub ip: IpAddr,
    pub worker_route: Option<&'static str>,
    pub worker_id: usize,
    pub target: &'static str,
}

trait StateLogData: Send + Sync + 'static {
    fn get_state_data(&self) -> StateData;
}

impl StateLogData for State {
    #[inline(always)]
    fn get_state_data(&self) -> StateData {
        let uuid = self.uuid;
        let ip = self.remote_addr;
        let worker_id = self.client.config.worker_id;
        let worker_route = self.client.config.worker_route;
        let target = self.client.config.target;
        StateData {
            uuid,
            ip,
            worker_id,
            worker_route,
            target,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum LogOption<T> {
    None,
    Some(T),
}

impl<T> From<Option<T>> for LogOption<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => LogOption::None,
            Some(v) => LogOption::Some(v),
        }
    }
}

impl<T> std::fmt::Display for LogOption<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogOption::None => write!(f, "-"),
            LogOption::Some(v) => write!(f, "{v}"),
        }
    }
}

impl<T> std::fmt::Debug for LogOption<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogOption::None => write!(f, r#""-""#),
            LogOption::Some(v) => write!(f, "{v:?}"),
        }
    }
}

pub struct HttpLogData {
    pub state_data: StateData,
    pub method: Method,
    pub path: Uri,
    pub version: Version,
    pub status: i16,
    pub user_agent: LogOption<HeaderValue>,
    pub elapsed: i64,
}

impl HttpLogData {
    fn log(&self) {
        log::info!(
            target: self.state_data.target,
            r#"{ip} "{method} {route}{path} {version:?}" {status} {user_agent:?} {elapsed}"#,
            route = self.state_data.worker_route.map(|r| r.trim_end_matches('/')).unwrap_or_default(),
            ip = self.state_data.ip,
            method = self.method,
            path = self.path,
            version = self.version,
            status = self.status,
            user_agent = self.user_agent,
            elapsed = self.elapsed,
        );
    }
}

#[inline(always)]
async fn capture_log_data<Body, ResBody, Error, State: StateLogData>(
    inner: &impl Service<Request<Body>, Response = Response<ResBody>, Error = Error>,
    req: Request<Body>,
) -> Result<(Response<ResBody>, HttpLogData), Error> {
    let start = time::Instant::now();

    // Extract request info for logging
    let state = req.extensions().get::<State>().expect("State not found");
    let state_data = state.get_state_data();
    let method = req.method().clone();
    let path = req.uri().clone();
    let version = req.version();
    let headers = req.headers();
    let user_agent: LogOption<_> = headers.get(hyper::header::USER_AGENT).cloned().into();

    // Make the request
    let res = inner.call(req, None).await?;

    // Extract response info for logging
    let status = res.status().as_u16() as i16;
    let elapsed = start.elapsed().as_millis() as i64;

    let log_data = HttpLogData {
        state_data,
        method,
        path,
        version,
        status,
        user_agent,
        elapsed,
    };

    Ok((res, log_data))
}

pub(super) struct LogService<S> {
    inner: S,
}

impl<S, Body, ResBody> Service<Request<Body>> for LogService<S>
where
    S: Service<Request<Body>, Response = Response<ResBody>> + Send + Sync,
{
    type Error = S::Error;
    type Response = Response<ResBody>;

    async fn call(
        &self,
        req: Request<Body>,
        _: Option<IpAddr>,
    ) -> Result<Self::Response, Self::Error> {
        let (res, log_data) = capture_log_data::<_, _, _, State>(&self.inner, req).await?;

        log_data.log();
        send_http_event(log_data);

        Ok(res)
    }
}

pub(super) struct LogLayer {}

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        LogService { inner }
    }
}

#[derive(serde::Deserialize, Clone, Copy)]
pub enum FaucetTracingLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl FaucetTracingLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            FaucetTracingLevel::Trace => "trace",
            FaucetTracingLevel::Debug => "debug",
            FaucetTracingLevel::Error => "error",
            FaucetTracingLevel::Warn => "warn",
            FaucetTracingLevel::Info => "info",
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum L1OrScalar<T> {
    Scalar(T),
    L1([T; 1]),
}

fn deserialize_l1_or_scalar<'de, T, D>(data: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: DeserializeOwned,
{
    let value: L1OrScalar<T> = serde::Deserialize::deserialize(data)?;
    match value {
        L1OrScalar::Scalar(v) => Ok(v),
        L1OrScalar::L1([v]) => Ok(v),
    }
}

#[derive(serde::Deserialize)]
pub struct EventLogData {
    #[serde(deserialize_with = "deserialize_l1_or_scalar")]
    pub target: String,
    #[serde(deserialize_with = "deserialize_l1_or_scalar")]
    pub event_id: Uuid,
    #[serde(deserialize_with = "deserialize_l1_or_scalar")]
    pub level: FaucetTracingLevel,
    #[serde(deserialize_with = "deserialize_l1_or_scalar")]
    pub parent_event_id: Option<Uuid>,
    #[serde(deserialize_with = "deserialize_l1_or_scalar")]
    pub event_type: String,
    #[serde(deserialize_with = "deserialize_l1_or_scalar")]
    pub message: String,
    pub body: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum FaucetEventParseError<'a> {
    UnableToSplit,
    InvalidString(&'a str),
    SerdeError {
        err: serde_json::Error,
        str: &'a str,
    },
}

pub enum FaucetEventResult<'a> {
    Event(EventLogData),
    Output(&'a str),
    EventError(FaucetEventParseError<'a>),
}

pub fn parse_faucet_event(content: &str) -> FaucetEventResult<'_> {
    use FaucetEventResult::*;

    let content = content.trim_end_matches('\n');

    if !content.starts_with("{{ faucet_event }}:") {
        return Output(content);
    }

    match content.split_once(':') {
        Some((_, content)) => {
            let structure: EventLogData = match serde_json::from_str(content.trim()) {
                Ok(structure) => structure,
                Err(e) => {
                    return FaucetEventResult::EventError(FaucetEventParseError::SerdeError {
                        err: e,
                        str: content,
                    })
                }
            };
            Event(structure)
        }
        None => EventError(FaucetEventParseError::UnableToSplit),
    }
}

#[cfg(test)]
mod tests {
    use hyper::StatusCode;

    use super::*;

    #[tokio::test]
    async fn log_capture() {
        #[derive(Clone)]
        struct MockState;

        impl StateLogData for MockState {
            fn get_state_data(&self) -> StateData {
                StateData {
                    uuid: uuid::Uuid::now_v7(),
                    ip: IpAddr::V4([127, 0, 0, 1].into()),
                    target: "test",
                    worker_id: 1,
                    worker_route: None,
                }
            }
        }

        struct Svc;

        impl Service<Request<()>> for Svc {
            type Response = Response<()>;
            type Error = ();
            async fn call(
                &self,
                _: Request<()>,
                _: Option<IpAddr>,
            ) -> Result<Self::Response, Self::Error> {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                Ok(Response::builder().status(StatusCode::OK).body(()).unwrap())
            }
        }

        let req = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .extension(MockState)
            .version(Version::HTTP_11)
            .header(hyper::header::USER_AGENT, "test")
            .body(())
            .unwrap();

        let (_, log_data) = capture_log_data::<_, _, _, MockState>(&Svc, req)
            .await
            .unwrap();

        assert_eq!(log_data.state_data.ip, IpAddr::V4([127, 0, 0, 1].into()));
        assert_eq!(log_data.method, Method::GET);
        assert_eq!(log_data.path, "https://example.com/");
        assert_eq!(log_data.version, Version::HTTP_11);
        assert_eq!(log_data.status, 200);
        assert_eq!(
            log_data.user_agent,
            LogOption::Some(HeaderValue::from_static("test"))
        );
        assert!(log_data.elapsed > 0);
        assert_eq!(log_data.state_data.target, "test");
    }

    #[test]
    fn log_option_display() {
        assert_eq!(LogOption::<u8>::None.to_string(), "-");
        assert_eq!(LogOption::Some(1).to_string(), "1");
    }

    #[test]
    fn log_option_debug() {
        assert_eq!(format!("{:?}", LogOption::<u8>::None), r#""-""#);
        assert_eq!(format!("{:?}", LogOption::Some(1)), "1");
    }

    #[test]
    fn log_option_from_option() {
        assert_eq!(LogOption::<u8>::from(None), LogOption::None);
        assert_eq!(LogOption::from(Some(1)), LogOption::Some(1));
    }

    #[test]
    fn log_data_log() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        struct Buffer(Arc<Mutex<Vec<u8>>>);

        impl Write for Buffer {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().write(buf)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.0.lock().unwrap().flush()
            }
        }

        impl Buffer {
            fn clone_buf(&self) -> Vec<u8> {
                self.0.lock().unwrap().clone()
            }
        }

        impl Clone for Buffer {
            fn clone(&self) -> Self {
                Buffer(Arc::clone(&self.0))
            }
        }

        let log_data = HttpLogData {
            state_data: StateData {
                uuid: uuid::Uuid::now_v7(),
                target: "test",
                ip: IpAddr::V4([127, 0, 0, 1].into()),
                worker_route: None,
                worker_id: 1,
            },
            method: Method::GET,
            path: "https://example.com/".parse().unwrap(),
            version: Version::HTTP_11,
            status: 200,
            user_agent: LogOption::Some(HeaderValue::from_static("test")),
            elapsed: 5,
        };

        let buf = Buffer(Arc::new(Mutex::new(Vec::new())));
        let mut logger = env_logger::Builder::new();
        // ALWAYS USE INFO LEVEL FOR LOGGING
        logger.filter_level(log::LevelFilter::Info);
        logger.format(|f, record| writeln!(f, "{}", record.args()));
        logger.target(env_logger::Target::Pipe(Box::new(buf.clone())));
        logger.init();

        log_data.log();

        let log = String::from_utf8(buf.clone_buf()).unwrap();

        assert_eq!(
            log.trim(),
            r#"127.0.0.1 "GET https://example.com/ HTTP/1.1" 200 "test" 5"#
        )
    }

    #[test]
    fn event_log_data_deserializes_from_scalars() {
        let event_id = Uuid::now_v7();
        let parent_event_id = Uuid::now_v7();
        let json_str = format!(
            r#"{{
                "target": "my_target",
                "event_id": "{}",
                "level": "Info",
                "parent_event_id": "{}",
                "event_type": "request",
                "message": "hello world",
                "body": {{ "key": "value" }}
            }}"#,
            event_id, parent_event_id
        );

        let data: EventLogData = serde_json::from_str(&json_str).unwrap();

        assert_eq!(data.target, "my_target");
        assert_eq!(data.event_id, event_id);
        assert!(matches!(data.level, FaucetTracingLevel::Info));
        assert_eq!(data.parent_event_id, Some(parent_event_id));
        assert_eq!(data.event_type, "request");
        assert_eq!(data.message, "hello world");
        assert!(data.body.is_some());
    }

    #[test]
    fn event_log_data_deserializes_from_scalars_with_null_parent() {
        let event_id = Uuid::now_v7();
        let json_str = format!(
            r#"{{
                "target": "my_target",
                "event_id": "{}",
                "level": "Info",
                "parent_event_id": null,
                "event_type": "request",
                "message": "hello world",
                "body": null
            }}"#,
            event_id
        );

        let data: EventLogData = serde_json::from_str(&json_str).unwrap();

        assert_eq!(data.target, "my_target");
        assert_eq!(data.event_id, event_id);
        assert!(matches!(data.level, FaucetTracingLevel::Info));
        assert_eq!(data.parent_event_id, None);
        assert_eq!(data.event_type, "request");
        assert_eq!(data.message, "hello world");
        assert!(data.body.is_none());
    }

    #[test]
    fn event_log_data_deserializes_from_l1_vectors() {
        let event_id = Uuid::now_v7();
        let parent_event_id = Uuid::now_v7();
        let json_str = format!(
            r#"{{
                "target": ["my_target"],
                "event_id": ["{}"],
                "level": ["Info"],
                "parent_event_id": ["{}"],
                "event_type": ["request"],
                "message": ["hello world"],
                "body": {{ "key": "value" }}
            }}"#,
            event_id, parent_event_id
        );

        let data: EventLogData = serde_json::from_str(&json_str).unwrap();

        assert_eq!(data.target, "my_target");
        assert_eq!(data.event_id, event_id);
        assert!(matches!(data.level, FaucetTracingLevel::Info));
        assert_eq!(data.parent_event_id, Some(parent_event_id));
        assert_eq!(data.event_type, "request");
        assert_eq!(data.message, "hello world");
        assert!(data.body.is_some());
    }

    #[test]
    fn event_log_data_deserializes_from_l1_vectors_with_null_parent() {
        let event_id = Uuid::now_v7();
        let json_str = format!(
            r#"{{
                "target": ["my_target"],
                "event_id": ["{}"],
                "level": ["Info"],
                "parent_event_id": [null],
                "event_type": ["request"],
                "message": ["hello world"],
                "body": null
            }}"#,
            event_id
        );

        let data: EventLogData = serde_json::from_str(&json_str).unwrap();

        assert_eq!(data.target, "my_target");
        assert_eq!(data.event_id, event_id);
        assert!(matches!(data.level, FaucetTracingLevel::Info));
        assert_eq!(data.parent_event_id, None);
        assert_eq!(data.event_type, "request");
        assert_eq!(data.message, "hello world");
        assert!(data.body.is_none());
    }

    #[test]
    fn event_log_data_deserializes_from_mixed_scalars_and_l1_vectors() {
        let event_id = Uuid::now_v7();
        let parent_event_id = Uuid::now_v7();
        let json_str = format!(
            r#"{{
                "target": "my_target",
                "event_id": ["{}"],
                "level": "Info",
                "parent_event_id": ["{}"],
                "event_type": "request",
                "message": ["hello world"],
                "body": {{ "key": "value" }}
            }}"#,
            event_id, parent_event_id
        );

        let data: EventLogData = serde_json::from_str(&json_str).unwrap();

        assert_eq!(data.target, "my_target");
        assert_eq!(data.event_id, event_id);
        assert!(matches!(data.level, FaucetTracingLevel::Info));
        assert_eq!(data.parent_event_id, Some(parent_event_id));
        assert_eq!(data.event_type, "request");
        assert_eq!(data.message, "hello world");
        assert!(data.body.is_some());
    }
}
