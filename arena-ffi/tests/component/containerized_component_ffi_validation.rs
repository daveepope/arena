use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use arena_ffi::{arena_free_string, arena_open};

fn err_text(err: *mut c_char) -> String {
    if err.is_null() {
        return String::new();
    }
    let msg = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
    arena_free_string(err);
    msg
}

fn open_expecting_failure(config_json: &str) -> String {
    let name = CString::new("containerized-validation").unwrap();
    let config = CString::new(config_json).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = arena_open(name.as_ptr(), config.as_ptr(), &mut err as *mut _);
    assert!(handle.is_null(), "expected arena_open to fail and return null");
    err_text(err)
}

#[test]
fn arena_open_bind_mount_missing_source_reports_error() {
    let err = open_expecting_failure(
        r#"{
            "components": [{
                "type": "container",
                "identifier": "web",
                "dockerfile": "FROM alpine",
                "mounts": [{
                    "type": "bind",
                    "source": "/arena-ffi-nonexistent-bind-source",
                    "container_path": "/mnt/data"
                }]
            }]
        }"#,
    );
    assert!(
        err.contains("bind mount source path does not exist"),
        "unexpected error: {err}"
    );
}

#[test]
fn arena_open_unknown_mount_type_reports_parse_error() {
    let err = open_expecting_failure(
        r#"{
            "components": [{
                "type": "container",
                "identifier": "web",
                "containerfile": "FROM alpine",
                "mounts": [{"type": "nfs", "source": "s", "container_path": "/mnt/data"}]
            }]
        }"#,
    );
    assert!(err.contains("config failed to parse"), "unexpected error: {err}");
}

#[test]
fn arena_open_bind_mount_missing_container_path_reports_parse_error() {
    let err = open_expecting_failure(
        r#"{
            "components": [{
                "type": "container",
                "identifier": "web",
                "containerfile": "FROM alpine",
                "mounts": [{"type": "bind", "source": "/tmp"}]
            }]
        }"#,
    );
    assert!(err.contains("config failed to parse"), "unexpected error: {err}");
}

#[test]
fn arena_open_containerfile_absent_reports_parse_error() {
    let err = open_expecting_failure(
        r#"{
            "components": [{"type": "container", "identifier": "web"}]
        }"#,
    );
    assert!(err.contains("config failed to parse"), "unexpected error: {err}");
}
