use arena_ffi::panic_payload::panic_message;

#[test]
fn panic_message_static_str_payload_returns_str() {
    let payload: Box<dyn std::any::Any + Send> = Box::new("boom");
    assert_eq!(panic_message(&payload), "boom");
}

#[test]
fn panic_message_string_payload_returns_string() {
    let payload: Box<dyn std::any::Any + Send> = Box::new("boom".to_string());
    assert_eq!(panic_message(&payload), "boom");
}

#[test]
fn panic_message_unknown_payload_returns_fallback() {
    let payload: Box<dyn std::any::Any + Send> = Box::new(42_i32);
    assert_eq!(panic_message(&payload), "unknown panic payload");
}
