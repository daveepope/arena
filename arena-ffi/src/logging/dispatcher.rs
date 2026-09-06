use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use super::shared_tracing::{
    ArenaEmittedRecord, ArenaLogDelivery, ArenaLoggingTarget,
};
use super::severity_level::Level as ArenaRustLevel;
use crate::boundary::call_across_boundary;

pub type ArenaLogCallback = unsafe extern "C" fn(
    level: i32,
    target: *const c_char,
    ts_unix_nanos: i64,
    message: *const c_char,
    caller_file_utf8: *const c_char,
    caller_line: u32,
    user_data: *mut c_void,
);

static ABI_LOG_TARGETS: LazyLock<Mutex<HashMap<u64, ArenaLogDelivery>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static ABI_NEXT_LOG_TARGET_TOKEN: AtomicU64 = AtomicU64::new(1);

static DEFAULT_DISPATCHER_LOGGING_TARGET_LOGGER_NAME: &[u8] = b"arena\0";

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaLogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

struct ForwardToCAbi {
    func: ArenaLogCallback,
    binding: *mut c_void,
}

unsafe impl Send for ForwardToCAbi {}
unsafe impl Sync for ForwardToCAbi {}

impl ArenaLoggingTarget for ForwardToCAbi {
    fn deliver(&self, record: ArenaEmittedRecord) {
        let level = match record.severity {
            ArenaRustLevel::Error => ArenaLogLevel::Error as i32,
            ArenaRustLevel::Warn => ArenaLogLevel::Warn as i32,
            ArenaRustLevel::Info => ArenaLogLevel::Info as i32,
            ArenaRustLevel::Debug => ArenaLogLevel::Debug as i32,
            ArenaRustLevel::Trace => ArenaLogLevel::Trace as i32,
        };

        let Ok(c_target) = CString::new(record.target) else {
            return;
        };
        let Ok(c_message) = CString::new(record.payload) else {
            return;
        };

        let c_caller = record
            .caller_file_utf8
            .as_ref()
            .and_then(|s| CString::new(s.as_bytes()).ok());
        let caller_ptr = c_caller
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null());
        let caller_line = if caller_ptr.is_null() {
            0
        } else {
            record.caller_line
        };

        unsafe {
            (self.func)(
                level,
                c_target.as_ptr(),
                record.unix_timestamp_ns,
                c_message.as_ptr(),
                caller_ptr,
                caller_line,
                self.binding,
            );
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_add_log_target(
    callback: Option<ArenaLogCallback>,
    user_data: *mut c_void,
) -> u64 {
    let Some(callback) = callback else {
        return 0;
    };
    let binding = user_data as usize;
    call_across_boundary(move || {
        super::init_logging();

        let token = ABI_NEXT_LOG_TARGET_TOKEN.fetch_add(1, Ordering::Relaxed);
        let subscription = ArenaLogDelivery::subscribe(Arc::new(ForwardToCAbi {
            func: callback,
            binding: binding as *mut c_void,
        }));

        let mut slots = ABI_LOG_TARGETS.lock().unwrap_or_else(|p| p.into_inner());
        slots.insert(token, subscription);
        token
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn arena_remove_log_target(token: u64) {
    if token == 0 {
        return;
    }
    let _ = call_across_boundary(move || {
        let mut slots = ABI_LOG_TARGETS.lock().unwrap_or_else(|p| p.into_inner());
        let _released = slots.remove(&token);
    });
}

#[no_mangle]
pub extern "C" fn arena_dispatcher_default_logging_target_logger_name_utf8() -> *const c_char {
    call_across_boundary(|| DEFAULT_DISPATCHER_LOGGING_TARGET_LOGGER_NAME.as_ptr() as usize)
        .unwrap_or(0) as *const c_char
}

#[no_mangle]
pub extern "C" fn arena_dispatcher_default_logging_target_publish_level(level: i32) -> i32 {
    call_across_boundary(move || dispatcher_default_logging_target_publish_level_of(level))
        .unwrap_or(ArenaLogLevel::Info as i32)
}

fn dispatcher_default_logging_target_publish_level_of(level: i32) -> i32 {
    match level {
        x if x == ArenaLogLevel::Error as i32 => ArenaLogLevel::Error as i32,
        x if x == ArenaLogLevel::Warn as i32 => ArenaLogLevel::Warn as i32,
        x if x == ArenaLogLevel::Info as i32 => ArenaLogLevel::Info as i32,
        x if x == ArenaLogLevel::Debug as i32 => ArenaLogLevel::Debug as i32,
        x if x == ArenaLogLevel::Trace as i32 => ArenaLogLevel::Trace as i32,
        _ => ArenaLogLevel::Info as i32,
    }
}
