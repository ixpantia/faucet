use std::sync::OnceLock;

use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};

static STOP_THREAD: OnceLock<std::thread::JoinHandle<()>> = OnceLock::new();
const WAIT_STOP_PRINT: std::time::Duration = std::time::Duration::from_secs(5);

pub fn graceful() {
    use crate::global_conn::current_connections;

    let mut signals = Signals::new([SIGTERM, SIGINT])
        .expect("Unable to initialize signals iterator, this is a bug");
    STOP_THREAD.get_or_init(|| {
        std::thread::spawn(move || {
            log::info!(target: "faucet", "Starting graceful shutdown handle");
            signals.forever().for_each(|_| {
                std::thread::spawn(|| {
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
                    std::process::exit(0);
                });
            })
        })
    });
}

pub fn immediate() {
    let mut signals = Signals::new([SIGTERM, SIGINT])
        .expect("Unable to initialize signals iterator, this is a bug");
    STOP_THREAD.get_or_init(|| {
        std::thread::spawn(move || {
            log::info!(target: "faucet", "Starting immediate shutdown handle");
            signals.forever().for_each(|_| {
                std::process::exit(0);
            })
        })
    });
}
