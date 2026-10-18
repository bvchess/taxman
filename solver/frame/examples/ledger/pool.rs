//! Work shared out over threads: the boards' facts, the games of a store, the bounds that a check recomputes.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

/// `f` of every item of `items`, on `workers` threads (at least one, at most one an item), in the order of the items.
pub fn map<T: Sync, R: Send>(items: &[T], workers: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let out = Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|scope| {
        for _ in 0..workers.clamp(1, items.len().max(1)) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let result = f(item);
                out.lock().unwrap_or_else(PoisonError::into_inner).push((i, result));
            });
        }
    });
    let mut done = out.into_inner().unwrap_or_else(PoisonError::into_inner);
    done.sort_by_key(|&(i, _)| i);
    done.into_iter().map(|(_, result)| result).collect()
}
