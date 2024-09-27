use std::sync::{atomic::AtomicI64, OnceLock};

pub static CORRENT_CONNECTIONS: OnceLock<AtomicI64> = OnceLock::new();

pub fn add_connection() {
    CORRENT_CONNECTIONS
        .get_or_init(|| AtomicI64::new(0))
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

pub fn remove_connection() {
    CORRENT_CONNECTIONS
        .get_or_init(|| unreachable!())
        .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
}

pub fn current_connections() -> i64 {
    CORRENT_CONNECTIONS
        .get_or_init(|| AtomicI64::new(0))
        .load(std::sync::atomic::Ordering::SeqCst)
}
