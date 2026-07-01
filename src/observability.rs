use serde::Serialize;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static CORRELATION_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn new_correlation_id() -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let seq = CORRELATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", now_ms, seq)
}

pub fn args_hash<T: Serialize>(value: &T) -> String {
    match serde_json::to_vec(value) {
        Ok(bytes) => {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        }
        Err(_) => "unavailable".to_string(),
    }
}
