use std::os::raw::c_char;

use arena::{ClosedArena, OpenArena};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

use crate::matches::build_match_async;

use crate::error::{clear_error, write_error};
use crate::logging::init_logging;
use crate::panic_payload::panic_message;
use crate::strings::c_str_to_string;
use crate::boundary::call_across_boundary;

#[repr(C)]
pub struct OpenArenaHandle {
    _private: [u8; 0],
}

pub(crate) struct OpenArenaRuntimeState {
    pub runtime: Runtime,
    pub state: Mutex<Option<OpenArena>>,
}

impl OpenArenaRuntimeState {
    pub fn new(runtime: Runtime, arena: OpenArena) -> Self {
        Self {
            runtime,
            state: Mutex::new(Some(arena)),
        }
    }

    pub fn into_raw(self) -> *mut OpenArenaHandle {
        Box::into_raw(Box::new(self)) as *mut OpenArenaHandle
    }

    pub unsafe fn from_raw(ptr: *mut OpenArenaHandle) -> Box<OpenArenaRuntimeState> {
        unsafe { Box::from_raw(ptr as *mut OpenArenaRuntimeState) }
    }

    pub unsafe fn as_ref<'a>(ptr: *mut OpenArenaHandle) -> &'a OpenArenaRuntimeState {
        unsafe { &*(ptr as *const OpenArenaRuntimeState) }
    }
}

#[no_mangle]
pub extern "C" fn arena_open(
    name: *const c_char,
    config: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut OpenArenaHandle {
    init_logging();
    unsafe { clear_error(err_out) };

    if name.is_null() {
        unsafe { write_error(err_out, "arena_open: name must not be null") };
        return std::ptr::null_mut();
    }
    let name_str = match unsafe { c_str_to_string(name) } {
        Some(v) => v,
        None => {
            unsafe { write_error(err_out, "arena_open: name is not valid UTF-8") };
            return std::ptr::null_mut();
        }
    };
    let parsed = match call_across_boundary(|| unsafe { parse_config(config) }) {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            unsafe { write_error(err_out, format!("arena_open: {e}")) };
            return std::ptr::null_mut();
        }
        Err(payload) => {
            let msg = panic_message(payload.as_ref());
            unsafe { write_error(err_out, format!("arena_open failed: {msg}")) };
            return std::ptr::null_mut();
        }
    };

    let outcome = call_across_boundary(|| -> Result<OpenArenaRuntimeState, String> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("failed to create tokio runtime: {e}"))?;
        let arena = runtime.block_on(open_arena_from_config(name_str, parsed))?;
        Ok(OpenArenaRuntimeState::new(runtime, arena))
    });

    match outcome {
        Ok(Ok(runtime_state)) => runtime_state.into_raw(),
        Ok(Err(msg)) => {
            tracing::error!(
                error = %msg,
                op = "arena_open",
                "open failed"
            );
            unsafe { write_error(err_out, msg) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(payload.as_ref());
            tracing::error!(
                panic_message = %msg,
                op = "arena_open",
                "panic while opening arena"
            );
            unsafe { write_error(err_out, format!("arena_open failed: {msg}")) };
            std::ptr::null_mut()
        }
    }
}

async fn open_arena_from_config(
    name: String,
    parsed: crate::matches::MatchConfig,
) -> Result<OpenArena, String> {
    let a_match = build_match_async(&parsed).await?;
    let closed = ClosedArena::new(name, vec![a_match]);
    closed.open().await.map_err(|state| state.to_string())
}

unsafe fn parse_config(ptr: *const c_char) -> Result<crate::matches::MatchConfig, String> {
    if ptr.is_null() {
        return Ok(crate::matches::MatchConfig::default());
    }
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|e| format!("config is not valid UTF-8: {e}"))?;
    if s.is_empty() {
        return Ok(crate::matches::MatchConfig::default());
    }
    serde_json::from_str(s).map_err(|e| format!("config failed to parse: {e}"))
}

/// Bench-only entry point exercising the same `MatchConfig` deserialization
/// path as `parse_config`, without the FFI/raw-pointer plumbing around it.
/// Not part of the real FFI surface; only compiled with `bench-support`.
#[cfg(feature = "bench-support")]
pub fn parse_config_for_bench(json: &str) -> Result<(), String> {
    serde_json::from_str::<crate::matches::MatchConfig>(json)
        .map(|_| ())
        .map_err(|e| format!("config failed to parse: {e}"))
}
