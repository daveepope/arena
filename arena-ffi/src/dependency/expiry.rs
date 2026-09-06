use std::time::Duration;

pub enum ExpiryOverride {
    Disabled,
    After(Duration),
}

pub fn expiry_override(expiry_seconds: Option<u64>) -> Option<ExpiryOverride> {
    match expiry_seconds {
        None => None,
        Some(0) => Some(ExpiryOverride::Disabled),
        Some(seconds) => Some(ExpiryOverride::After(Duration::from_secs(seconds))),
    }
}
