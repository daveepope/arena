use std::ffi::CString;
use std::os::raw::c_char;
use std::thread;

use arena_ffi::{
    arena_close, arena_free_string, arena_hard_reset, arena_open, arena_soft_reset, ArenaStatus,
    OpenArenaHandle,
};

#[path = "ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

#[test]
fn arena_open_valid_name_plain_returns_live_handle_and_close_drains_cleanly() {
    let name = CString::new("test").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _, std::ptr::null_mut());
    assert!(!h.is_null(), "expected handle, got error: {}", err_text(err));
    arena_close(h, std::ptr::null_mut(), std::ptr::null_mut());
}

#[test]
fn arena_open_null_name_pointer_writes_error_returns_null_handle() {
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(std::ptr::null(), std::ptr::null(), &mut err as *mut _, std::ptr::null_mut());
    assert!(h.is_null());
    assert!(err_text(err).contains("name must not be null"));
}

#[test]
fn arena_open_malformed_config_json_writes_parse_error_returns_null_handle() {
    let name = CString::new("test").unwrap();
    let config = CString::new("{not valid json").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _, std::ptr::null_mut());
    assert!(h.is_null());
    assert!(err_text(err).contains("config failed to parse"));
}

#[test]
fn arena_open_empty_config_string_uses_default_config() {
    let name = CString::new("test-empty-config").unwrap();
    let config = CString::new("").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _, std::ptr::null_mut());
    assert!(!h.is_null(), "expected handle, got error: {}", err_text(err));
    arena_close(h, std::ptr::null_mut(), std::ptr::null_mut());
}

#[test]
fn arena_open_invalid_utf8_config_writes_error_returns_null_handle() {
    let name = CString::new("test").unwrap();
    let config = unsafe { CString::from_vec_with_nul_unchecked(vec![0xFF, 0xFE, 0x00]) };
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _, std::ptr::null_mut());
    assert!(h.is_null());
    assert!(err_text(err).contains("not valid UTF-8"));
}

#[test]
fn arena_close_null_handle_does_not_panic() {
    arena_close(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
}

#[test]
fn arena_soft_reset_missing_dependency_writes_not_found_keeps_live_handle() {
    let name = CString::new("test").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _, std::ptr::null_mut());
    assert!(!h.is_null());
    let dep = CString::new("does-not-exist").unwrap();
    let status = arena_soft_reset(h, dep.as_ptr(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::NotFound);
    assert!(err_text(err).contains("not found"));
    arena_close(h, std::ptr::null_mut(), std::ptr::null_mut());
}

#[test]
fn arena_hard_reset_missing_dependency_writes_not_found_keeps_live_handle() {
    let name = CString::new("test").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _, std::ptr::null_mut());
    assert!(!h.is_null());
    let dep = CString::new("does-not-exist").unwrap();
    let status = arena_hard_reset(h, dep.as_ptr(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::NotFound);
    assert!(err_text(err).contains("not found"));
    arena_close(h, std::ptr::null_mut(), std::ptr::null_mut());
}

#[test]
fn arena_soft_reset_null_handle_returns_invalid_argument() {
    let dep = CString::new("dep").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let status = arena_soft_reset(std::ptr::null_mut(), dep.as_ptr(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("handle must not be null"));
}

#[test]
fn arena_soft_reset_null_dependency_identifier_returns_invalid_argument() {
    let name = CString::new("test").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _, std::ptr::null_mut());
    assert!(!h.is_null());
    let status = arena_soft_reset(h, std::ptr::null(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("dependency_identifier must not be null"));
    arena_close(h, std::ptr::null_mut(), std::ptr::null_mut());
}

#[test]
fn arena_hard_reset_null_handle_returns_invalid_argument() {
    let dep = CString::new("dep").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let status = arena_hard_reset(std::ptr::null_mut(), dep.as_ptr(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("handle must not be null"));
}

#[test]
fn arena_soft_reset_many_threads_plain_all_see_unknown_dependency_as_not_found() {
    let name = CString::new("test").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _, std::ptr::null_mut());
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
    arena_close(h, std::ptr::null_mut(), std::ptr::null_mut());
}

#[test]
#[ignore]
fn arena_open_nested_component_children_builds_starts_and_closes_cleanly() {
    let name = CString::new("test-with-children").unwrap();
    let config = CString::new(
        r#"{
            "components": [
                {
                    "type": "exec",
                    "identifier": "exec-parent",
                    "executable_path": "/bin/true",
                    "children": [
                        {"type": "exec", "identifier": "exec-child", "executable_path": "/bin/true"}
                    ]
                }
            ]
        }"#,
    )
    .unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _, std::ptr::null_mut());
    assert!(!h.is_null(), "expected handle, got error: {}", err_text(err));
    arena_close(h, std::ptr::null_mut(), std::ptr::null_mut());
}

#[test]
#[ignore]
fn arena_open_nested_component_children_two_levels_builds_and_closes_cleanly() {
    let name = CString::new("test-with-deep-children").unwrap();
    let config = CString::new(
        r#"{
            "components": [
                {
                    "type": "exec",
                    "identifier": "exec-root",
                    "executable_path": "/bin/true",
                    "children": [
                        {
                            "type": "exec",
                            "identifier": "exec-mid",
                            "executable_path": "/bin/true",
                            "children": [
                                {"type": "exec", "identifier": "exec-leaf", "executable_path": "/bin/true"}
                            ]
                        }
                    ]
                }
            ]
        }"#,
    )
    .unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let h = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _, std::ptr::null_mut());
    assert!(!h.is_null(), "expected handle, got error: {}", err_text(err));
    arena_close(h, std::ptr::null_mut(), std::ptr::null_mut());
}
