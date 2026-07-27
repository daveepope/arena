use arena_profile::render::render_folded_to_html;

fn temp_html_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("arena-profile-render-test-{name}-{}.html", std::process::id()))
}

#[test]
fn render_folded_to_html_single_stack_produces_embedded_svg() {
    let folded = "main;handler;compute 42\n";
    let output_path = temp_html_path("single-stack");

    render_folded_to_html(folded.as_bytes(), &output_path).unwrap();

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

    render_folded_to_html(folded.as_bytes(), &output_path).unwrap();

    let report = std::fs::read_to_string(&output_path).unwrap();
    assert!(report.contains("<html"));
    assert!(report.contains("</html>"));
    let _ = std::fs::remove_file(&output_path);
}
