use std::ffi::CString;
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use arena_oauth::{OauthDependency, Provider};

use crate::closed_arena::OpenArenaRuntimeState;
use crate::error::{clear_error, write_error};
use crate::panic_payload::panic_message;
use crate::strings::c_str_to_string;
use crate::OpenArenaHandle;

fn with_oauth_dependency<F, R>(
    runtime_state: &OpenArenaRuntimeState,
    identifier: &str,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&OauthDependency) -> R,
{
    let guard = runtime_state.state.blocking_lock();
    let arena = guard
        .as_ref()
        .ok_or_else(|| "arena is already closed".to_string())?;
    let dep = arena
        .dependency(identifier)
        .ok_or_else(|| format!("dependency '{identifier}' not found"))?;
    let oauth = dep
        .as_any()
        .downcast_ref::<OauthDependency>()
        .ok_or_else(|| format!("dependency '{identifier}' is not an OauthDependency"))?;
    Ok(f(oauth))
}

fn sign_claims(
    runtime_state: &OpenArenaRuntimeState,
    identifier: &str,
    provider_json: &str,
    claims_json: &str,
) -> Result<String, String> {
    let provider: Provider = serde_json::from_str(provider_json)
        .map_err(|e| format!("arena_oauth_sign_claims: provider is not valid JSON: {e}"))?;
    let claims: serde_json::Value = serde_json::from_str(claims_json)
        .map_err(|e| format!("arena_oauth_sign_claims: claims is not valid JSON: {e}"))?;
    with_oauth_dependency(runtime_state, identifier, |oauth| {
        oauth.sign_claims(&provider, &claims)
    })?
}

#[no_mangle]
pub extern "C" fn arena_oauth_sign_claims(
    handle: *mut OpenArenaHandle,
    dependency_identifier: *const c_char,
    provider_json: *const c_char,
    claims_json: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut c_char {
    unsafe { clear_error(err_out) };

    if handle.is_null() {
        unsafe { write_error(err_out, "arena_oauth_sign_claims: handle must not be null") };
        return std::ptr::null_mut();
    }
    if dependency_identifier.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_oauth_sign_claims: dependency_identifier must not be null",
            )
        };
        return std::ptr::null_mut();
    }
    if provider_json.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_oauth_sign_claims: provider_json must not be null",
            )
        };
        return std::ptr::null_mut();
    }
    if claims_json.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_oauth_sign_claims: claims_json must not be null",
            )
        };
        return std::ptr::null_mut();
    }

    let identifier = match unsafe { c_str_to_string(dependency_identifier) } {
        Some(v) => v,
        None => {
            unsafe {
                write_error(
                    err_out,
                    "arena_oauth_sign_claims: dependency_identifier is not valid UTF-8",
                )
            };
            return std::ptr::null_mut();
        }
    };
    let provider_str = match unsafe { c_str_to_string(provider_json) } {
        Some(v) => v,
        None => {
            unsafe {
                write_error(
                    err_out,
                    "arena_oauth_sign_claims: provider_json is not valid UTF-8",
                )
            };
            return std::ptr::null_mut();
        }
    };
    let claims_str = match unsafe { c_str_to_string(claims_json) } {
        Some(v) => v,
        None => {
            unsafe {
                write_error(
                    err_out,
                    "arena_oauth_sign_claims: claims_json is not valid UTF-8",
                )
            };
            return std::ptr::null_mut();
        }
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let runtime_state = unsafe { OpenArenaRuntimeState::as_ref(handle) };
        sign_claims(runtime_state, &identifier, &provider_str, &claims_str)
    }));

    match outcome {
        Ok(Ok(jwt)) => match CString::new(jwt) {
            Ok(c) => c.into_raw(),
            Err(_) => {
                unsafe { write_error(err_out, "arena_oauth_sign_claims: token contained interior NUL") };
                std::ptr::null_mut()
            }
        },
        Ok(Err(msg)) => {
            unsafe { write_error(err_out, format!("arena_oauth_sign_claims: {msg}")) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(
                panic_message = %msg,
                op = "arena_oauth_sign_claims",
                "panic during oauth sign_claims"
            );
            unsafe { write_error(err_out, format!("panic in arena_oauth_sign_claims: {msg}")) };
            std::ptr::null_mut()
        }
    }
}
