use arena_profiler::backend::perf::{no_samples_message, perf_event_paranoid};

#[test]
fn no_samples_message_explains_cause_is_permission_not_an_idle_program() {
    let message = no_samples_message();

    assert!(message.contains("perf recorded no samples"));
    assert!(message.contains("CPU performance counters"));
}

#[test]
fn no_samples_message_includes_both_remedies() {
    let message = no_samples_message();

    assert!(message.contains("setcap cap_perfmon+ep"));
    assert!(message.contains("kernel.perf_event_paranoid=1"));
}

#[test]
fn no_samples_message_readable_paranoid_level_reports_it() {
    let message = no_samples_message();

    match perf_event_paranoid() {
        Some(level) => {
            assert!(message.contains("/proc/sys/kernel/perf_event_paranoid"));
            assert!(message.contains(&level.to_string()));
        }
        None => assert!(!message.contains("is currently")),
    }
}

#[test]
fn no_samples_message_permissive_paranoid_level_points_at_missing_capability() {
    let Some(level) = perf_event_paranoid() else {
        return;
    };

    let message = no_samples_message();

    if level <= 2 {
        assert!(message.contains("missing CAP_PERFMON"));
    } else {
        assert!(message.contains("or lower"));
    }
}
