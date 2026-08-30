use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use arena_ffi::{arena_close, arena_free_string, arena_oauth_sign_claims, arena_open};

fn err_text(err: *mut c_char) -> String {
    if err.is_null() {
        return String::new();
    }
    let msg = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
    arena_free_string(err);
    msg
}

fn take_string(ptr: *mut c_char) -> String {
    let s = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    arena_free_string(ptr);
    s
}

fn open_arena_with_oauth(identifier: &str) -> *mut arena_ffi::OpenArenaHandle {
    let name = CString::new("test").unwrap();
    let config_json = format!(
        r#"{{"dependencies":[{{"type":"oauth","identifier":"{identifier}","transport":"http"}}]}}"#
    );
    let config = CString::new(config_json).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _);
    assert!(!handle.is_null(), "arena_open failed: {}", err_text(err));
    handle
}

#[test]
fn arena_oauth_sign_claims_running_dependency_returns_verifiable_token() {
    let identifier = "oauth-sign-claims-ffitst";
    let arena_handle = open_arena_with_oauth(identifier);

    let identifier_c = CString::new(identifier).unwrap();
    let provider = CString::new(r#"{"provider":"custom"}"#).unwrap();
    let claims = CString::new(r#"{"sub":"test-subject","iat":0,"exp":9999999999}"#).unwrap();
    let mut sign_err: *mut c_char = std::ptr::null_mut();
    let jwt_ptr = arena_oauth_sign_claims(
        arena_handle,
        identifier_c.as_ptr(),
        provider.as_ptr(),
        claims.as_ptr(),
        &mut sign_err as *mut _,
    );
    assert!(
        !jwt_ptr.is_null(),
        "arena_oauth_sign_claims failed: {}",
        err_text(sign_err)
    );
    let jwt = take_string(jwt_ptr);
    assert_eq!(jwt.split('.').count(), 3, "expected a 3-part JWT, got {jwt}");

    arena_close(arena_handle);
}

#[test]
fn arena_oauth_sign_claims_unregistered_provider_returns_error() {
    let identifier = "oauth-sign-claims-badprv";
    let arena_handle = open_arena_with_oauth(identifier);

    let identifier_c = CString::new(identifier).unwrap();
    let provider = CString::new(r#"{"provider":"okta"}"#).unwrap();
    let claims = CString::new("{}").unwrap();
    let mut sign_err: *mut c_char = std::ptr::null_mut();
    let jwt_ptr = arena_oauth_sign_claims(
        arena_handle,
        identifier_c.as_ptr(),
        provider.as_ptr(),
        claims.as_ptr(),
        &mut sign_err as *mut _,
    );
    assert!(jwt_ptr.is_null());
    assert!(err_text(sign_err).contains("no issuer registered"));

    arena_close(arena_handle);
}
