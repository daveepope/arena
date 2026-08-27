use std::ffi::CString;
use std::os::raw::c_char;

use arena_ffi::{arena_oauth_sign_claims, OpenArenaHandle};

#[path = "../../ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

#[test]
fn sign_claims_null_handle_returns_error() {
    let identifier = CString::new("oauth-dep").unwrap();
    let claims = CString::new("{}").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();

    let jwt = arena_oauth_sign_claims(
        std::ptr::null_mut(),
        identifier.as_ptr(),
        0,
        claims.as_ptr(),
        &mut err as *mut _,
    );

    assert!(jwt.is_null());
    assert!(err_text(err).contains("handle must not be null"));
}

#[test]
fn sign_claims_null_dependency_identifier_returns_error() {
    let claims = CString::new("{}").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = 0x1 as *mut OpenArenaHandle;

    let jwt = arena_oauth_sign_claims(
        handle,
        std::ptr::null(),
        0,
        claims.as_ptr(),
        &mut err as *mut _,
    );

    assert!(jwt.is_null());
    assert!(err_text(err).contains("dependency_identifier must not be null"));
}

#[test]
fn sign_claims_null_claims_json_returns_error() {
    let identifier = CString::new("oauth-dep").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = 0x1 as *mut OpenArenaHandle;

    let jwt = arena_oauth_sign_claims(
        handle,
        identifier.as_ptr(),
        0,
        std::ptr::null(),
        &mut err as *mut _,
    );

    assert!(jwt.is_null());
    assert!(err_text(err).contains("claims_json must not be null"));
}

#[test]
fn sign_claims_non_utf8_dependency_identifier_returns_error() {
    let invalid_utf8: &[u8] = &[0x66, 0x6f, 0xff, 0x00];
    let identifier_ptr = invalid_utf8.as_ptr() as *const c_char;
    let claims = CString::new("{}").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = 0x1 as *mut OpenArenaHandle;

    let jwt = arena_oauth_sign_claims(
        handle,
        identifier_ptr,
        0,
        claims.as_ptr(),
        &mut err as *mut _,
    );

    assert!(jwt.is_null());
    assert!(err_text(err).contains("dependency_identifier is not valid UTF-8"));
}

#[test]
fn sign_claims_non_utf8_claims_json_returns_error() {
    let identifier = CString::new("oauth-dep").unwrap();
    let invalid_utf8: &[u8] = &[0x66, 0x6f, 0xff, 0x00];
    let claims_ptr = invalid_utf8.as_ptr() as *const c_char;
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = 0x1 as *mut OpenArenaHandle;

    let jwt = arena_oauth_sign_claims(
        handle,
        identifier.as_ptr(),
        0,
        claims_ptr,
        &mut err as *mut _,
    );

    assert!(jwt.is_null());
    assert!(err_text(err).contains("claims_json is not valid UTF-8"));
}
