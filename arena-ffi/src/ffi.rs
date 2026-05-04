pub mod error;
pub(crate) mod handle;
mod logging;
pub(crate) mod strings;

mod loopback_tls;

use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use arena::ClosedArena;

use crate::matches::build_match_async;

pub use error::ArenaStatus;
pub use handle::ArenaHandle;
pub use strings::arena_free_string;

use error::{clear_error, write_error};
use handle::HandleInner;
use logging::init_logging;
use strings::c_str_to_string;

#[no_mangle]
pub extern "C" fn arena_open(
    name: *const c_char,
    config: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut ArenaHandle {
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
    let parsed = match unsafe { parse_config(config) } {
        Ok(c) => c,
        Err(e) => {
            unsafe { write_error(err_out, format!("arena_open: {e}")) };
            return std::ptr::null_mut();
        }
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<HandleInner, String> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("failed to create tokio runtime: {e}"))?;
        let arena = runtime.block_on(async {
            let a_match = build_match_async(&parsed).await?;
            let closed = ClosedArena::new(name_str, vec![a_match]);
            Ok::<_, String>(closed.open().await)
        })?;
        Ok(HandleInner::new(runtime, arena))
    }));

    match outcome {
        Ok(Ok(inner)) => inner.into_raw(),
        Ok(Err(msg)) => {
            log::error!("arena_open failed: {msg}");
            unsafe { write_error(err_out, msg) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            log::error!("panic in arena_open: {msg}");
            unsafe { write_error(err_out, format!("panic in arena_open: {msg}")) };
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_close(handle: *mut ArenaHandle) {
    if handle.is_null() {
        return;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let inner = unsafe { HandleInner::from_raw(handle) };
        let arena = inner
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(arena) = arena {
            inner.runtime.block_on(arena.close());
        }
    }));
    if let Err(payload) = outcome {
        log::error!("panic in arena_close: {}", panic_message(&payload));
    }
}

#[no_mangle]
pub extern "C" fn arena_soft_reset(
    handle: *mut ArenaHandle,
    dependency_identifier: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    run_reset(handle, dependency_identifier, err_out, ResetKind::Soft)
}

#[no_mangle]
pub extern "C" fn arena_hard_reset(
    handle: *mut ArenaHandle,
    dependency_identifier: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    run_reset(handle, dependency_identifier, err_out, ResetKind::Hard)
}

#[derive(Clone, Copy)]
enum ResetKind {
    Soft,
    Hard,
}

fn reset_dependency(inner: &HandleInner, identifier: &str, kind: ResetKind) -> ArenaStatus {
    let mut guard = inner
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let arena = match guard.as_mut() {
        Some(a) => a,
        None => return ArenaStatus::Failed,
    };
    let dep = match arena.dependency_mut(identifier) {
        Some(d) => d,
        None => return ArenaStatus::NotFound,
    };
    inner.runtime.block_on(async {
        match kind {
            ResetKind::Soft => dep.soft_reset().await,
            ResetKind::Hard => dep.hard_reset().await,
        }
    });
    ArenaStatus::Ok
}

fn run_reset(
    handle: *mut ArenaHandle,
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
        let inner = unsafe { HandleInner::as_ref(handle) };
        reset_dependency(inner, &identifier, kind)
    }));
    match outcome {
        Ok(status) => {
            if status == ArenaStatus::NotFound {
                unsafe {
                    write_error(err_out, format!("dependency '{identifier}' not found"))
                };
            } else if status == ArenaStatus::Failed {
                unsafe { write_error(err_out, "reset: arena is already closed") };
            }
            status
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            log::error!("panic in arena reset: {msg}");
            unsafe { write_error(err_out, format!("panic in arena reset: {msg}")) };
            ArenaStatus::Panic
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

unsafe fn parse_config(
    ptr: *const c_char,
) -> Result<crate::matches::MatchConfig, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    #[test]
    fn open_close_roundtrip() {
        let name = CString::new("test").unwrap();
        let mut err: *mut c_char = std::ptr::null_mut();
        let h = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _);
        assert!(!h.is_null(), "expected handle, got error: {:?}", unsafe {
            if err.is_null() {
                None
            } else {
                Some(CStr::from_ptr(err).to_string_lossy().into_owned())
            }
        });
        arena_close(h);
        arena_free_string(err);
    }

    #[test]
    fn open_with_null_name_writes_error() {
        let mut err: *mut c_char = std::ptr::null_mut();
        let h = arena_open(std::ptr::null(), std::ptr::null(), &mut err as *mut _);
        assert!(h.is_null());
        assert!(!err.is_null());
        let msg = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
        assert!(msg.contains("name must not be null"), "got: {msg}");
        arena_free_string(err);
    }

    #[test]
    fn soft_reset_missing_dependency_reports_not_found() {
        let name = CString::new("test").unwrap();
        let mut err: *mut c_char = std::ptr::null_mut();
        let h = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _);
        assert!(!h.is_null());
        let dep = CString::new("does-not-exist").unwrap();
        let status = arena_soft_reset(h, dep.as_ptr(), &mut err as *mut _);
        assert_eq!(status, ArenaStatus::NotFound);
        assert!(!err.is_null());
        arena_free_string(err);
        arena_close(h);
    }

    #[test]
    fn concurrent_resets_are_serialized() {
        use std::thread;

        let name = CString::new("test").unwrap();
        let mut err: *mut c_char = std::ptr::null_mut();
        let h = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _);
        assert!(!h.is_null());
        arena_free_string(err);

        let handle_addr = h as usize;
        let threads: Vec<_> = (0..8)
            .map(|_| {
                thread::spawn(move || {
                    let ptr = handle_addr as *mut ArenaHandle;
                    let dep = CString::new("not-there").unwrap();
                    let mut e: *mut c_char = std::ptr::null_mut();
                    let status = arena_soft_reset(ptr, dep.as_ptr(), &mut e as *mut _);
                    arena_free_string(e);
                    status
                })
            })
            .collect();
        for t in threads {
            assert_eq!(t.join().unwrap(), ArenaStatus::NotFound);
        }
        arena_close(h);
    }
}
