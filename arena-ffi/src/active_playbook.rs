use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::closed_arena::OpenArenaRuntimeState;
use crate::error::{clear_error, write_error};
use crate::panic_payload::panic_message;
use crate::strings::c_str_to_string;
use crate::{ArenaStatus, OpenArenaHandle};

#[repr(C)]
pub struct ArenaActivePlaybookHandle {
    _private: [u8; 0],
}

pub(crate) struct ActivePlaybookInner {
    pub runtime_handle: tokio::runtime::Handle,
    pub active: Option<Box<dyn arena::ActivePlaybook>>,
}

impl ActivePlaybookInner {
    pub fn into_raw(self) -> *mut ArenaActivePlaybookHandle {
        Box::into_raw(Box::new(self)) as *mut ArenaActivePlaybookHandle
    }

    pub unsafe fn from_raw(ptr: *mut ArenaActivePlaybookHandle) -> Box<ActivePlaybookInner> {
        unsafe { Box::from_raw(ptr as *mut ActivePlaybookInner) }
    }

    pub unsafe fn as_ref<'a>(ptr: *mut ArenaActivePlaybookHandle) -> &'a ActivePlaybookInner {
        unsafe { &*(ptr as *const ActivePlaybookInner) }
    }
}

#[no_mangle]
pub extern "C" fn arena_match_playbook_run(
    arena_handle: *mut OpenArenaHandle,
    identifier: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut ArenaActivePlaybookHandle {
    unsafe { clear_error(err_out) };

    if arena_handle.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_match_playbook_run: arena handle must not be null",
            )
        };
        return std::ptr::null_mut();
    }
    if identifier.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_match_playbook_run: identifier must not be null",
            )
        };
        return std::ptr::null_mut();
    }

    let id_str = match unsafe { c_str_to_string(identifier) } {
        Some(v) => v,
        None => {
            unsafe {
                write_error(
                    err_out,
                    "arena_match_playbook_run: identifier is not valid UTF-8",
                )
            };
            return std::ptr::null_mut();
        }
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<ActivePlaybookInner, String> {
        let arena_runtime = unsafe { OpenArenaRuntimeState::as_ref(arena_handle) };
        let runtime_handle = arena_runtime.runtime.handle().clone();

        let guard = arena_runtime
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let arena = guard
            .as_ref()
            .ok_or_else(|| "arena is already closed".to_string())?;

        let active = runtime_handle
            .block_on(arena.run_playbook(&id_str))
            .ok_or_else(|| format!("playbook '{id_str}' is not registered on any match"))?;

        Ok(ActivePlaybookInner {
            runtime_handle,
            active: Some(active),
        })
    }));

    match outcome {
        Ok(Ok(inner)) => inner.into_raw(),
        Ok(Err(msg)) => {
            tracing::error!(error = %msg, op = "match_playbook_run", "playbook run failed");
            unsafe { write_error(err_out, format!("arena_match_playbook_run: {msg}")) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(
                panic_message = %msg,
                op = "match_playbook_run",
                "panic during playbook run"
            );
            unsafe { write_error(err_out, format!("panic in arena_match_playbook_run: {msg}")) };
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_active_playbook_drop(
    handle: *mut ArenaActivePlaybookHandle,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };
    if handle.is_null() {
        return ArenaStatus::Ok;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _dropped = unsafe { ActivePlaybookInner::from_raw(handle) };
    }));
    match outcome {
        Ok(()) => ArenaStatus::Ok,
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(error = %msg, op = "active_playbook_drop", "playbook drop failed");
            unsafe { write_error(err_out, format!("arena_active_playbook_drop: {msg}")) };
            ArenaStatus::Failed
        }
    }
}
