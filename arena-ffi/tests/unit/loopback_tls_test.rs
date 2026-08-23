use std::ffi::CStr;
use std::os::raw::c_char;

use arena_ffi::{arena_free_string, arena_oauth_loopback_tls_pem_json};

#[test]
fn arena_oauth_loopback_tls_pem_json_returns_certificate_and_key_document() {
    let mut err: *mut c_char = std::ptr::null_mut();

    let result = arena_oauth_loopback_tls_pem_json(&mut err as *mut _);

    assert!(!result.is_null());
    assert!(err.is_null());
    let json = unsafe { CStr::from_ptr(result) }.to_string_lossy().into_owned();
    arena_free_string(result);

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json document");
    assert!(parsed.get("certificate_pem").is_some());
    assert!(parsed.get("private_key_pem").is_some());
}
