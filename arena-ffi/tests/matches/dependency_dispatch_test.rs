use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use arena_ffi::{arena_close, arena_free_string, arena_open};

#[test]
fn arena_open_temporal_dependency_config_returns_live_handle() {
    let name = CString::new("test").unwrap();
    let config = CString::new(r#"{"dependencies":[{"type":"temporal","identifier":"temporal"}]}"#)
        .unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _);
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
