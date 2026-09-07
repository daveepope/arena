use arena_ffi::{
    arena_dispatcher_component_allow_json_set, arena_dispatcher_dependency_allow_json_set,
    arena_set_log_level, ArenaLogLevel, ArenaStatus,
};

#[test]
fn arena_set_log_level_each_known_level_returns_ok() {
    let levels = [
        ArenaLogLevel::Error,
        ArenaLogLevel::Warn,
        ArenaLogLevel::Info,
        ArenaLogLevel::Debug,
        ArenaLogLevel::Trace,
    ];

    for level in levels {
        assert_eq!(
            arena_set_log_level(level as i32),
            ArenaStatus::Ok,
            "level {level:?} must be accepted"
        );
    }
    arena_set_log_level(ArenaLogLevel::Info as i32);
}

#[test]
fn arena_set_log_level_out_of_range_returns_invalid_argument() {
    for level in [0, 6, -1, i32::MIN, i32::MAX] {
        assert_eq!(
            arena_set_log_level(level),
            ArenaStatus::InvalidArgument,
            "level {level} must be rejected"
        );
    }
}

#[test]
fn arena_dispatcher_allow_json_set_null_pointer_clears_both_allowlists() {
    unsafe {
        arena_dispatcher_dependency_allow_json_set(std::ptr::null());
        arena_dispatcher_component_allow_json_set(std::ptr::null());
    }
}
