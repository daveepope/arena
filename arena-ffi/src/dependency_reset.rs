use std::os::raw::c_char;

use crate::error::{clear_error, write_error};
use crate::error::ArenaStatus;
use crate::closed_arena::{OpenArenaHandle, OpenArenaRuntimeState};
use crate::panic_payload::panic_message;
use crate::strings::c_str_to_string;
use crate::boundary::call_across_boundary;

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

enum ResetOutcome {
    Reset,
    Faulted(String),
    NotFound,
    Closed,
}

fn reset_dependency(
    runtime_state: &OpenArenaRuntimeState,
    identifier: &str,
    kind: ResetKind,
) -> ResetOutcome {
    runtime_state.runtime.block_on(async {
        let mut guard = runtime_state.state.lock().await;
        let arena = match guard.as_mut() {
            Some(a) => a,
            None => return ResetOutcome::Closed,
        };
        let outcome = match kind {
            ResetKind::Soft => arena.soft_reset(identifier).await,
            ResetKind::Hard => arena.hard_reset(identifier).await,
        };
        match outcome {
            None => ResetOutcome::NotFound,
            Some(Ok(())) => ResetOutcome::Reset,
            Some(Err(fault)) => {
                tracing::error!(
                    error = %fault,
                    op = "arena_reset",
                    "dependency reset failed"
                );
                ResetOutcome::Faulted(fault.to_string())
            }
        }
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
    let outcome = call_across_boundary(|| {
        let runtime_state = unsafe { OpenArenaRuntimeState::as_ref(handle) };
        reset_dependency(runtime_state, &identifier, kind)
    });
    match outcome {
        Ok(ResetOutcome::Reset) => ArenaStatus::Ok,
        Ok(ResetOutcome::NotFound) => {
            unsafe { write_error(err_out, format!("dependency '{identifier}' not found")) };
            ArenaStatus::NotFound
        }
        Ok(ResetOutcome::Closed) => {
            unsafe { write_error(err_out, "reset: arena is already closed") };
            ArenaStatus::Failed
        }
        Ok(ResetOutcome::Faulted(message)) => {
            unsafe { write_error(err_out, message) };
            ArenaStatus::Failed
        }
        Err(payload) => {
            let msg = panic_message(payload.as_ref());
            tracing::error!(
                panic_message = %msg,
                op = "arena_reset",
                "panic during dependency reset"
            );
            unsafe { write_error(err_out, format!("arena reset failed: {msg}")) };
            ArenaStatus::Panic
        }
    }
}
