#[test]
fn log_line_handles_all_level_markers_without_panicking() {
    arena_container::logging::log_line("test-component", "2024-01-01 ERROR something broke");
    arena_container::logging::log_line("test-component", "2024-01-01 WARN something odd");
    arena_container::logging::log_line("test-component", "2024-01-01 DEBUG details");
    arena_container::logging::log_line("test-component", "2024-01-01 TRACE fine details");
    arena_container::logging::log_line("test-component", "plain line with no level marker");
}
