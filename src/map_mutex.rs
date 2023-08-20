use std::collections::HashMap;
use std::marker::Send;
use std::sync::Arc;
use tokio::sync::Mutex;

/// The state of a lock is either locked or open.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum State {
    Locked,
    Open,
}

#[derive(Copy, Clone, Debug)]
struct Lock<K>
where
    K: Eq + std::hash::Hash + Copy + Clone + Send + 'static,
{
    state: State,
    key: K,
}

impl<K> Lock<K>
where
    K: Eq + std::hash::Hash + Copy + Clone + Send + 'static,
{
    /// Create a new lock.
    fn new(key: K) -> Self {
        Self {
            state: State::Open,
            key,
        }
    }
    /// Get the state of the lock.
    fn state(&self) -> &State {
        &self.state
    }
}

/// A collection of locks.
struct Locks<K>
where
    K: Eq + std::hash::Hash + Copy + Clone + Send + 'static,
{
    locks: HashMap<K, Lock<K>>,
}

impl<K> Locks<K>
where
    K: Eq + std::hash::Hash + Copy + Clone + Send + 'static,
{
    fn new() -> Self {
        Self {
            locks: HashMap::new(),
        }
    }
    /// Tries to acquire a lock. If the lock is already locked, then `None` is
    /// returned. Otherwise, the lock is acquired and a `LockGuard` is returned.
    ///
    /// The lock is released when the method `release` is called on the
    /// `LockGuard`.
    fn try_acquire(&mut self, key: K) -> Option<Lock<K>> {
        let lock = self.locks.entry(key).or_insert_with(|| Lock::new(key));
        match lock.state() {
            State::Locked => None,
            State::Open => {
                lock.state = State::Locked;
                Some(*lock)
            }
        }
    }
    /// Release a lock.
    fn release(&mut self, lock: Lock<K>) {
        let lock = self.locks.get_mut(&lock.key).expect("lock is valid");
        lock.state = State::Open;
    }
}

/// A lock guard is returned when a lock is acquired. The lock is released when
/// the `release` method is called.
pub struct LockGuard<K>
where
    K: Eq + std::hash::Hash + Copy + Clone + Send + 'static,
{
    lock: Lock<K>,
    locks: Arc<Mutex<Locks<K>>>,
}

impl<K> LockGuard<K>
where
    K: Eq + std::hash::Hash + Copy + Clone + Send + 'static,
{
    /// Get the key of the lock.
    pub fn key(&self) -> K {
        self.lock.key
    }
    /// Release the lock.
    pub async fn release(self) {
        let mut locks = self.locks.lock().await;
        locks.release(self.lock);
    }
}

/// A dispatcher is used to acquire locks. It is thread safe and can be shared
/// between threads.
pub struct Dispatcher<K>
where
    K: Eq + std::hash::Hash + Copy + Clone + Send + 'static,
{
    locks: Arc<Mutex<Locks<K>>>,
}

impl<K> Dispatcher<K>
where
    K: Eq + std::hash::Hash + Copy + Clone + Send + 'static,
{
    pub fn new() -> Self {
        Self {
            locks: Arc::new(Mutex::new(Locks::new())),
        }
    }
    /// Try to acquire a lock. If the lock is already locked, then `None` is
    /// returned. Otherwise, the lock is acquired and a `LockGuard` is returned.
    ///
    /// The lock is released when the method `release` is called on the
    /// `LockGuard`.
    pub async fn try_acquire(&self, key: K) -> Option<LockGuard<K>> {
        let mut locks = self.locks.lock().await;
        locks.try_acquire(key).map(|lock| LockGuard {
            locks: self.locks.clone(),
            lock,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    /// Test that once a lock is acquired it cannot be acquired again until it
    /// is released.
    async fn test_locking() {
        let dispatcher = Dispatcher::new();
        {
            let _lock = dispatcher.try_acquire(1).await.expect("lock is valid");
            let lock = dispatcher.try_acquire(1).await;
            assert!(lock.is_none());
            _lock.release().await;
        }
        {
            let _lock = dispatcher.try_acquire(1).await.expect("lock is valid");
            let lock = dispatcher.try_acquire(1).await;
            assert!(lock.is_none());
            _lock.release().await;
        }
        {
            let _lock = dispatcher.try_acquire(1).await.expect("lock is valid");
            let lock = dispatcher.try_acquire(1).await;
            assert!(lock.is_none());
            _lock.release().await;
        }
    }

    #[tokio::test]
    /// Test manual release of a lock.
    async fn test_release() {
        let dispatcher = Dispatcher::new();
        {
            let lock = dispatcher.try_acquire(1).await.expect("lock is valid");
            lock.release().await;
            let lock = dispatcher.try_acquire(1).await.expect("lock is valid");
            lock.release().await;
        }
    }
}
