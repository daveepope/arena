use arena_ffi::dependency::expiry::{expiry_override, ExpiryOverride};
use std::time::Duration;

#[test]
fn expiry_override_absent_seconds_returns_none() {
    assert!(expiry_override(None).is_none());
}

#[test]
fn expiry_override_zero_seconds_returns_disabled() {
    assert!(matches!(
        expiry_override(Some(0)),
        Some(ExpiryOverride::Disabled)
    ));
}

#[test]
fn expiry_override_positive_seconds_returns_that_duration() {
    match expiry_override(Some(30)) {
        Some(ExpiryOverride::After(expiry)) => assert_eq!(expiry, Duration::from_secs(30)),
        other => panic!("expected an expiry of 30s, got {:?}", other.is_some()),
    }
}
