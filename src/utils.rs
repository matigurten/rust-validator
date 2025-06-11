pub fn now_nanos() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    now.as_secs() as u128 * 1_000_000_000 + now.subsec_nanos() as u128
} 