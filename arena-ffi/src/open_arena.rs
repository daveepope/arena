use std::os::raw::c_char;

use crate::arena_state::write_state;
use crate::boundary::call_across_boundary;
use crate::closed_arena::{OpenArenaHandle, OpenArenaRuntimeState};
use crate::error::{clear_error, clear_out_string, write_error, ArenaStatus};
use crate::logging;
use crate::panic_payload::panic_message;

#[no_mangle]
pub extern "C" fn arena_close(
    handle: *mut OpenArenaHandle,
    err_out: *mut *mut c_char,
    state_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };
    unsafe { clear_out_string(state_out) };

    if handle.is_null() {
        unsafe { write_error(err_out, "arena_close: handle must not be null") };
        return ArenaStatus::InvalidArgument;
    }

    let outcome = call_across_boundary(|| {
        let runtime_state = unsafe { OpenArenaRuntimeState::from_raw(handle) };
        runtime_state.runtime.block_on(async {
            let arena = runtime_state.state.lock().await.take();
            let Some(arena) = arena else {
                tracing::warn!(
                    target: "arena::ffi",
                    phase = "arena_close_skip",
                    "arena close skipped; arena slot empty (stale or duplicate close handle?)"
                );
                return None;
            };
            match arena.close().await {
                Ok(closed) => Some(Ok(closed.state())),
                Err(state) => {
                    tracing::error!(
                        target: "arena::ffi",
                        arena_state = %state,
                        phase = "arena_close_faulted",
                        "arena close faulted"
                    );
                    Some(Err(state))
                }
            }
        })
    });
    logging::dispatcher_allowlists_reset();

    match outcome {
        Ok(Some(Ok(state))) => {
            unsafe { write_state(state_out, &state) };
            ArenaStatus::Ok
        }
        Ok(Some(Err(state))) => {
            unsafe { write_error(err_out, state.to_string()) };
            unsafe { write_state(state_out, &state) };
            ArenaStatus::Failed
        }
        Ok(None) => {
            unsafe { write_error(err_out, "arena_close: arena is already closed") };
            ArenaStatus::NotFound
        }
        Err(payload) => {
            let msg = panic_message(payload.as_ref());
            tracing::error!(
                panic_message = %msg,
                op = "arena_close",
                "panic while closing arena"
            );
            unsafe { write_error(err_out, format!("arena_close failed: {msg}")) };
            ArenaStatus::Panic
        }
    }
}
