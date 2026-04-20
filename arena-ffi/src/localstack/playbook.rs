use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use arena_localstack::{ActivePlaybook, LocalstackDependency};
use serde::Deserialize;

use crate::ffi::error::{clear_error, write_error};
use crate::ffi::{ArenaHandle, ArenaStatus};
use crate::ffi::handle::HandleInner;
use crate::ffi::strings::c_str_to_string;

#[repr(C)]
pub struct ArenaLocalstackPlaybookHandle {
    _private: [u8; 0],
}

#[allow(dead_code)]
struct PlaybookInner {
    runtime_handle: tokio::runtime::Handle,
    active: Option<ActivePlaybook>,
}

impl PlaybookInner {
    fn into_raw(self) -> *mut ArenaLocalstackPlaybookHandle {
        Box::into_raw(Box::new(self)) as *mut ArenaLocalstackPlaybookHandle
    }

    unsafe fn from_raw(ptr: *mut ArenaLocalstackPlaybookHandle) -> Box<PlaybookInner> {
        unsafe { Box::from_raw(ptr as *mut PlaybookInner) }
    }
}

#[derive(Debug, Deserialize)]
struct PlaybookSpec {
    dependency_identifier: String,
}

fn with_localstack_dependency<F, R>(
    inner: &HandleInner,
    identifier: &str,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&LocalstackDependency) -> R,
{
    let guard = inner
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let arena = guard
        .as_ref()
        .ok_or_else(|| "arena is already closed".to_string())?;
    let dep = arena
        .dependency(identifier)
        .ok_or_else(|| format!("dependency '{identifier}' not found"))?;
    let localstack = dep
        .as_any()
        .downcast_ref::<LocalstackDependency>()
        .ok_or_else(|| {
            format!("dependency '{identifier}' is not a LocalstackDependency")
        })?;
    Ok(f(localstack))
}

#[no_mangle]
pub extern "C" fn arena_localstack_playbook_open(
    arena_handle: *mut ArenaHandle,
    spec: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut ArenaLocalstackPlaybookHandle {
    unsafe { clear_error(err_out) };

    if arena_handle.is_null() {
        unsafe {
            write_error(err_out, "arena_localstack_playbook_open: arena handle must not be null")
        };
        return std::ptr::null_mut();
    }
    if spec.is_null() {
        unsafe { write_error(err_out, "arena_localstack_playbook_open: spec must not be null") };
        return std::ptr::null_mut();
    }

    let spec_str = match unsafe { c_str_to_string(spec) } {
        Some(v) => v,
        None => {
            unsafe {
                write_error(err_out, "arena_localstack_playbook_open: spec is not valid UTF-8")
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
                    format!("arena_localstack_playbook_open: spec parse failed: {e}"),
                )
            };
            return std::ptr::null_mut();
        }
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<PlaybookInner, String> {
        let inner = unsafe { HandleInner::as_ref(arena_handle) };
        let runtime_handle = inner.runtime.handle().clone();

        let playbook = with_localstack_dependency(
            inner,
            &parsed.dependency_identifier,
            |localstack| localstack.playbook(),
        )?;

        let active = runtime_handle.block_on(async move { playbook.run().await });

        Ok(PlaybookInner {
            runtime_handle,
            active: Some(active),
        })
    }));

    match outcome {
        Ok(Ok(inner)) => inner.into_raw(),
        Ok(Err(msg)) => {
            log::error!("arena_localstack_playbook_open failed: {msg}");
            unsafe { write_error(err_out, format!("arena_localstack_playbook_open: {msg}")) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            log::error!("panic in arena_localstack_playbook_open: {msg}");
            unsafe {
                write_error(
                    err_out,
                    format!("panic in arena_localstack_playbook_open: {msg}"),
                )
            };
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_localstack_playbook_close(
    handle: *mut ArenaLocalstackPlaybookHandle,
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
            log::error!("arena_localstack_playbook_close: {msg}");
            unsafe { write_error(err_out, format!("arena_localstack_playbook_close: {msg}")) };
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
