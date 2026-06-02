use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::error::{clear_error, write_error};
use crate::error::ArenaStatus;
use crate::closed_arena::{OpenArenaHandle, OpenArenaRuntimeState};
use crate::panic_payload::panic_message;
use crate::strings::c_str_to_string;

#[derive(Clone, Copy)]
enum ResetKind {
    Soft,
    Hard,
}

#[no_mangle]
pub extern "C" fn arena_soft_reset(
    handle: *mut OpenArenaHandle,
    dependency_identifier: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    run_reset(handle, dependency_identifier, err_out, ResetKind::Soft)
}

#[no_mangle]
pub extern "C" fn arena_hard_reset(
    handle: *mut OpenArenaHandle,
    dependency_identifier: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    run_reset(handle, dependency_identifier, err_out, ResetKind::Hard)
}

fn reset_dependency(runtime_state: &OpenArenaRuntimeState, identifier: &str, kind: ResetKind) -> ArenaStatus {
    runtime_state.runtime.block_on(async {
        let mut guard = runtime_state.state.lock().await;
        let arena = match guard.as_mut() {
            Some(a) => a,
            None => return ArenaStatus::Failed,
        };
        let dep = match arena.dependency_mut(identifier) {
            Some(d) => d,
            None => return ArenaStatus::NotFound,
        };
        match kind {
            ResetKind::Soft => dep.soft_reset().await,
            ResetKind::Hard => dep.hard_reset().await,
        }
        ArenaStatus::Ok
    })
}

fn run_reset(
    handle: *mut OpenArenaHandle,
    dependency_identifier: *const c_char,
    err_out: *mut *mut c_char,
    kind: ResetKind,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };
    if handle.is_null() {
        unsafe { write_error(err_out, "reset: handle must not be null") };
        return ArenaStatus::InvalidArgument;
    }
    if dependency_identifier.is_null() {
        unsafe { write_error(err_out, "reset: dependency_identifier must not be null") };
        return ArenaStatus::InvalidArgument;
    }
    let identifier = match unsafe { c_str_to_string(dependency_identifier) } {
        Some(v) => v,
        None => {
            unsafe { write_error(err_out, "reset: dependency_identifier is not valid UTF-8") };
            return ArenaStatus::InvalidArgument;
        }
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let runtime_state = unsafe { OpenArenaRuntimeState::as_ref(handle) };
        reset_dependency(runtime_state, &identifier, kind)
    }));
    match outcome {
        Ok(status) => {
            if status == ArenaStatus::NotFound {
                unsafe { write_error(err_out, format!("dependency '{identifier}' not found")) };
            } else if status == ArenaStatus::Failed {
                unsafe { write_error(err_out, "reset: arena is already closed") };
            }
            status
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(
                panic_message = %msg,
                op = "arena_reset",
                "panic during dependency reset"
            );
            unsafe { write_error(err_out, format!("panic in arena reset: {msg}")) };
            ArenaStatus::Panic
        }
    }
}
