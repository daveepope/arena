use arena_profile::render::{render_folded, top_hotspots, RenderError, Severity};

#[test]
fn render_folded_single_stack_returns_svg_naming_the_frame() {
    let rendered = render_folded("main;handler;compute 42\n").unwrap();

    assert!(rendered.svg.contains("<svg"));
    assert!(rendered.svg.contains("compute"));
}

#[test]
fn render_folded_multiple_stacks_returns_one_closed_svg_document() {
    let rendered = render_folded("main;foo 10\nmain;bar 5\nmain;foo;baz 3\n").unwrap();

    assert!(rendered.svg.contains("<svg"));
    assert!(rendered.svg.trim_end().ends_with("</svg>"));
}

#[test]
fn render_folded_any_input_returns_ranked_hotspots_alongside_the_svg() {
    let rendered = render_folded("main;foo;compute 42\nmain;bar;idle 8\n").unwrap();

    assert_eq!(rendered.hotspots.len(), 2);
    assert_eq!(rendered.hotspots[0].function, "compute");
    assert_eq!(rendered.hotspots[1].function, "idle");
}

#[test]
fn render_folded_empty_input_returns_inferno_error() {
    let result = render_folded("");

    assert!(matches!(result, Err(RenderError::Inferno(_))));
}

#[test]
fn render_error_display_io_includes_underlying_error() {
    let e = RenderError::Io(std::io::Error::other("missing"));

    assert!(e.to_string().contains("missing"));
}

#[test]
fn render_error_display_not_utf8_includes_underlying_error() {
    let invalid = String::from_utf8(vec![0xff]).unwrap_err();
    let e = RenderError::NotUtf8(invalid);

    assert!(e.to_string().contains("utf-8"));
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
fn top_hotspots_assigns_severity_from_self_pct() {
    let folded = "main;hot 93\nmain;cold 7\n";

    let hotspots = top_hotspots(folded, 10);

    let hot = hotspots.iter().find(|h| h.function == "hot").unwrap();
    let cold = hotspots.iter().find(|h| h.function == "cold").unwrap();
    assert_eq!(hot.severity, Severity::Critical);
    assert_eq!(cold.severity, Severity::Medium);
}

#[test]
fn render_folded_hotspots_spanning_all_bands_carry_each_severity() {
    let rendered =
        render_folded("main;critical_fn 60\nmain;high_fn 15\nmain;medium_fn 7\nmain;low_fn 3\n")
            .unwrap();

    let severity_of = |name: &str| {
        rendered
            .hotspots
            .iter()
            .find(|hotspot| hotspot.function == name)
            .map(|hotspot| hotspot.severity)
            .unwrap_or_else(|| panic!("missing hotspot {name}"))
    };

    assert_eq!(severity_of("critical_fn"), Severity::Critical);
    assert_eq!(severity_of("high_fn"), Severity::High);
    assert_eq!(severity_of("medium_fn"), Severity::Medium);
    assert_eq!(severity_of("low_fn"), Severity::Low);
}

#[test]
fn severity_labels_colours_and_descriptions_are_distinct_per_band() {
    let bands = [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
    ];

    for band in bands {
        assert!(!band.label().is_empty());
        assert!(band.badge_color().starts_with('#'));
        assert!(band.description().contains("samples"));
    }

    let labels: Vec<&str> = bands.iter().map(|band| band.label()).collect();
    let colours: Vec<&str> = bands.iter().map(|band| band.badge_color()).collect();
    assert_eq!(labels.len(), labels.iter().collect::<std::collections::HashSet<_>>().len());
    assert_eq!(colours.len(), colours.iter().collect::<std::collections::HashSet<_>>().len());
}

#[test]
fn top_hotspots_lines_without_a_count_are_skipped() {
    let folded = "main;ok 10\nnocountatall\nmain;bad notanumber\n";

    let hotspots = top_hotspots(folded, 10);

    assert_eq!(hotspots.len(), 1);
    assert_eq!(hotspots[0].function, "ok");
}

#[test]
fn top_hotspots_all_counts_zero_returns_zero_percentage() {
    let folded = "main;idle 0\n";

    let hotspots = top_hotspots(folded, 10);

    assert_eq!(hotspots.len(), 1);
    assert_eq!(hotspots[0].self_count, 0);
    assert_eq!(hotspots[0].self_pct, 0.0);
    assert_eq!(hotspots[0].severity, Severity::Low);
}
