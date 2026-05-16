use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use arena_mssql::{ActivePlaybook, MssqlDependency};
use serde::Deserialize;

use crate::error::{clear_error, write_error};
use crate::closed_arena::OpenArenaRuntimeState;
use crate::strings::c_str_to_string;
use crate::{ArenaStatus, OpenArenaHandle};

#[repr(C)]
pub struct ArenaMssqlPlaybookHandle {
    _private: [u8; 0],
}

struct PlaybookInner {
    runtime_handle: tokio::runtime::Handle,
    active: Option<ActivePlaybook>,
}

impl PlaybookInner {
    fn into_raw(self) -> *mut ArenaMssqlPlaybookHandle {
        Box::into_raw(Box::new(self)) as *mut ArenaMssqlPlaybookHandle
    }

    unsafe fn from_raw(ptr: *mut ArenaMssqlPlaybookHandle) -> Box<PlaybookInner> {
        unsafe { Box::from_raw(ptr as *mut PlaybookInner) }
    }

    unsafe fn as_ref<'a>(ptr: *mut ArenaMssqlPlaybookHandle) -> &'a PlaybookInner {
        unsafe { &*(ptr as *const PlaybookInner) }
    }
}

#[derive(Debug, Deserialize)]
struct PlaybookSpec {
    dependency_identifier: String,
}

#[derive(Debug, Deserialize)]
struct VerifySpec {
    #[serde(default)]
    dependency_identifier: Option<String>,
    query: String,
    expected_value: i32,
}

fn with_mssql_dependency<F, R>(
    runtime_state: &OpenArenaRuntimeState,
    identifier: &str,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&MssqlDependency) -> R,
{
    let guard = runtime_state
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let arena = guard
        .as_ref()
        .ok_or_else(|| "arena is already closed".to_string())?;
    let dep = arena
        .dependency(identifier)
        .ok_or_else(|| format!("dependency '{identifier}' not found"))?;
    let mssql = dep
        .as_any()
        .downcast_ref::<MssqlDependency>()
        .ok_or_else(|| format!("dependency '{identifier}' is not an MssqlDependency"))?;
    Ok(f(mssql))
}

#[no_mangle]
pub extern "C" fn arena_mssql_playbook_open(
    arena_handle: *mut OpenArenaHandle,
    spec: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut ArenaMssqlPlaybookHandle {
    unsafe { clear_error(err_out) };

    if arena_handle.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_mssql_playbook_open: arena handle must not be null",
            )
        };
        return std::ptr::null_mut();
    }
    if spec.is_null() {
        unsafe { write_error(err_out, "arena_mssql_playbook_open: spec must not be null") };
        return std::ptr::null_mut();
    }

    let spec_str = match unsafe { c_str_to_string(spec) } {
        Some(v) => v,
        None => {
            unsafe {
                write_error(
                    err_out,
                    "arena_mssql_playbook_open: spec is not valid UTF-8",
                )
            };
            return std::ptr::null_mut();
        }
    };

    let parsed: PlaybookSpec = match serde_json::from_str(&spec_str) {
        Ok(v) => v,
        Err(e) => {
            unsafe {
                write_error(
                    err_out,
                    format!("arena_mssql_playbook_open: spec parse failed: {e}"),
                )
            };
            return std::ptr::null_mut();
        }
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<PlaybookInner, String> {
        let arena_runtime = unsafe { OpenArenaRuntimeState::as_ref(arena_handle) };
        let runtime_handle = arena_runtime.runtime.handle().clone();

        let playbook = with_mssql_dependency(arena_runtime, &parsed.dependency_identifier, |mssql| {
            mssql.playbook()
        })?;

        let active = runtime_handle.block_on(async move { playbook.run().await });

        Ok(PlaybookInner {
            runtime_handle,
            active: Some(active),
        })
    }));

    match outcome {
        Ok(Ok(inner)) => inner.into_raw(),
        Ok(Err(msg)) => {
            tracing::error!(error = %msg, op = "mssql_playbook_open", "playbook open failed");
            unsafe { write_error(err_out, format!("arena_mssql_playbook_open: {msg}")) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(
                panic_message = %msg,
                op = "mssql_playbook_open",
                "panic during playbook open"
            );
            unsafe {
                write_error(
                    err_out,
                    format!("panic in arena_mssql_playbook_open: {msg}"),
                )
            };
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_mssql_playbook_close(
    handle: *mut ArenaMssqlPlaybookHandle,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };
    if handle.is_null() {
        return ArenaStatus::Ok;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _dropped = unsafe { PlaybookInner::from_raw(handle) };
    }));
    match outcome {
        Ok(()) => ArenaStatus::Ok,
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(error = %msg, op = "mssql_playbook_close", "playbook close failed");
            unsafe { write_error(err_out, format!("arena_mssql_playbook_close: {msg}")) };
            ArenaStatus::Failed
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_mssql_playbook_verify(
    handle: *mut ArenaMssqlPlaybookHandle,
    verify_spec: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };

    if handle.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_mssql_playbook_verify: handle must not be null",
            )
        };
        return ArenaStatus::InvalidArgument;
    }
    if verify_spec.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_mssql_playbook_verify: verify_spec must not be null",
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
                    "arena_mssql_playbook_verify: verify_spec is not valid UTF-8",
                )
            };
            return ArenaStatus::InvalidArgument;
        }
    };

    let parsed: VerifySpec = match serde_json::from_str(&spec_str) {
        Ok(v) => v,
        Err(e) => {
            unsafe {
                write_error(
                    err_out,
                    format!("arena_mssql_playbook_verify: parse failed: {e}"),
                )
            };
            return ArenaStatus::InvalidArgument;
        }
    };
    let _ = parsed.dependency_identifier;

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        let inner = unsafe { PlaybookInner::as_ref(handle) };
        let active = inner
            .active
            .as_ref()
            .ok_or_else(|| "playbook is already closed".to_string())?;

        let actual = inner
            .runtime_handle
            .block_on(async { active.verify(&parsed.query).await });

        if actual != parsed.expected_value {
            return Err(format!(
                "verify failed for query {:?}: expected {}, got {}",
                parsed.query, parsed.expected_value, actual
            ));
        }
        Ok(())
    }));

    match outcome {
        Ok(Ok(())) => ArenaStatus::Ok,
        Ok(Err(msg)) => {
            unsafe { write_error(err_out, format!("arena_mssql_playbook_verify: {msg}")) };
            ArenaStatus::Failed
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(error = %msg, op = "mssql_playbook_verify", "playbook verify failed");
            unsafe { write_error(err_out, format!("arena_mssql_playbook_verify: {msg}")) };
            ArenaStatus::Failed
        }
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}
