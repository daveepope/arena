use std::ffi::CString;
use std::os::raw::c_char;

use arena_ffi::{arena_close, arena_oauth_sign_claims, arena_open};

#[path = "../../ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

fn open_empty_arena() -> *mut arena_ffi::OpenArenaHandle {
    let name = CString::new("test").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _);
    assert!(!handle.is_null(), "arena_open failed: {}", err_text(err));
    handle
}

#[test]
fn arena_oauth_sign_claims_unknown_identifier_returns_error() {
    let arena_handle = open_empty_arena();

    let identifier = CString::new("does-not-exist").unwrap();
    let claims = CString::new("{}").unwrap();
    let mut sign_err: *mut c_char = std::ptr::null_mut();
    let jwt_ptr = arena_oauth_sign_claims(
        arena_handle,
        identifier.as_ptr(),
        0,
        claims.as_ptr(),
        &mut sign_err as *mut _,
    );
    assert!(jwt_ptr.is_null());
    assert!(err_text(sign_err).contains("not found"));

    arena_close(arena_handle);
}

#[test]
fn arena_oauth_sign_claims_malformed_claims_json_returns_error() {
    let arena_handle = open_empty_arena();

    let identifier = CString::new("oauth").unwrap();
    let claims = CString::new("{not valid json").unwrap();
    let mut sign_err: *mut c_char = std::ptr::null_mut();
    let jwt_ptr = arena_oauth_sign_claims(
        arena_handle,
        identifier.as_ptr(),
        0,
        claims.as_ptr(),
        &mut sign_err as *mut _,
    );
    assert!(jwt_ptr.is_null());
    assert!(err_text(sign_err).contains("claims is not valid JSON"));

    arena_close(arena_handle);
}

#[test]
fn arena_open_with_misconfigured_oauth_issuers_returns_error_not_panic() {
    let name = CString::new("test").unwrap();
    let config_json = r#"{"dependencies":[{"type":"oauth","identifier":"oauth-misconfig-ffitst","transport":"http","issuers":[{"provider":"custom","issuer_path":"/a","jwks_path":"/oauth/token"}]}]}"#;
    let config = CString::new(config_json).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();

    let handle = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _);

    assert!(
        handle.is_null(),
        "a jwks_path colliding with a reserved route must fail to open, not silently succeed"
    );
    let message = err_text(err);
    assert!(
        message.contains("duplicate JWKS path"),
        "expected a clear duplicate-path error, got: {message}"
    );
}
