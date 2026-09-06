use arena_container::expiry::{
    expiry_labels, expiry_labels_for, is_expired, now_millis, DEFAULT_EXPIRY, EXPIRES_AT_LABEL,
    MODULE_LABEL, SWEEP_INTERVAL,
};
use std::time::Duration;

#[test]
fn default_expiry_no_override_is_five_minutes() {
    assert_eq!(DEFAULT_EXPIRY, Duration::from_secs(300));
}

#[test]
fn expiry_labels_module_and_duration_returns_module_and_future_deadline() {
    let labels = expiry_labels("arena-postgres", Duration::from_secs(300));

    let module = labels.iter().find(|(k, _)| k == MODULE_LABEL).unwrap();
    assert_eq!(module.1, "arena-postgres");

    let deadline: u128 = labels
        .iter()
        .find(|(k, _)| k == EXPIRES_AT_LABEL)
        .unwrap()
        .1
        .parse()
        .unwrap();
    assert!(deadline > now_millis());
}

#[test]
fn is_expired_deadline_in_the_past_returns_true() {
    assert!(is_expired(Some(&"1000".to_string()), 2000));
}

#[test]
fn is_expired_deadline_in_the_future_returns_false() {
    assert!(!is_expired(Some(&"5000".to_string()), 2000));
}

#[test]
fn is_expired_missing_or_unparsable_deadline_returns_false() {
    assert!(!is_expired(None, 2000));
    assert!(!is_expired(Some(&"not-a-number".to_string()), 2000));
}

#[test]
fn is_expired_deadline_equal_to_now_returns_true() {
    assert!(is_expired(Some(&"2000".to_string()), 2000));
}

#[test]
fn expiry_labels_for_disabled_expiry_returns_no_labels() {
    assert!(expiry_labels_for("arena-postgres", None).is_empty());
}

#[test]
fn expiry_labels_for_enabled_expiry_returns_both_labels() {
    let labels = expiry_labels_for("arena-postgres", Some(Duration::from_secs(60)));
    assert_eq!(labels.len(), 2);
}

#[test]
fn expiry_labels_for_zero_expiry_returns_no_labels() {
    assert!(expiry_labels_for("arena-postgres", Some(Duration::ZERO)).is_empty());
}

#[test]
fn sweep_interval_is_sixty_seconds() {
    assert_eq!(SWEEP_INTERVAL, Duration::from_secs(60));
}
