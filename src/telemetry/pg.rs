use chrono::{DateTime, Local};
use deadpool_postgres::Pool;
use std::path::Path;
use std::{io::Write, pin::pin};
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};

use crate::server::logging::EventLogData;
use crate::{
    cli::PgSslMode,
    server::{HttpLogData, LogOption},
    shutdown::ShutdownSignal,
};

pub fn make_tls(
    sslmode: PgSslMode,
    sslcert: Option<&Path>,
) -> tokio_postgres_rustls::MakeRustlsConnect {
    let mut root_store = rustls::RootCertStore::empty();

    if matches!(sslmode, PgSslMode::VerifyCa | PgSslMode::VerifyFull) {
        match sslcert {
            Some(cert_path) => {
                let mut reader =
                    std::io::BufReader::new(std::fs::File::open(cert_path).unwrap_or_else(|e| {
                        panic!("Failed to open certificate file '{:?}': {}", cert_path, e)
                    }));
                for cert in rustls_pemfile::certs(&mut reader).flatten() {
                    if let Err(e) = root_store.add(cert.clone()) {
                        log::error!("Failed to add PEM certificate: {}", e);
                    }
                }
            }
            None => panic!(
                "Specified {} but did not provide a certificate path.",
                sslmode.as_str()
            ),
        }
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    tokio_postgres_rustls::MakeRustlsConnect::new(config)
}

type PgType = tokio_postgres::types::Type;

pub fn spawn_http_events_task(
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

pub fn spawn_events_task(
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
