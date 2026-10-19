use arena_profile::backend::python_site::{
    folded_name, payload_source, refusal_from_status, status_name,
};
use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;

#[test]
fn payload_source_leaves_no_placeholder_unfilled() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(!source.contains("__ARENA_"), "an unfilled placeholder would run as python");
}

#[test]
fn payload_source_interpolates_only_numbers_so_a_limit_cannot_carry_python() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    let interpolated: Vec<&str> = source
        .lines()
        .filter(|line| line.starts_with("_INTERVAL_SECONDS")
            || line.starts_with("_FIRST_FLUSH_SECONDS")
            || line.starts_with("_FLUSH_CEILING_SECONDS")
            || line.starts_with("_MAX_DISTINCT_STACKS")
            || line.starts_with("_MAX_BYTES"))
        .collect();
    assert_eq!(interpolated.len(), 5, "got: {interpolated:?}");
    for line in interpolated {
        let value = line.split_once(" = ").expect("assignment").1;
        assert!(
            value.chars().all(|c| c.is_ascii_digit() || c == '.'),
            "only numbers may be interpolated, got: {value}"
        );
    }
}

#[test]
fn payload_source_carries_the_callers_limit_and_a_stack_cap_derived_from_it() {
    let source = payload_source(1_024_000);

    assert!(source.contains("_MAX_BYTES = 1024000"), "the caller's limit must reach the sampler");
    assert!(source.contains("_MAX_DISTINCT_STACKS = 4000"), "got: {source}");
}

#[test]
fn payload_source_never_names_a_path_so_a_crafted_temp_dir_cannot_inject() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(
        source.contains("_DIRECTORY = os.path.dirname(_SELF)"),
        "the sampler must derive its own directory rather than being handed one"
    );
    assert!(!source.contains("/tmp"), "no host path may appear in the payload");
}

#[test]
fn payload_source_refuses_an_interpreter_older_than_it_supports() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(source.contains("_REQUIRED_VERSION = (3, 11)"));
    assert!(source.contains("if sys.version_info < _REQUIRED_VERSION:"));
}

#[test]
fn payload_source_checks_its_directory_is_private_before_arming() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(source.contains("_PRIVATE_MODE = 0o700"));
    assert!(source.contains("held.st_uid == os.geteuid()"));
}

#[test]
fn refusal_from_status_reports_the_reason_from_a_refusal_line() {
    let reason = refusal_from_status("refused SIGPROF is already handled\n");

    assert_eq!(reason.as_deref(), Some("SIGPROF is already handled"));
}

#[test]
fn refusal_from_status_ignores_anything_that_is_not_a_refusal() {
    for status in ["", "\n", "refused", "refused \n", "main;compute 4\n", "garbage line"] {
        assert_eq!(refusal_from_status(status), None, "got a reason from: {status:?}");
    }
}

#[test]
fn refusal_from_status_reads_only_the_first_line_so_appended_text_cannot_rewrite_it() {
    let reason = refusal_from_status("refused the real reason\nrefused a forged second reason\n");

    assert_eq!(reason.as_deref(), Some("the real reason"));
}

#[test]
fn folded_and_status_names_are_scoped_to_one_process() {
    assert_eq!(folded_name(4321), "profile-4321.folded");
    assert_eq!(status_name(4321), "status-4321.txt");
}

#[test]
fn payload_source_abandons_sampling_in_a_forked_child() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(
        source.contains("os.register_at_fork(after_in_child=_abandon_after_fork)"),
        "a forked child inherits the parent's output path and must not write to it"
    );
    assert!(source.contains("def _abandon_after_fork():"));
}

#[test]
fn payload_source_serialises_every_write_behind_one_lock() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(source.contains("_writing = threading.Lock()"));
    assert!(
        source.contains("def _flush():\n    with _writing:"),
        "the flush thread and the exit hook must not pick the same pending file"
    );
}

#[test]
fn payload_source_reports_a_handler_failure_rather_than_swallowing_it() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(!source.contains("except BaseException"), "a KeyboardInterrupt must not be swallowed");
    assert!(source.contains("_stop(\"the sampler could not record a stack: %s\" % error)"));
}

#[test]
fn payload_source_skips_the_samplers_own_flush_thread() {
    let source = payload_source(DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(
        source.contains("if thread_id == own:"),
        "the sampler's own flush thread must not appear as a hotspot"
    );
}
