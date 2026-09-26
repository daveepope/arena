mod dispatcher;
mod env_filter_reload;
mod severity_level;
mod shared_tracing;

pub use dispatcher::{
    arena_add_log_target, arena_dispatcher_default_logging_target_logger_name_utf8,
    arena_dispatcher_default_logging_target_publish_level, arena_remove_log_target,
    ArenaLogCallback, ArenaLogLevel,
};

use std::ffi::c_char;

use crate::boundary::call_across_boundary;
use crate::error::ArenaStatus;
use env_filter_reload::set_global_level;
use severity_level::Level as InternalLogLevel;

pub(crate) fn init_logging() {
    shared_tracing::ensure_shared_tracing_installed();
}

pub(crate) fn dispatcher_allowlists_reset() {
    shared_tracing::dispatcher_allowlists_reset();
}

#[no_mangle]
pub unsafe extern "C" fn arena_dispatcher_dependency_allow_json_set(json_utf8: *const c_char) {
    let address = json_utf8 as usize;
    let _ = call_across_boundary(move || {
        init_logging();
        unsafe {
            shared_tracing::dispatcher_dependency_allowlist_set_ptr(address as *const c_char);
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn arena_dispatcher_component_allow_json_set(json_utf8: *const c_char) {
    let address = json_utf8 as usize;
    let _ = call_across_boundary(move || {
        init_logging();
        unsafe {
            shared_tracing::dispatcher_component_allowlist_set_ptr(address as *const c_char);
        }
    });
}

fn to_internal_level(level: ArenaLogLevel) -> InternalLogLevel {
    match level {
        ArenaLogLevel::Error => InternalLogLevel::Error,
        ArenaLogLevel::Warn => InternalLogLevel::Warn,
        ArenaLogLevel::Info => InternalLogLevel::Info,
        ArenaLogLevel::Debug => InternalLogLevel::Debug,
        ArenaLogLevel::Trace => InternalLogLevel::Trace,
    }
}

#[no_mangle]
pub extern "C" fn arena_set_log_level(level: i32) -> ArenaStatus {
    let Some(level) = arena_log_level_from_i32(level) else {
        return ArenaStatus::InvalidArgument;
    };
    let outcome = call_across_boundary(|| {
        init_logging();
        set_global_level(to_internal_level(level));
        tracing::info!(
            target: "arena::ffi",
            arena_log_level = ?level,
            "arena log level set",
        );
    });
    match outcome {
        Ok(()) => ArenaStatus::Ok,
        Err(_) => ArenaStatus::Panic,
    }
}

fn arena_log_level_from_i32(level: i32) -> Option<ArenaLogLevel> {
    match level {
        x if x == ArenaLogLevel::Error as i32 => Some(ArenaLogLevel::Error),
        x if x == ArenaLogLevel::Warn as i32 => Some(ArenaLogLevel::Warn),
        x if x == ArenaLogLevel::Info as i32 => Some(ArenaLogLevel::Info),
        x if x == ArenaLogLevel::Debug as i32 => Some(ArenaLogLevel::Debug),
        x if x == ArenaLogLevel::Trace as i32 => Some(ArenaLogLevel::Trace),
        _ => None,
    }
}
