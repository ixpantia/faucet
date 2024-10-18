const WAIT_STOP_PRINT: std::time::Duration = std::time::Duration::from_secs(5);

pub struct ShutdownSignal(tokio::sync::mpsc::Receiver<()>);

impl ShutdownSignal {
    fn new() -> (tokio::sync::mpsc::Sender<()>, Self) {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        (tx, Self(rx))
    }
    pub async fn wait(mut self) {
        let _ = self.0.recv().await;
    }
}

pub fn graceful() -> ShutdownSignal {
    use crate::global_conn::current_connections;

    let (tx, signal) = ShutdownSignal::new();

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
        let _ = tx.blocking_send(());
    })
    .expect("Unable to set term handler. This is a bug");

    signal
}

pub fn immediate() -> ShutdownSignal {
    let (tx, signal) = ShutdownSignal::new();
    ctrlc::set_handler(move || {
        log::info!(target: "faucet", "Starting immediate shutdown handle");
        let _ = tx.blocking_send(());
    })
    .expect("Unable to set term handler. This is a bug");
    signal
}
