#[path = "dispatcher_capture.rs"]
mod dispatcher_capture;

use arena_ffi::boundary::call_across_boundary;
use arena_ffi::{arena_add_log_target, arena_remove_log_target, ArenaLogLevel};
use dispatcher_capture::{
    collecting_callback, drain, record_emitted_within, records_emitted_within, RECORDED,
    TARGET_API_LOCK,
};

const STDERR_PROBE_CHILD: &str = "ARENA_FFI_PANIC_STDERR_PROBE_CHILD";
const STDERR_PROBE_TEST: &str =
    "install_panic_reporter_panic_inside_boundary_leaves_stderr_free_of_panic_text";

#[test]
fn on_event_arena_and_subject_spans_target_the_subject_logger() {
    let cases = [
        ("dependency", "orders-postgres", "arena.orders.dependency.orders-postgres"),
        ("component", "orders-api", "arena.orders.component.orders-api"),
        ("playbook", "orders-seed", "arena.orders.playbook.orders-seed"),
    ];

    for (kind, subject_id, expected) in cases {
        let marker = format!("namespace-subject-{kind}");
        let arena_id = String::from("orders");
        let subject_id = String::from(subject_id);
        let record = record_emitted_within(&marker, || {
            let arena = tracing::info_span!("arena", arena.id = %arena_id);
            let _arena = arena.enter();
            let subject = tracing::info_span!(
                "subject",
                arena.subject.kind = kind,
                arena.subject.id = %subject_id
            );
            let _subject = subject.enter();
            tracing::info!(target: "arena::ffi", "{}", marker);
        });

        assert_eq!(record.target, expected, "kind {kind}");
    }
}

#[test]
fn on_event_arena_span_only_targets_the_arena_logger() {
    let arena_id = String::from("orders");
    let record = record_emitted_within("namespace-arena", || {
        let arena = tracing::info_span!("arena", arena.id = %arena_id);
        let _arena = arena.enter();
        tracing::info!(target: "arena::ffi", "namespace-arena");
    });

    assert_eq!(record.target, "arena.orders");
}

#[test]
fn on_event_outside_any_arena_span_targets_the_root_logger() {
    let record = record_emitted_within("namespace-root", || {
        tracing::info!(target: "arena::ffi", "namespace-root");
    });

    assert_eq!(record.target, "arena");
}

#[test]
fn on_event_subject_span_without_an_arena_span_keeps_the_subject_segments() {
    let subject_id = String::from("orders-postgres");
    let record = record_emitted_within("namespace-orphan-subject", || {
        let subject = tracing::info_span!(
            "subject",
            arena.subject.kind = "dependency",
            arena.subject.id = %subject_id
        );
        let _subject = subject.enter();
        tracing::info!(target: "arena::ffi", "namespace-orphan-subject");
    });

    assert_eq!(record.target, "arena.dependency.orders-postgres");
}

#[test]
fn on_event_span_without_identity_inherits_the_enclosing_subject_logger() {
    let arena_id = String::from("orders");
    let subject_id = String::from("orders-postgres");
    let record = record_emitted_within("namespace-inherited", || {
        let arena = tracing::info_span!("arena", arena.id = %arena_id);
        let _arena = arena.enter();
        let subject = tracing::info_span!(
            "subject",
            arena.subject.kind = "dependency",
            arena.subject.id = %subject_id
        );
        let _subject = subject.enter();
        let unrelated = tracing::info_span!("readiness_attempt", attempt = 3);
        let _unrelated = unrelated.enter();
        tracing::info!(target: "arena::ffi", "namespace-inherited");
    });

    assert_eq!(record.target, "arena.orders.dependency.orders-postgres");
}

#[test]
fn on_event_nested_arena_span_drops_the_outer_subject() {
    let outer = String::from("outer");
    let inner = String::from("inner");
    let subject_id = String::from("outer-postgres");
    let record = record_emitted_within("namespace-nested-arena", || {
        let arena = tracing::info_span!("arena", arena.id = %outer);
        let _arena = arena.enter();
        let subject = tracing::info_span!(
            "subject",
            arena.subject.kind = "dependency",
            arena.subject.id = %subject_id
        );
        let _subject = subject.enter();
        let nested = tracing::info_span!("arena", arena.id = %inner);
        let _nested = nested.enter();
        tracing::info!(target: "arena::ffi", "namespace-nested-arena");
    });

    assert_eq!(record.target, "arena.inner");
}

#[test]
fn on_event_identifier_containing_dots_collapses_into_one_segment() {
    let arena_id = String::from("orders.v2");
    let subject_id = String::from("orders.postgres");
    let record = record_emitted_within("namespace-dotted", || {
        let arena = tracing::info_span!("arena", arena.id = %arena_id);
        let _arena = arena.enter();
        let subject = tracing::info_span!(
            "subject",
            arena.subject.kind = "dependency",
            arena.subject.id = %subject_id
        );
        let _subject = subject.enter();
        tracing::info!(target: "arena::ffi", "namespace-dotted");
    });

    assert_eq!(record.target, "arena.orders_v2.dependency.orders_postgres");
}

#[test]
fn on_event_blank_arena_identifier_falls_back_to_the_root_logger() {
    let arena_id = String::from("   ");
    let record = record_emitted_within("namespace-blank", || {
        let arena = tracing::info_span!("arena", arena.id = %arena_id);
        let _arena = arena.enter();
        tracing::info!(target: "arena::ffi", "namespace-blank");
    });

    assert_eq!(record.target, "arena");
}

#[test]
fn on_event_payload_carries_no_bracketed_target_prefix() {
    let arena_id = String::from("orders");
    let record = record_emitted_within("namespace-payload", || {
        let arena = tracing::info_span!("arena", arena.id = %arena_id);
        let _arena = arena.enter();
        tracing::info!(target: "arena::ffi", dependency = "orders-postgres", "namespace-payload");
    });

    assert_eq!(
        record.message,
        "namespace-payload | dependency=\"orders-postgres\""
    );
}

#[test]
fn install_panic_reporter_panic_inside_boundary_delivers_a_log_record() {
    let captured = records_emitted_within(|| {
        let outcome = call_across_boundary(|| panic!("boundary-panic-probe"));
        assert!(outcome.is_err());
    });

    let record = captured
        .iter()
        .find(|r| r.message.contains("boundary-panic-probe"))
        .unwrap_or_else(|| panic!("panic was not delivered to the target: {captured:?}"));
    assert_eq!(record.level, ArenaLogLevel::Error as i32);
    assert_eq!(record.target, "arena");
    assert!(
        record.message.contains("location="),
        "panic record should carry its location: {}",
        record.message
    );
}

#[test]
fn install_panic_reporter_panic_outside_boundary_delivers_no_log_record() {
    let captured = records_emitted_within(|| {
        let outcome = std::panic::catch_unwind(|| panic!("unbounded-panic-probe"));
        assert!(outcome.is_err());
    });

    assert!(
        captured
            .iter()
            .all(|r| !r.message.contains("unbounded-panic-probe")),
        "a panic raised outside the arena boundary must stay with the host hook: {captured:?}"
    );
}

#[test]
fn install_panic_reporter_panic_inside_boundary_leaves_stderr_free_of_panic_text() {
    if std::env::var(STDERR_PROBE_CHILD).is_ok() {
        let _g = TARGET_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drain(&RECORDED);
        let handle = arena_add_log_target(Some(collecting_callback), std::ptr::null_mut());
        assert_ne!(handle, 0);
        let outcome = call_across_boundary(|| panic!("stderr-panic-probe"));
        let captured = drain(&RECORDED);
        arena_remove_log_target(handle);
        assert!(outcome.is_err());
        assert!(
            captured
                .iter()
                .any(|r| r.message.contains("stderr-panic-probe")),
            "child run did not deliver the panic record: {captured:?}"
        );
        return;
    }

    let exe = std::env::current_exe().expect("current test binary path");
    let output = std::process::Command::new(exe)
        .args(["--exact", "--nocapture", STDERR_PROBE_TEST])
        .env(STDERR_PROBE_CHILD, "1")
        .output()
        .expect("re-run the test binary as a child process");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "child run failed:\n{stderr}\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !stderr.contains("panicked at"),
        "panic text reached stderr instead of the log target:\n{stderr}"
    );
}
