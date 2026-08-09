use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use arena_ffi::{arena_close, arena_free_string, arena_open};

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
