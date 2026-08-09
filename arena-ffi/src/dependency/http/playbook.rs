use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use arena_http::{
    delete_requested_for, get_requested_for, post_requested_for, put_requested_for, ActivePlaybook,
    HttpDependency, RequestCriteria,
};
use serde::Deserialize;

use crate::active_playbook::{ActivePlaybookInner, ArenaActivePlaybookHandle};
use crate::closed_arena::OpenArenaRuntimeState;
use crate::dependency::http::mapping::{build_playbook_from_mappings, MappingSpec};
use crate::error::{clear_error, write_error};
use crate::panic_payload::panic_message;
use crate::strings::c_str_to_string;
use crate::{ArenaStatus, OpenArenaHandle};

#[derive(Debug, Deserialize)]
struct PlaybookSpec {
    dependency_identifier: String,
    mappings: Vec<MappingSpec>,
}

#[derive(Debug, Deserialize)]
struct VerifySpec {
    #[serde(default)]
    dependency_identifier: Option<String>,
    method: String,
    url_path: String,
    #[serde(default)]
    expected_count: Option<u64>,
    #[serde(default)]
    minimum_count: Option<u64>,
}

fn criteria_for(method: &str, url_path: &str) -> Result<RequestCriteria, String> {
    Ok(match method.to_ascii_uppercase().as_str() {
        "GET" => get_requested_for(url_path),
        "POST" => post_requested_for(url_path),
        "PUT" => put_requested_for(url_path),
        "DELETE" => delete_requested_for(url_path),
        other => return Err(format!("unsupported HTTP method: {other}")),
    })
}

fn with_http_dependency<F, R>(
    runtime_state: &OpenArenaRuntimeState,
    identifier: &str,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&HttpDependency) -> R,
{
    let guard = runtime_state.state.blocking_lock();
    let arena = guard
        .as_ref()
        .ok_or_else(|| "arena is already closed".to_string())?;
    let dep = arena
        .dependency(identifier)
        .ok_or_else(|| format!("dependency '{identifier}' not found"))?;
    let http = dep
        .as_any()
        .downcast_ref::<HttpDependency>()
        .ok_or_else(|| format!("dependency '{identifier}' is not an HttpDependency"))?;
    Ok(f(http))
}

#[no_mangle]
pub extern "C" fn arena_http_playbook_open(
    arena_handle: *mut OpenArenaHandle,
    spec: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut ArenaActivePlaybookHandle {
    unsafe { clear_error(err_out) };

    if arena_handle.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_http_playbook_open: arena handle must not be null",
            )
        };
        return std::ptr::null_mut();
    }
    if spec.is_null() {
        unsafe { write_error(err_out, "arena_http_playbook_open: spec must not be null") };
        return std::ptr::null_mut();
    }

    let spec_str = match unsafe { c_str_to_string(spec) } {
        Some(v) => v,
        None => {
            unsafe { write_error(err_out, "arena_http_playbook_open: spec is not valid UTF-8") };
            return std::ptr::null_mut();
        }
    };

    let parsed: PlaybookSpec = match catch_unwind(AssertUnwindSafe(|| serde_json::from_str(&spec_str)))
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            unsafe {
                write_error(
                    err_out,
                    format!("arena_http_playbook_open: spec parse failed: {e}"),
                )
            };
            return std::ptr::null_mut();
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            unsafe {
                write_error(err_out, format!("panic in arena_http_playbook_open: {msg}"))
            };
            return std::ptr::null_mut();
        }
    };

    if parsed.mappings.is_empty() {
        unsafe {
            write_error(
                err_out,
                "arena_http_playbook_open: mappings must not be empty",
            )
        };
        return std::ptr::null_mut();
    }

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<ActivePlaybookInner, String> {
        let arena_runtime = unsafe { OpenArenaRuntimeState::as_ref(arena_handle) };
        let runtime_handle = arena_runtime.runtime.handle().clone();

        let playbook = with_http_dependency(arena_runtime, &parsed.dependency_identifier, |http| {
            build_playbook_from_mappings(http.playbook(), &parsed.mappings)
        })??;

        let active = runtime_handle.block_on(async move { playbook.run().await });

        Ok(ActivePlaybookInner {
            runtime_handle,
            active: Some(Box::new(active)),
        })
    }));

    match outcome {
        Ok(Ok(inner)) => inner.into_raw(),
        Ok(Err(msg)) => {
            tracing::error!(error = %msg, op = "http_playbook_open", "playbook open failed");
            unsafe { write_error(err_out, format!("arena_http_playbook_open: {msg}")) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(
                panic_message = %msg,
                op = "http_playbook_open",
                "panic during playbook open"
            );
            unsafe { write_error(err_out, format!("panic in arena_http_playbook_open: {msg}")) };
            std::ptr::null_mut()
        }
    }
}

fn run_http_verify(
    handle: *mut ArenaActivePlaybookHandle,
    parsed: VerifySpec,
) -> Result<(), String> {
    let count_modes = parsed.expected_count.is_some() as u8 + parsed.minimum_count.is_some() as u8;
    if count_modes != 1 {
        return Err(
            "verify spec requires exactly one of expected_count or minimum_count".to_string(),
        );
    }

    let criteria = criteria_for(&parsed.method, &parsed.url_path)?;
    let inner = unsafe { ActivePlaybookInner::as_ref(handle) };
    let active = inner
        .active
        .as_ref()
        .ok_or_else(|| "playbook is already dropped".to_string())?;
    let http_active = active
        .as_any()
        .downcast_ref::<ActivePlaybook>()
        .ok_or_else(|| "playbook handle is not an HTTP playbook".to_string())?;

    inner.runtime_handle.block_on(async {
        if let Some(expected) = parsed.expected_count {
            http_active.verify(expected, criteria).await;
        } else if let Some(minimum) = parsed.minimum_count {
            http_active.verify_at_least(minimum, criteria).await;
        }
    });
    Ok(())
}

#[no_mangle]
pub extern "C" fn arena_http_playbook_verify(
    handle: *mut ArenaActivePlaybookHandle,
    verify_spec: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };

    if handle.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_http_playbook_verify: handle must not be null",
            )
        };
        return ArenaStatus::InvalidArgument;
    }
    if verify_spec.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_http_playbook_verify: verify_spec must not be null",
            )
        };
        return ArenaStatus::InvalidArgument;
    }

    let spec_str = match unsafe { c_str_to_string(verify_spec) } {
        Some(v) => v,
        None => {
            unsafe {
                write_error(
                    err_out,
                    "arena_http_playbook_verify: verify_spec is not valid UTF-8",
                )
            };
            return ArenaStatus::InvalidArgument;
        }
    };

    let parsed: VerifySpec = match catch_unwind(AssertUnwindSafe(|| serde_json::from_str(&spec_str)))
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            unsafe {
                write_error(
                    err_out,
                    format!("arena_http_playbook_verify: parse failed: {e}"),
                )
            };
            return ArenaStatus::InvalidArgument;
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            unsafe {
                write_error(
                    err_out,
                    format!("panic in arena_http_playbook_verify: {msg}"),
                )
            };
            return ArenaStatus::Failed;
        }
    };
    let _ = parsed.dependency_identifier;

    let outcome =
        catch_unwind(AssertUnwindSafe(|| run_http_verify(handle, parsed).map_err(|msg| msg)));

    match outcome {
        Ok(Ok(())) => ArenaStatus::Ok,
        Ok(Err(msg)) => {
            unsafe { write_error(err_out, format!("arena_http_playbook_verify: {msg}")) };
            ArenaStatus::Failed
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(error = %msg, op = "http_playbook_verify", "playbook verify failed");
            unsafe { write_error(err_out, format!("arena_http_playbook_verify: {msg}")) };
            ArenaStatus::Failed
        }
    }
}
