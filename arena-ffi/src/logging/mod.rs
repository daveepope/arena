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

use env_filter_reload::set_global_level;
use severity_level::Level as InternalLogLevel;

pub(crate) fn init_logging() {
    shared_tracing::ensure_shared_tracing_installed();
}

#[no_mangle]
pub unsafe extern "C" fn arena_dispatcher_dependency_allow_json_set(json_utf8: *const c_char) {
    init_logging();
    unsafe {
        shared_tracing::dispatcher_dependency_allowlist_set_ptr(json_utf8);
    }
}

#[no_mangle]
pub unsafe extern "C" fn arena_dispatcher_component_allow_json_set(json_utf8: *const c_char) {
    init_logging();
    unsafe {
        shared_tracing::dispatcher_component_allowlist_set_ptr(json_utf8);
    }
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
pub extern "C" fn arena_set_log_level(level: ArenaLogLevel) {
    init_logging();
    set_global_level(to_internal_level(level));
    tracing::info!(
        target: "arena::ffi",
        arena_log_level = ?level,
        "arena log level set",
    );
}
