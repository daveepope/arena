use std::os::raw::c_char;

use arena::lifecycle::ArenaState;

use crate::boundary::call_across_boundary;
use crate::closed_arena::{OpenArenaHandle, OpenArenaRuntimeState};
use crate::error::{clear_error, clear_out_string, write_error, write_out_string, ArenaStatus};
use crate::panic_payload::panic_message;

pub(crate) fn state_json(state: &ArenaState) -> String {
    serde_json::to_string(state).unwrap_or_else(|_| String::from("{}"))
}

pub(crate) unsafe fn write_state(state_out: *mut *mut c_char, state: &ArenaState) {
    unsafe { write_out_string(state_out, state_json(state)) };
}

#[no_mangle]
pub extern "C" fn arena_state_json(
    handle: *mut OpenArenaHandle,
    err_out: *mut *mut c_char,
    state_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };
    unsafe { clear_out_string(state_out) };

    if handle.is_null() {
        unsafe { write_error(err_out, "arena_state_json: handle must not be null") };
        return ArenaStatus::InvalidArgument;
    }

    let outcome = call_across_boundary(|| {
        let runtime_state = unsafe { OpenArenaRuntimeState::as_ref(handle) };
        runtime_state
            .runtime
            .block_on(async { runtime_state.state.lock().await.as_ref().map(|a| a.state()) })
    });

    match outcome {
        Ok(Some(state)) => {
            unsafe { write_state(state_out, &state) };
            ArenaStatus::Ok
        }
        Ok(None) => {
            unsafe { write_error(err_out, "arena_state_json: arena is already closed") };
            ArenaStatus::NotFound
        }
        Err(payload) => {
            let msg = panic_message(payload.as_ref());
            tracing::error!(
                target: "arena::ffi",
                panic_message = %msg,
                op = "arena_state_json",
                "panic while reading arena state"
            );
            unsafe { write_error(err_out, format!("arena_state_json failed: {msg}")) };
            ArenaStatus::Panic
        }
    }
}
