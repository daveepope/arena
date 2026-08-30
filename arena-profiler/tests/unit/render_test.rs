use arena_profiler::render::{
    html_escape, is_wsl, render_folded_to_html, top_hotspots, RenderError, Severity,
};

fn temp_html_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .join(format!("arena-profiler-render-test-{name}-{}.html", std::process::id()))
}

#[test]
fn render_folded_to_html_single_stack_produces_embedded_svg() {
    let folded = "main;handler;compute 42\n";
    let output_path = temp_html_path("single-stack");

    render_folded_to_html(folded.as_bytes(), &output_path, false).unwrap();

    let report = std::fs::read_to_string(&output_path).unwrap();
    assert!(report.starts_with("<!DOCTYPE html>"));
    assert!(report.contains("<svg"));
    assert!(report.contains("compute"));
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn render_folded_to_html_multiple_stacks_produces_valid_report() {
    let folded = "main;foo 10\nmain;bar 5\nmain;foo;baz 3\n";
    let output_path = temp_html_path("multi-stack");

    render_folded_to_html(folded.as_bytes(), &output_path, false).unwrap();

    let report = std::fs::read_to_string(&output_path).unwrap();
    assert!(report.contains("<html"));
    assert!(report.contains("</html>"));
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn render_folded_to_html_hotspots_disabled_omits_hotspots_table() {
    let folded = "main;foo;compute 42\n";
    let output_path = temp_html_path("hotspots-disabled");

    render_folded_to_html(folded.as_bytes(), &output_path, false).unwrap();

    let report = std::fs::read_to_string(&output_path).unwrap();
    assert!(!report.contains("arena-profiler-hotspots"));
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn render_folded_to_html_hotspots_enabled_includes_ranked_table() {
    let folded = "main;foo;compute 42\nmain;bar;idle 8\n";
    let output_path = temp_html_path("hotspots-enabled");

    render_folded_to_html(folded.as_bytes(), &output_path, true).unwrap();

    let report = std::fs::read_to_string(&output_path).unwrap();
    assert!(report.contains("arena-profiler-hotspots"));
    assert!(report.contains("Top 2 Hotspots"));
    assert!(report.contains("compute"));
    assert!(report.contains("idle"));
    assert!(report.contains("severity-badge"));
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn render_folded_to_html_unwritable_output_path_returns_io_error() {
    let folded = "main;handler;compute 42\n";
    let bogus_path = std::path::Path::new("/nonexistent-dir/arena-profiler-render-test.html");

    let result = render_folded_to_html(folded.as_bytes(), bogus_path, false);

    assert!(matches!(result, Err(RenderError::Io(_))));
}

#[test]
fn render_error_display_io_includes_underlying_error() {
    let e = RenderError::Io(std::io::Error::other("missing"));

    assert!(e.to_string().contains("missing"));
}

#[test]
fn render_error_display_inferno_includes_message() {
    let e = RenderError::Inferno("bad folded stacks".to_string());

    assert!(e.to_string().contains("bad folded stacks"));
}

#[test]
fn render_error_from_io_error_wraps_as_io_variant() {
    let e: RenderError = std::io::Error::other("boom").into();

    assert!(matches!(e, RenderError::Io(_)));
}

#[test]
fn top_hotspots_ranks_by_leaf_self_time_descending() {
    let folded = "main;foo;compute 10\nmain;bar;compute 40\nmain;bar;idle 5\n";

    let hotspots = top_hotspots(folded, 10);

    assert_eq!(hotspots[0].function, "compute");
    assert_eq!(hotspots[0].self_count, 50);
    assert_eq!(hotspots[1].function, "idle");
    assert_eq!(hotspots[1].self_count, 5);
}

#[test]
fn top_hotspots_computes_self_percentage_of_total_samples() {
    let folded = "main;a 3\nmain;b 1\n";

    let hotspots = top_hotspots(folded, 10);

    let a = hotspots.iter().find(|h| h.function == "a").unwrap();
    assert!((a.self_pct - 75.0).abs() < f64::EPSILON);
}

#[test]
fn top_hotspots_limit_truncates_result() {
    let folded = "main;a 3\nmain;b 2\nmain;c 1\n";

    let hotspots = top_hotspots(folded, 2);

    assert_eq!(hotspots.len(), 2);
}

#[test]
fn top_hotspots_empty_folded_returns_empty() {
    let hotspots = top_hotspots("", 10);

    assert!(hotspots.is_empty());
}

#[test]
fn top_hotspots_assigns_severity_from_self_pct() {
    let folded = "main;hot 93\nmain;cold 7\n";

    let hotspots = top_hotspots(folded, 10);

    let hot = hotspots.iter().find(|h| h.function == "hot").unwrap();
    let cold = hotspots.iter().find(|h| h.function == "cold").unwrap();
    assert_eq!(hot.severity, Severity::Critical);
    assert_eq!(cold.severity, Severity::Medium);
}

#[test]
fn html_escape_reserved_characters_are_escaped() {
    assert_eq!(html_escape("a<b> && c"), "a&lt;b&gt; &amp;&amp; c");
}

#[test]
fn severity_for_self_pct_below_medium_threshold_returns_low() {
    assert_eq!(Severity::for_self_pct(4.9), Severity::Low);
}

#[test]
fn severity_for_self_pct_at_medium_threshold_returns_medium() {
    assert_eq!(Severity::for_self_pct(5.0), Severity::Medium);
}

#[test]
fn severity_for_self_pct_at_high_threshold_returns_high() {
    assert_eq!(Severity::for_self_pct(10.0), Severity::High);
}

#[test]
fn severity_for_self_pct_at_critical_threshold_returns_critical() {
    assert_eq!(Severity::for_self_pct(20.0), Severity::Critical);
}

#[test]
fn is_wsl_matches_wsl_distro_env_or_proc_version() {
    let expected = std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false);

    assert_eq!(is_wsl(), expected);
}
