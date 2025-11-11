use std::{path::Path, str::FromStr, sync::OnceLock};

mod pg;

use chrono::Local;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{
    cli::PgSslMode,
    error::FaucetResult,
    leak,
    server::{logging::EventLogData, HttpLogData},
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
    pub http_events_join_handle: JoinHandle<()>,
    pub log_events_join_handle: JoinHandle<()>,
}
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
    pub fn start_postgres(
        namespace: &str,
        version: Option<&str>,
        database_url: &str,
        sslmode: PgSslMode,
        sslcert: Option<&Path>,
        shutdown_signal: &'static ShutdownSignal,
    ) -> FaucetResult<TelemetryManager> {
        log::debug!("Connecting to PostgreSQL with params: namespace='{}', version='{:?}', database_url='[REDACTED]'", namespace, version);
        let namespace = leak!(namespace) as &'static str;
        let version = version.map(|v| leak!(v) as &'static str);

        let config = tokio_postgres::Config::from_str(database_url)?;
        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(config, pg::make_tls(sslmode, sslcert), mgr_config);
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
            http_events_join_handle,
            log_events_join_handle,
        })
    }
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
        pg::spawn_events_task(event_rx, pool.clone(), namespace, version, shutdown_signal);

    let http_handle =
        pg::spawn_http_events_task(http_rx, pool, namespace, version, shutdown_signal);
    (http_tx, event_tx, http_handle, event_handle)
}
