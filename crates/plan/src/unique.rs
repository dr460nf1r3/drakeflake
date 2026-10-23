use std::sync::atomic::{AtomicU64, Ordering};

static SERIAL: AtomicU64 = AtomicU64::new(0);

#[must_use = "the serial is used up even if dropped"]
pub fn next_serial() -> u64 {
    SERIAL.fetch_add(1, Ordering::Relaxed)
}
