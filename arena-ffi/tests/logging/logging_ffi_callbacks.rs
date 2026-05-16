use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;

use arena_ffi::{
    arena_add_log_target, arena_dispatcher_component_allow_json_set,
    arena_dispatcher_default_logging_target_logger_name_utf8,
    arena_dispatcher_default_logging_target_publish_level,
    arena_dispatcher_dependency_allow_json_set, arena_remove_log_target,
    arena_set_log_level, ArenaLogLevel,
};

#[derive(Clone, Debug)]
struct Record {
    level: i32,
    target: String,
    message: String,
    caller_file: Option<String>,
    caller_line: u32,
    user_data: usize,
}

static TARGET_API_LOCK: Mutex<()> = Mutex::new(());
static RECORDED: Mutex<Vec<Record>> = Mutex::new(Vec::new());
static RECORDED_B: Mutex<Vec<Record>> = Mutex::new(Vec::new());

const FFI_ALLOWLIST_SYNTH_KAFKA_DEP_MSG: &str = "ffi-dep-dispatcher-allow-carrier-marker";
const FFI_ALLOWLIST_SYNTH_EXEC_COMP_MSG: &str = "ffi-comp-dispatcher-allow-carrier-marker";
const FFI_LOGGING_SYNTH_DEP_TAIL: &str = "ffi-logging-deps-needle-xq";
const FFI_LOGGING_SYNTH_COMP_TAIL: &str = "ffi-logging-comp-needle-yq";

unsafe extern "C" fn collecting_callback(
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

unsafe extern "C" fn collecting_callback_b(
    level: i32,
    target: *const c_char,
    _ts: i64,
    message: *const c_char,
    caller_file: *const c_char,
    caller_line: u32,
    user_data: *mut c_void,
) {
    push(
        &RECORDED_B,
        level,
        target,
        message,
        caller_file,
        caller_line,
        user_data,
    );
}

fn push(
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
        Some(unsafe { CStr::from_ptr(caller_file) }.to_string_lossy().into_owned())
    };
    records.lock().unwrap_or_else(|e| e.into_inner()).push(Record {
        level,
        target,
        message,
        caller_file: caller_path,
        caller_line,
        user_data: user_data as usize,
    });
}

fn drain(records: &Mutex<Vec<Record>>) -> Vec<Record> {
    let mut g = records.lock().unwrap_or_else(|e| e.into_inner());
    let out = g.clone();
    g.clear();
    out
}

fn reset_dispatcher_allowlists_via_ffi() {
    unsafe {
        arena_dispatcher_dependency_allow_json_set(std::ptr::null());
        arena_dispatcher_component_allow_json_set(std::ptr::null());
    }
}

fn deps_allow_json_ffi(json_array_utf8: &str) {
    let c = CString::new(json_array_utf8).expect("dep allow json cstring");
    unsafe {
        arena_dispatcher_dependency_allow_json_set(c.as_ptr());
    }
}

fn comps_allow_json_ffi(json_array_utf8: &str) {
    let c = CString::new(json_array_utf8).expect("component allow json cstring");
    unsafe {
        arena_dispatcher_component_allow_json_set(c.as_ptr());
    }
}

fn expect_dispatcher_callback_payload_format(record: &Record) {
    let msg = &record.message;
    assert!(
        msg.starts_with('['),
        "callback message (payload) must start with [tracing-target]: {:?}",
        msg
    );
    let close_bracket = msg
        .char_indices()
        .find(|(_, c)| *c == ']')
        .map(|(i, _)| i)
        .expect("closing ] for tracing target bracket");
    let tag = &msg[1..close_bracket];
    assert!(
        !tag.contains('/') && !tag.contains('\\'),
        "tracing target tag inside payload must not look like a filesystem path: {tag:?}"
    );
}

fn expect_tracing_target_param_is_module_like(record: &Record) {
    assert!(
        record.target.starts_with("ffi_logging_test"),
        "tracing target param should use module_path-derived string: {:?}",
        record.target
    );
    assert!(
        !record.target.contains('/') && !record.target.contains('\\'),
        "tracing target param must not embed filesystem path segments: {:?}",
        record.target
    );
}

fn expect_caller_field_basename_rs(record: &Record) {
    assert!(
        record.caller_line > 0,
        "caller_line should be set when rustc provides file/line metadata: {:?}:{:?}",
        record.caller_file,
        record.caller_line
    );
    let file = record
        .caller_file
        .as_deref()
        .expect("caller_file should accompany caller_line");
    assert!(
        file.ends_with(".rs") && !file.contains('/') && !file.contains('\\'),
        "caller_file must be basename only: {:?}",
        record.caller_file
    );
}

#[test]
fn arena_add_log_target_tracing_emit_invokes_callback() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain(&RECORDED);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    tracing::info!(case = "dispatcher_roundtrip", "dispatcher-roundtrip {}", 42);
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);

    let record = captured
        .iter()
        .find(|r| r.message.contains("dispatcher-roundtrip"))
        .unwrap_or_else(|| panic!("expected captured record, got {captured:?}"));
    assert_eq!(record.level, ArenaLogLevel::Info as i32);
    expect_tracing_target_param_is_module_like(record);
    assert!(
        record.message.contains("dispatcher-roundtrip 42"),
        "message did not render fmt args: {}",
        record.message
    );
    expect_dispatcher_callback_payload_format(record);
    assert!(
        record.message.starts_with("[ffi_logging_test"),
        "integration test crate should lead payload with [ffi_logging_test…]: {:?}",
        record.message
    );
    expect_caller_field_basename_rs(record);
    assert_eq!(
        record.caller_file.as_deref(),
        Some("logging_ffi_callbacks.rs"),
        "expected this test file basename in caller_file"
    );
}

#[test]
fn arena_add_log_target_tracing_structured_kv_fields_follow_message_fragment_in_payload() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain(&RECORDED);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    tracing::info!(
        target: "arena::payload_kv_probe",
        elapsed_ms = 93_u64,
        "kv-probe-msg"
    );
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);

    let record = captured
        .iter()
        .find(|r| r.message.contains("kv-probe-msg"))
        .unwrap_or_else(|| panic!("expected captured record, got {captured:?}"));
    assert!(
        record.message.contains("kv-probe-msg | elapsed_ms=93"),
        "expected structured field forwarded after message with separator, got {}",
        record.message
    );
    expect_dispatcher_callback_payload_format(record);
    expect_tracing_target_param_is_module_like(record);
    expect_caller_field_basename_rs(record);
}

#[test]
fn arena_remove_log_target_callbacks_stop_after_removal() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    arena_remove_log_target(handle);
    drain(&RECORDED);
    tracing::info!(case = "after_remove_negative", "after-remove-message");
    let captured = drain(&RECORDED);
    assert!(
        captured
            .iter()
            .all(|r| !r.message.contains("after-remove-message")),
        "expected no records after removal, got {captured:?}"
    );
}

#[test]
fn arena_add_log_target_user_data_plain_passes_through() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain(&RECORDED);
    let token: *mut c_void = 0xDEAD_BEEFusize as *mut c_void;
    let handle = arena_add_log_target(Some(collecting_callback), token);
    tracing::info!(case = "user_data_plain", "user-data-passthrough");
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);

    let record = captured
        .iter()
        .find(|r| r.message.contains("user-data-passthrough"))
        .unwrap_or_else(|| panic!("expected captured record, got {captured:?}"));
    assert_eq!(record.user_data, 0xDEAD_BEEFusize);
    expect_dispatcher_callback_payload_format(record);
    expect_tracing_target_param_is_module_like(record);
    expect_caller_field_basename_rs(record);
}

#[test]
fn arena_add_log_target_tracing_level_maps_into_ffi_level_int() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain(&RECORDED);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    tracing::error!(case = "level_probe_error", "level-probe-error");
    tracing::warn!(case = "level_probe_warn", "level-probe-warn");
    tracing::info!(case = "level_probe_info", "level-probe-info");
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);

    let pick_record = |needle: &str| -> &Record {
        captured
            .iter()
            .find(|r| r.message.contains(needle))
            .unwrap_or_else(|| panic!("missing {needle} in {captured:?}"))
    };

    for needle in ["level-probe-error", "level-probe-warn", "level-probe-info"] {
        let r = pick_record(needle);
        expect_dispatcher_callback_payload_format(r);
        expect_tracing_target_param_is_module_like(r);
        expect_caller_field_basename_rs(r);
        assert_eq!(r.caller_file.as_deref(), Some("logging_ffi_callbacks.rs"));
    }

    let pick = |needle: &str| -> i32 { pick_record(needle).level };

    assert_eq!(pick("level-probe-error"), ArenaLogLevel::Error as i32);
    assert_eq!(pick("level-probe-warn"), ArenaLogLevel::Warn as i32);
    assert_eq!(pick("level-probe-info"), ArenaLogLevel::Info as i32);
}

#[test]
fn arena_add_log_target_many_targets_fan_out_each_receive() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain(&RECORDED);
    drain(&RECORDED_B);
    let h1 = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    let h2 = arena_add_log_target(Some(collecting_callback_b), std::ptr::null_mut());
    assert_ne!(h1, h2);
    tracing::info!(case = "fan_out_marker", "fan-out-marker");
    let a = drain(&RECORDED);
    let b = drain(&RECORDED_B);
    arena_remove_log_target(h1);
    arena_remove_log_target(h2);

    assert!(
        a.iter().any(|r| r.message.contains("fan-out-marker")),
        "target A missed the event: {a:?}"
    );
    assert!(
        b.iter().any(|r| r.message.contains("fan-out-marker")),
        "target B missed the event: {b:?}"
    );
    let ra = a.iter().find(|r| r.message.contains("fan-out-marker")).expect("fan-out a");
    let rb = b.iter().find(|r| r.message.contains("fan-out-marker")).expect("fan-out b");
    for record in [ra, rb] {
        expect_dispatcher_callback_payload_format(record);
        expect_tracing_target_param_is_module_like(record);
        expect_caller_field_basename_rs(record);
    }
}

#[test]
fn arena_set_log_level_info_suppresses_debug_until_debug_allowed() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain(&RECORDED);
    arena_set_log_level(ArenaLogLevel::Info);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);

    tracing::debug!(
        case = "level_gate_blocked_debug",
        "level-gate-should-not-reach-callback"
    );
    let after_debug = drain(&RECORDED);
    assert!(
        after_debug
            .iter()
            .all(|r| !r.message.contains("level-gate-should-not-reach-callback")),
        "expected no callback for debug when filter is info, got {after_debug:?}"
    );

    tracing::info!(case = "level_gate_info", "level-gate-info-should-reach");
    let after_info = drain(&RECORDED);
    assert!(
        after_info
            .iter()
            .any(|r| r.message.contains("level-gate-info-should-reach")),
        "expected callback for info when filter is info, got {after_info:?}"
    );
    let info_rec = after_info
        .iter()
        .find(|r| r.message.contains("level-gate-info-should-reach"))
        .expect("info record");
    expect_dispatcher_callback_payload_format(info_rec);
    expect_tracing_target_param_is_module_like(info_rec);
    expect_caller_field_basename_rs(info_rec);

    arena_set_log_level(ArenaLogLevel::Debug);
    drain(&RECORDED);
    tracing::debug!(
        case = "level_gate_debug_after_raise",
        "level-gate-debug-should-reach-after-raise",
    );
    let after_lowered = drain(&RECORDED);
    assert!(
        after_lowered
            .iter()
            .any(|r| r.message.contains("level-gate-debug-should-reach-after-raise")),
        "expected callback for debug after set_log_level(Debug), got {after_lowered:?}"
    );
    assert_eq!(
        after_lowered
            .iter()
            .find(|r| r.message.contains("level-gate-debug-should-reach-after-raise"))
            .expect("record")
            .level,
        ArenaLogLevel::Debug as i32
    );
    let dbg_rec = after_lowered
        .iter()
        .find(|r| r.message.contains("level-gate-debug-should-reach-after-raise"))
        .expect("debug record after lower");
    expect_dispatcher_callback_payload_format(dbg_rec);
    expect_tracing_target_param_is_module_like(dbg_rec);
    expect_caller_field_basename_rs(dbg_rec);

    arena_remove_log_target(handle);
    arena_set_log_level(ArenaLogLevel::Info);
}

#[test]
fn arena_dispatcher_default_logging_target_logger_name_utf8_matches_dispatcher_channel() {
    let ptr = arena_dispatcher_default_logging_target_logger_name_utf8();
    assert!(!ptr.is_null());
    let name = unsafe { CStr::from_ptr(ptr).to_string_lossy() };
    assert_eq!(name.as_ref(), "arena.rust.dispatcher");
}

#[test]
fn arena_dispatcher_default_logging_target_publish_level_passthrough_known_levels() {
    assert_eq!(
        arena_dispatcher_default_logging_target_publish_level(ArenaLogLevel::Debug as i32),
        ArenaLogLevel::Debug as i32
    );
    assert_eq!(
        arena_dispatcher_default_logging_target_publish_level(ArenaLogLevel::Trace as i32),
        ArenaLogLevel::Trace as i32
    );
    assert_eq!(
        arena_dispatcher_default_logging_target_publish_level(ArenaLogLevel::Error as i32),
        ArenaLogLevel::Error as i32
    );
    assert_eq!(
        arena_dispatcher_default_logging_target_publish_level(ArenaLogLevel::Warn as i32),
        ArenaLogLevel::Warn as i32
    );
    assert_eq!(
        arena_dispatcher_default_logging_target_publish_level(ArenaLogLevel::Info as i32),
        ArenaLogLevel::Info as i32
    );
    assert_eq!(
        arena_dispatcher_default_logging_target_publish_level(999_i32),
        ArenaLogLevel::Info as i32
    );
}

#[test]
fn arena_dispatcher_dependency_allow_json_cleared_kafka_dep_marker_callbacks_empty() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_dispatcher_allowlists_via_ffi();
    drain(&RECORDED);
    arena_set_log_level(ArenaLogLevel::Trace);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    let dep_ident = format!("arena-kafka-{FFI_LOGGING_SYNTH_DEP_TAIL}-id");
    tracing::info!(
        target: "arena_kafka::ffi_allowlist_kafka_carrier",
        dependency = %dep_ident,
        "{}",
        FFI_ALLOWLIST_SYNTH_KAFKA_DEP_MSG
    );
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);
    reset_dispatcher_allowlists_via_ffi();
    arena_set_log_level(ArenaLogLevel::Info);
    assert!(
        captured
            .iter()
            .all(|r| !r.message.contains(FFI_ALLOWLIST_SYNTH_KAFKA_DEP_MSG)),
        "unexpected records with cleared dep allowlists: {captured:?}"
    );
}

#[test]
fn arena_dispatcher_dependency_allow_json_matching_needle_kafka_dep_marker_invokes_callback() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_dispatcher_allowlists_via_ffi();
    drain(&RECORDED);
    arena_set_log_level(ArenaLogLevel::Trace);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    deps_allow_json_ffi(r#"["ffi-logging-deps-needle"]"#);
    let dep_ident = format!("arena-kafka-{FFI_LOGGING_SYNTH_DEP_TAIL}-id");
    tracing::info!(
        target: "arena_kafka::ffi_allowlist_kafka_carrier",
        dependency = %dep_ident,
        "{}",
        FFI_ALLOWLIST_SYNTH_KAFKA_DEP_MSG
    );
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);
    reset_dispatcher_allowlists_via_ffi();
    arena_set_log_level(ArenaLogLevel::Info);

    let record = captured
        .iter()
        .find(|r| r.message.contains(FFI_ALLOWLIST_SYNTH_KAFKA_DEP_MSG))
        .unwrap_or_else(|| panic!("expected captured record, got {captured:?}"));
    assert_eq!(record.level, ArenaLogLevel::Info as i32);
    expect_dispatcher_callback_payload_format(record);
    assert!(
        record.message.contains("dependency=")
            && record.message.contains("ffi-logging-deps-needle"),
        "expected dependency field in payload: {}",
        record.message
    );
    expect_tracing_target_param_is_module_like(record);
    expect_caller_field_basename_rs(record);
}

#[test]
fn arena_dispatcher_dependency_allow_json_nonmatching_needle_kafka_dep_marker_callbacks_empty() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_dispatcher_allowlists_via_ffi();
    drain(&RECORDED);
    arena_set_log_level(ArenaLogLevel::Trace);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    deps_allow_json_ffi(r#"["only-other-substr-present"]"#);
    let dep_ident = format!("arena-kafka-{FFI_LOGGING_SYNTH_DEP_TAIL}-id");
    tracing::info!(
        target: "arena_kafka::ffi_allowlist_kafka_carrier",
        dependency = %dep_ident,
        "{}",
        FFI_ALLOWLIST_SYNTH_KAFKA_DEP_MSG
    );
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);
    reset_dispatcher_allowlists_via_ffi();
    arena_set_log_level(ArenaLogLevel::Info);
    assert!(
        captured
            .iter()
            .all(|r| !r.message.contains(FFI_ALLOWLIST_SYNTH_KAFKA_DEP_MSG)),
        "expected allowlist mismatch to drop synthetic dep event: {captured:?}"
    );
}

#[test]
fn arena_dispatcher_component_allow_json_cleared_exec_comp_marker_callbacks_empty() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_dispatcher_allowlists_via_ffi();
    drain(&RECORDED);
    arena_set_log_level(ArenaLogLevel::Trace);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    let comp_ident = format!("arena-executable-component-{FFI_LOGGING_SYNTH_COMP_TAIL}-zz");
    tracing::info!(
        target: "arena_executable_component::executable_component",
        component = %comp_ident,
        "{}",
        FFI_ALLOWLIST_SYNTH_EXEC_COMP_MSG
    );
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);
    reset_dispatcher_allowlists_via_ffi();
    arena_set_log_level(ArenaLogLevel::Info);
    assert!(
        captured
            .iter()
            .all(|r| !r.message.contains(FFI_ALLOWLIST_SYNTH_EXEC_COMP_MSG)),
        "unexpected records with cleared component allowlists: {captured:?}"
    );
}

#[test]
fn arena_dispatcher_component_allow_json_matching_needle_exec_comp_marker_invokes_callback() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_dispatcher_allowlists_via_ffi();
    drain(&RECORDED);
    arena_set_log_level(ArenaLogLevel::Trace);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    comps_allow_json_ffi(r#"["ffi-logging-comp-needle"]"#);
    let comp_ident = format!("arena-executable-component-{FFI_LOGGING_SYNTH_COMP_TAIL}-zz");
    tracing::info!(
        target: "arena_executable_component::executable_component",
        component = %comp_ident,
        "{}",
        FFI_ALLOWLIST_SYNTH_EXEC_COMP_MSG
    );
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);
    reset_dispatcher_allowlists_via_ffi();
    arena_set_log_level(ArenaLogLevel::Info);

    let record = captured
        .iter()
        .find(|r| r.message.contains(FFI_ALLOWLIST_SYNTH_EXEC_COMP_MSG))
        .unwrap_or_else(|| panic!("expected captured record, got {captured:?}"));
    assert_eq!(record.level, ArenaLogLevel::Info as i32);
    expect_dispatcher_callback_payload_format(record);
    assert!(
        record.message.contains("component=") && record.message.contains("ffi-logging-comp-needle"),
        "expected component field in payload: {}",
        record.message
    );
    expect_tracing_target_param_is_module_like(record);
    expect_caller_field_basename_rs(record);
}

#[test]
fn arena_dispatcher_component_allow_json_nonmatching_needle_exec_comp_marker_callbacks_empty() {
    let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_dispatcher_allowlists_via_ffi();
    drain(&RECORDED);
    arena_set_log_level(ArenaLogLevel::Trace);
    let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
    assert_ne!(handle, 0);
    comps_allow_json_ffi(r#"["only-other-comp-substr-present"]"#);
    let comp_ident = format!("arena-executable-component-{FFI_LOGGING_SYNTH_COMP_TAIL}-zz");
    tracing::info!(
        target: "arena_executable_component::executable_component",
        component = %comp_ident,
        "{}",
        FFI_ALLOWLIST_SYNTH_EXEC_COMP_MSG
    );
    let captured = drain(&RECORDED);
    arena_remove_log_target(handle);
    reset_dispatcher_allowlists_via_ffi();
    arena_set_log_level(ArenaLogLevel::Info);
    assert!(
        captured
            .iter()
            .all(|r| !r.message.contains(FFI_ALLOWLIST_SYNTH_EXEC_COMP_MSG)),
        "expected allowlist mismatch to drop synthetic component event: {captured:?}"
    );
}

#[test]
fn arena_add_log_target_null_callback_returns_zero_identifier() {
    let handle = arena_add_log_target(None, std::ptr::null_mut());
    assert_eq!(handle, 0);
}
