use std::{pin::pin, str::FromStr, sync::OnceLock};

use chrono::{DateTime, Local};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use std::io::Write;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::{
    error::FaucetResult,
    leak,
    server::{logging::EventLogData, HttpLogData, LogOption},
    shutdown::ShutdownSignal,
};

#[derive(Clone, Debug)]
pub struct TelemetrySender {
    pub sender_http_events: UnboundedSender<(chrono::DateTime<Local>, HttpLogData)>,
    pub sender_log_events: UnboundedSender<(chrono::DateTime<Local>, EventLogData)>,
}

impl TelemetrySender {
    pub fn send_http_event(&self, data: HttpLogData) {
        let timestamp = chrono::Local::now();
        let _ = self.sender_http_events.send((timestamp, data));
    }
    pub fn send_log_event(&self, data: EventLogData) {
        let timestamp = chrono::Local::now();
        let _ = self.sender_log_events.send((timestamp, data));
    }
}

pub struct TelemetryManager {
    _pool: deadpool_postgres::Pool,
    pub http_events_join_handle: JoinHandle<()>,
    pub log_events_join_handle: JoinHandle<()>,
}


fn make_tls() -> tokio_postgres_rustls::MakeRustlsConnect {
    use std::env;
    use std::io::Cursor;

    let sslmode = env::var("FAUCET_TELEMETRY_POSTGRES_SSLMODE").unwrap_or_else(|_| "prefer".to_string());
    let allowed_modes = ["disable", "prefer", "require", "verify-ca", "verify-full"];
    if !allowed_modes.contains(&sslmode.as_str()) {
        panic!("Invalid SSL mode '{}'. Allowed values: disable, prefer, require, verify-ca, verify-full.", sslmode);
    }
    let mut root_store = rustls::RootCertStore::empty();

    if matches!(sslmode.as_str(), "verify-ca" | "verify-full") {
        let cert_path = env::var("FAUCET_TELEMETRY_POSTGRES_SSLCERT")
            .expect("SSL mode requires FAUCET_TELEMETRY_POSTGRES_SSLCERT to be set");
        let cert_data = std::fs::read(&cert_path)
            .unwrap_or_else(|e| panic!("Failed to read certificate file '{}': {}", cert_path, e));
        let mut reader = Cursor::new(&cert_data);
        let mut added = false;
        if let Ok(certs) = rustls_pemfile::certs(&mut reader) {
            if let Some(cert) = certs.first() {
                if let Err(e) = root_store.add(cert.clone().into()) {
                    log::error!("Failed to add PEM certificate: {}", e);
                } else {
                    added = true;
                }
            }
        }
        if !added {
            if let Err(e) = root_store.add(cert_data.clone().into()) {
                panic!("Failed to add certificate to root store: {}", e);
            }
        }
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    tokio_postgres_rustls::MakeRustlsConnect::new(config)
}

type PgType = tokio_postgres::types::Type;

static TELEMETRY_SENDER: OnceLock<TelemetrySender> = OnceLock::new();

pub fn send_http_event(http_event: HttpLogData) {
    if let Some(sender) = TELEMETRY_SENDER.get() {
        sender.send_http_event(http_event);
    }
}

pub fn send_log_event(http_event: EventLogData) {
    if let Some(sender) = TELEMETRY_SENDER.get() {
        sender.send_log_event(http_event);
    }
}

impl TelemetryManager {
    pub fn start(
        namespace: &str,
        version: Option<&str>,
        database_url: &str,
        shutdown_signal: &'static ShutdownSignal,
    ) -> FaucetResult<TelemetryManager> {
        log::debug!("Connecting to PostgreSQL with params: namespace='{}', version='{:?}', database_url='[REDACTED]'", namespace, version);
        let namespace = leak!(namespace) as &'static str;
        let version = version.map(|v| leak!(v) as &'static str);

        let config = tokio_postgres::Config::from_str(database_url)?;
        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(config, make_tls(), mgr_config);
        let pool = Pool::builder(mgr).max_size(10).build()?;

        let (
            sender_http_events,
            sender_log_events,
            http_events_join_handle,
            log_events_join_handle,
        ) = handle_http_events(pool.clone(), namespace, version, shutdown_signal);

        let sender = TelemetrySender {
            sender_http_events,
            sender_log_events,
        };

        TELEMETRY_SENDER
            .set(sender)
            .expect("Unable to set telemetry sender. This is a bug! Report it!");

        Ok(TelemetryManager {
            _pool: pool,
            http_events_join_handle,
            log_events_join_handle,
        })
    }
}

fn spawn_events_task(
    mut event_rx: UnboundedReceiver<(chrono::DateTime<Local>, EventLogData)>,
    pool: Pool,
    namespace: &'static str,
    version: Option<&'static str>,
    shutdown_signal: &'static ShutdownSignal,
) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        let types = &[
            PgType::TEXT,        // Namespace
            PgType::TEXT,        // Version
            PgType::TEXT,        // Target
            PgType::TIMESTAMPTZ, // Timestamp
            PgType::UUID,        // Event_Id
            PgType::UUID,        // Parent_Event_Id
            PgType::TEXT,        // Level
            PgType::TEXT,        // Event Type
            PgType::TEXT,        // Message
            PgType::JSONB,       // Body
        ];
        let mut logs_buffer = Vec::with_capacity(100);

        'recv: loop {
            tokio::select! {
                    _ = shutdown_signal.wait() => break 'recv,
                    received = event_rx.recv_many(&mut logs_buffer, 100)  => {
                        if received == 0 {
                            break 'recv;
                        }
                    let connection = match pool.get().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            log::error!("Unable to acquire postgresql connection: {e}");
                            continue 'recv;
                        }
                    };
                    let copy_sink_res = connection
                        .copy_in::<_, bytes::Bytes>(
                            "COPY faucet_log_events FROM STDIN WITH (FORMAT binary)",
                        )
                        .await;

                    match copy_sink_res {
                        Ok(copy_sink) => {
                            let copy_in_writer =
                                tokio_postgres::binary_copy::BinaryCopyInWriter::new(copy_sink, types);

                            let mut copy_in_writer = pin!(copy_in_writer);

                            log::debug!("Writing {} log events to the database", logs_buffer.len());

                            'write: for (timestamp, event) in logs_buffer.drain(..) {
                                let target = &event.target;
                                let event_id = &event.event_id;
                                let parent_event_id = &event.parent_event_id;
                                let event_type = &event.event_type;
                                let message = &event.message;
                                let body = &event.body;
                                let level = &event.level.as_str();

                                let copy_result = copy_in_writer
                                    .as_mut()
                                    .write(&[
                                        &namespace,
                                        &version,
                                        target,
                                        &timestamp,
                                        event_id,
                                        parent_event_id,
                                        level,
                                        event_type,
                                        message,
                                        body,
                                    ])
                                    .await;

                                if let Err(e) = copy_result {
                                    log::error!("Error writing to PostgreSQL: {e}");
                                    break 'write;
                                }
                            }

                            let copy_in_finish_res = copy_in_writer.finish().await;
                            if let Err(e) = copy_in_finish_res {
                                log::error!("Error writing to PostgreSQL: {e}");
                                continue 'recv;
                            }
                        }
                        Err(e) => {
                            log::error!(target: "telemetry", "Error writing to the database: {e}")
                        }
                    }
                }
            }
        }
    })
}

fn spawn_http_events_task(
    mut http_rx: UnboundedReceiver<(DateTime<Local>, HttpLogData)>,
    pool: Pool,
    namespace: &'static str,
    version: Option<&'static str>,
    shutdown_signal: &'static ShutdownSignal,
) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        let types = &[
            PgType::UUID,        // UUID
            PgType::TEXT,        // Namespace
            PgType::TEXT,        // Version
            PgType::TEXT,        // Target
            PgType::TEXT,        // Worker Route
            PgType::INT4,        // Worker ID
            PgType::INET,        // IpAddr
            PgType::TEXT,        // Method
            PgType::TEXT,        // Path
            PgType::TEXT,        // Query Params
            PgType::TEXT,        // HTTP Version
            PgType::INT2,        // Status
            PgType::TEXT,        // User Agent
            PgType::INT8,        // Elapsed
            PgType::TIMESTAMPTZ, // TIMESTAMP
        ];
        let mut logs_buffer = Vec::with_capacity(100);
        let mut path_buffer = Vec::<u8>::new();
        let mut query_buffer = Vec::<u8>::new();
        let mut version_buffer = Vec::<u8>::new();
        let mut user_agent_buffer = Vec::<u8>::new();

        'recv: loop {
            tokio::select! {
                _ = shutdown_signal.wait() => break 'recv,
                received = http_rx.recv_many(&mut logs_buffer, 100)  => {
                    if received == 0 {
                        break 'recv;
                    }
                    let connection = match pool.get().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            log::error!("Unable to acquire postgresql connection: {e}");
                            continue 'recv;
                        }
                    };
                    let copy_sink_res = connection
                        .copy_in::<_, bytes::Bytes>(
                            "COPY faucet_http_events FROM STDIN WITH (FORMAT binary)",
                        )
                        .await;

                    match copy_sink_res {
                        Ok(copy_sink) => {
                            let copy_in_writer =
                                tokio_postgres::binary_copy::BinaryCopyInWriter::new(copy_sink, types);

                            let mut copy_in_writer = pin!(copy_in_writer);

                            log::debug!("Writing {} http events to the database", logs_buffer.len());

                            'write: for (timestamp, log_data) in logs_buffer.drain(..) {
                                let uuid = &log_data.state_data.uuid;
                                let target = &log_data.state_data.target;
                                let worker_id = log_data.state_data.worker_id as i32;
                                let worker_route = log_data.state_data.worker_route;
                                let ip = &log_data.state_data.ip;
                                let method = &log_data.method.as_str();
                                let _ = write!(path_buffer, "{}", log_data.path.path());
                                let path = &std::str::from_utf8(&path_buffer).unwrap_or_default();
                                let _ = write!(
                                    query_buffer,
                                    "{}",
                                    log_data.path.query().unwrap_or_default()
                                );
                                let query = &std::str::from_utf8(&query_buffer).unwrap_or_default();
                                let query = if query.is_empty() { None } else { Some(query) };
                                let _ = write!(version_buffer, "{:?}", log_data.version);
                                let http_version =
                                    &std::str::from_utf8(&version_buffer).unwrap_or_default();
                                let status = &log_data.status;
                                let user_agent = match &log_data.user_agent {
                                    LogOption::Some(v) => v.to_str().ok(),
                                    LogOption::None => None,
                                };

                                let elapsed = &log_data.elapsed;
                                let copy_result = copy_in_writer
                                    .as_mut()
                                    .write(&[
                                        uuid,
                                        &namespace,
                                        &version,
                                        target,
                                        &worker_route,
                                        &worker_id,
                                        ip,
                                        method,
                                        path,
                                        &query,
                                        http_version,
                                        status,
                                        &user_agent,
                                        elapsed,
                                        &timestamp,
                                    ])
                                    .await;

                                path_buffer.clear();
                                version_buffer.clear();
                                user_agent_buffer.clear();
                                query_buffer.clear();

                                if let Err(e) = copy_result {
                                    log::error!("Error writing to PostgreSQL: {e}");
                                    break 'write;
                                }
                            }

                            let copy_in_finish_res = copy_in_writer.finish().await;
                            if let Err(e) = copy_in_finish_res {
                                log::error!("Error writing to PostgreSQL: {e}");
                                continue 'recv;
                            }
                        }
                        Err(e) => {
                            log::error!(target: "telemetry", "Error writing to the database: {e}")
                        }
                    }
                }
            }
        }
    })
}

fn handle_http_events(
    pool: Pool,
    namespace: &'static str,
    version: Option<&'static str>,
    shutdown_signal: &'static ShutdownSignal,
) -> (
    UnboundedSender<(chrono::DateTime<Local>, HttpLogData)>,
    UnboundedSender<(chrono::DateTime<Local>, EventLogData)>,
    JoinHandle<()>,
    JoinHandle<()>,
) {
    let (http_tx, http_rx) = tokio::sync::mpsc::unbounded_channel::<(_, HttpLogData)>();
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<(_, EventLogData)>();

    let event_handle =
        spawn_events_task(event_rx, pool.clone(), namespace, version, shutdown_signal);

    let http_handle = spawn_http_events_task(http_rx, pool, namespace, version, shutdown_signal);
    (http_tx, event_tx, http_handle, event_handle)
}
