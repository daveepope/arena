use arena_profile::render::{render_folded_to_html, RenderError};

fn temp_html_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("arena-profile-render-test-{name}-{}.html", std::process::id()))
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
    assert!(!report.contains("arena-profile-hotspots"));
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn render_folded_to_html_hotspots_enabled_includes_ranked_table() {
    let folded = "main;foo;compute 42\nmain;bar;idle 8\n";
    let output_path = temp_html_path("hotspots-enabled");

    render_folded_to_html(folded.as_bytes(), &output_path, true).unwrap();

    let report = std::fs::read_to_string(&output_path).unwrap();
    assert!(report.contains("arena-profile-hotspots"));
    assert!(report.contains("Top 2 Hotspots"));
    assert!(report.contains("compute"));
    assert!(report.contains("idle"));
    assert!(report.contains("severity-badge"));
    let _ = std::fs::remove_file(&output_path);
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
fn render_folded_to_html_unwritable_output_path_returns_io_error() {
    let folded = "main;handler;compute 42\n";
    let bogus_path = std::path::Path::new("/nonexistent-dir/arena-profile-render-test.html");

    let result = render_folded_to_html(folded.as_bytes(), bogus_path, false);

    assert!(matches!(result, Err(RenderError::Io(_))));
}
