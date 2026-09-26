use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;

use arena_ffi::{arena_add_log_target, arena_remove_log_target};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Record {
    pub level: i32,
    pub target: String,
    pub message: String,
    pub caller_file: Option<String>,
    pub caller_line: u32,
    pub user_data: usize,
}

pub static TARGET_API_LOCK: Mutex<()> = Mutex::new(());
pub static RECORDED: Mutex<Vec<Record>> = Mutex::new(Vec::new());

pub unsafe extern "C" fn collecting_callback(
    level: i32,
    target: *const c_char,
    _ts: i64,
    message: *const c_char,
    caller_file: *const c_char,
    caller_line: u32,
    user_data: *mut c_void,
) {
    push(
        &RECORDED,
        level,
        target,
        message,
        caller_file,
        caller_line,
        user_data,
    );
}

pub fn push(
    records: &Mutex<Vec<Record>>,
    level: i32,
    target: *const c_char,
    message: *const c_char,
    caller_file: *const c_char,
    caller_line: u32,
    user_data: *mut c_void,
) {
    let target = unsafe { CStr::from_ptr(target) }
        .to_string_lossy()
        .into_owned();
    let message = unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned();
    let caller_path = if caller_file.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(caller_file) }
                .to_string_lossy()
                .into_owned(),
        )
    };
    records
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Record {
            level,
            target,
            message,
            caller_file: caller_path,
            caller_line,
            user_data: user_data as usize,
        });
}

pub fn drain(records: &Mutex<Vec<Record>>) -> Vec<Record> {
    let mut g = records.lock().unwrap_or_else(|e| e.into_inner());
    let out = g.clone();
    g.clear();
    out
}

pub fn records_emitted_within<F: FnOnce()>(emit: F) -> Vec<Record> {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain(&RECORDED);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    emit();
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);
    captured
}

pub fn record_emitted_within<F: FnOnce()>(marker: &str, emit: F) -> Record {
    records_emitted_within(emit)
        .into_iter()
        .find(|r| r.message.contains(marker))
        .unwrap_or_else(|| panic!("no record captured for {marker}"))
}
