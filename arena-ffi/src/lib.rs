mod builder;
mod parse;

use arena::{ClosedArena, OpenArena};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr::NonNull;

use builder::build_encounter;
use parse::EncounterJson;

#[repr(C)]
pub struct ArenaHandle {
    _private: [u8; 0],
}

#[no_mangle]
pub extern "C" fn arena_ffi_version() -> *const c_char {
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn arena_open(name: *const c_char, config_json: *const c_char) -> *mut ArenaHandle {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name_str = match unsafe { CStr::from_ptr(name).to_str() } {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };

    let json: EncounterJson = if config_json.is_null() {
        EncounterJson::default()
    } else {
        match unsafe { CStr::from_ptr(config_json).to_str() } {
            Ok(s) if s.is_empty() => EncounterJson::default(),
            Ok(s) => match serde_json::from_str(s) {
                Ok(v) => v,
                Err(_) => return std::ptr::null_mut(),
            },
            Err(_) => return std::ptr::null_mut(),
        }
    };

    let encounter = build_encounter(&json);
    let closed = ClosedArena::new(name_str, vec![encounter]);

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return std::ptr::null_mut(),
    };
    let open_arena = rt.block_on(closed.open());

    let boxed = Box::new(open_arena);
    let ptr = Box::into_raw(boxed) as *mut ArenaHandle;
    NonNull::new(ptr).map_or(std::ptr::null_mut(), |p| p.as_ptr())
}

#[no_mangle]
pub extern "C" fn arena_close(handle: *mut ArenaHandle) {
    if handle.is_null() {
        return;
    }
    let open = unsafe { Box::from_raw(handle as *mut OpenArena) };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return,
    };
    let _closed = rt.block_on(open.close());
}

fn run_reset<F>(handle: *mut ArenaHandle, dependency_identifier: *const c_char, reset_fn: F) -> bool
where
    F: FnOnce(&mut OpenArena, &str),
{
    if handle.is_null() || dependency_identifier.is_null() {
        return false;
    }
    let identifier = match unsafe { CStr::from_ptr(dependency_identifier).to_str() } {
        Ok(s) => s.to_string(),
        Err(_) => return false,
    };
    let open = unsafe { &mut *(handle as *mut OpenArena) };
    reset_fn(open, &identifier);
    true
}

fn do_soft_reset(open: &mut OpenArena, identifier: &str) {
    if let Some(dep) = open.dependency_mut(identifier) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(dep.soft_reset());
    }
}

fn do_hard_reset(open: &mut OpenArena, identifier: &str) {
    if let Some(dep) = open.dependency_mut(identifier) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(dep.hard_reset());
    }
}

#[no_mangle]
pub extern "C" fn arena_soft_reset(handle: *mut ArenaHandle, dependency_identifier: *const c_char) -> bool {
    run_reset(handle, dependency_identifier, do_soft_reset)
}

#[no_mangle]
pub extern "C" fn arena_hard_reset(handle: *mut ArenaHandle, dependency_identifier: *const c_char) -> bool {
    run_reset(handle, dependency_identifier, do_hard_reset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn version_returns_non_null() {
        let p = arena_ffi_version();
        assert!(!p.is_null());
        let s = unsafe { CStr::from_ptr(p).to_str().unwrap() };
        assert!(!s.is_empty());
    }

    #[test]
    fn open_close_roundtrip() {
        let name = CString::new("test").unwrap();
        let h = arena_open(name.as_ptr(), std::ptr::null());
        assert!(!h.is_null());
        arena_close(h);
    }
}
