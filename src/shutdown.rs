use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tokio::sync::Notify;

use crate::leak;

const WAIT_STOP_PRINT: std::time::Duration = std::time::Duration::from_secs(5);

pub struct ShutdownSignal {
    is_shutdown: AtomicBool,
    notify: Notify,
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownSignal {
    pub fn new() -> Self {
        ShutdownSignal {
            is_shutdown: AtomicBool::new(false),
            notify: Notify::new(),
        }
    }

    pub fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    pub async fn wait(&self) {
        if self.is_shutdown.load(Ordering::Relaxed) {
            return;
        }
        // Wait for notification
        self.notify.notified().await;
    }
}

pub fn graceful() -> &'static ShutdownSignal {
    use crate::global_conn::current_connections;

    let signal = leak!(ShutdownSignal::new()) as &'static ShutdownSignal;

    {
        ctrlc::set_handler(move || {
        log::info!(target: "faucet", "Received stop signal, waiting for all users to disconnect");
        let mut last_5_sec = std::time::Instant::now();
        while current_connections() > 0 {
            std::thread::yield_now();
            if last_5_sec.elapsed() > WAIT_STOP_PRINT {
                log::info!(
                    target: "faucet",
                    "Active connections = {}, waiting for all connections to stop.",
                    current_connections()
                );
                last_5_sec = std::time::Instant::now();
            }
        }
        signal.shutdown();
    })
    .expect("Unable to set term handler. This is a bug");
    }

    signal
}

pub fn immediate() -> &'static ShutdownSignal {
    let signal = leak!(ShutdownSignal::new()) as &'static ShutdownSignal;
    {
        ctrlc::set_handler(move || {
            log::info!(target: "faucet", "Starting immediate shutdown handle");
            signal.shutdown()
        })
        .expect("Unable to set term handler. This is a bug");
    }
    signal
}
