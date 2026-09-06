
use crate::closed_arena::{OpenArenaHandle, OpenArenaRuntimeState};
use crate::logging;
use crate::panic_payload::panic_message;
use crate::boundary::call_across_boundary;

#[no_mangle]
pub extern "C" fn arena_close(handle: *mut OpenArenaHandle) {
    if handle.is_null() {
        return;
    }
    let outcome = call_across_boundary(|| {
        let runtime_state = unsafe { OpenArenaRuntimeState::from_raw(handle) };
        runtime_state.runtime.block_on(async {
            let arena = runtime_state.state.lock().await.take();
            if let Some(arena) = arena {
                if let Err(state) = arena.close().await {
                    tracing::error!(
                        target: "arena::ffi",
                        arena_state = %state,
                        phase = "arena_close_faulted",
                        "arena close faulted"
                    );
                }
            } else {
                tracing::warn!(
                    target: "arena::ffi",
                    phase = "arena_close_skip",
                    "arena close skipped; arena slot empty (stale or duplicate close handle?)"
                );
            }
        });
    });
    logging::dispatcher_allowlists_reset();
    if let Err(payload) = outcome {
        tracing::error!(
            panic_message = %panic_message(payload.as_ref()),
            op = "arena_close",
            "panic while closing arena"
        );
    }
}
