use std::{pin::pin, str::FromStr};

use chrono::Local;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use std::io::Write;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{
    error::FaucetResult,
    server::{LogData, LogOption},
};

#[derive(Clone)]
pub struct TelemetrySender {
    pub sender_http_events: UnboundedSender<(chrono::DateTime<Local>, LogData)>,
}

impl TelemetrySender {
    pub fn send_http_event(&self, data: LogData) {
        let timestamp = chrono::Local::now();
        let _ = self.sender_http_events.send((timestamp, data));
    }
}

pub struct TelemetryManager {
    _pool: deadpool_postgres::Pool,
    pub sender: TelemetrySender,
    pub http_events_join_handle: JoinHandle<()>,
}

fn make_tls() -> tokio_postgres_rustls::MakeRustlsConnect {
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();

    tokio_postgres_rustls::MakeRustlsConnect::new(config)
}

type PgType = tokio_postgres::types::Type;

impl TelemetryManager {
    pub fn start(
        namespace: &str,
        version: Option<&str>,
        database_url: &str,
    ) -> FaucetResult<TelemetryManager> {
        let config = tokio_postgres::Config::from_str(database_url)?;
        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(config, make_tls(), mgr_config);
        let pool = Pool::builder(mgr).max_size(10).build()?;

        let (sender_http_events, http_events_join_handle) =
            handle_http_events(pool.clone(), namespace, version);

        Ok(TelemetryManager {
            _pool: pool,
            http_events_join_handle,
            sender: TelemetrySender { sender_http_events },
        })
    }
}

fn handle_http_events(
    pool: Pool,
    namespace: &str,
    version: Option<&str>,
) -> (
    UnboundedSender<(chrono::DateTime<Local>, LogData)>,
    JoinHandle<()>,
) {
    let namespace = Box::<str>::from(namespace);
    let version = version.map(Box::<str>::from);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(_, LogData)>();
    let handle = tokio::task::spawn(async move {
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

        'recv: while rx.recv_many(&mut logs_buffer, 100).await > 0 {
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
                    let mut copy_in_writer =
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
    });
    (tx, handle)
}
