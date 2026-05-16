use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use arena_ffi::{
    arena_close, arena_free_string, arena_open, arena_soft_reset, ArenaStatus, OpenArenaHandle,
};

#[test]
fn arena_open_valid_name_plain_returns_live_handle_and_close_drains_cleanly() {
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
fn arena_open_null_name_pointer_writes_error_returns_null_handle() {
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(std::ptr::null(), std::ptr::null(), &mut err as *mut _);
    assert!(h.is_null());
    assert!(!err.is_null());
    let msg = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
    assert!(msg.contains("name must not be null"), "got: {msg}");
    arena_free_string(err);
}

#[test]
fn arena_soft_reset_missing_dependency_writes_not_found_keeps_live_handle() {
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
fn arena_soft_reset_many_threads_plain_all_see_unknown_dependency_as_not_found() {
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
                let ptr = handle_addr as *mut OpenArenaHandle;
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
