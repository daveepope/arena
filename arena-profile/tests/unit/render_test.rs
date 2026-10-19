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

#[test]
fn top_application_hotspots_runtime_leaf_credits_the_frame_that_called_it() {
    let folded = "main;handler;compute 40\nmain;UNMANAGED_CODE_TIME 400\nmain;java.lang.Thread.run 50\n";

    let application = arena_profile::render::top_application_hotspots(folded, 10);

    let functions: Vec<&str> = application.iter().map(|h| h.function.as_str()).collect();
    assert_eq!(functions, vec!["main", "compute"]);
    assert_eq!(application[0].self_count, 450);
}

#[test]
fn top_application_hotspots_frame_without_runtime_descendants_keeps_its_share() {
    let folded = "main;handler;compute 40\nmain;UNMANAGED_CODE_TIME 360\n";

    let all = arena_profile::render::top_hotspots(folded, 10);
    let application = arena_profile::render::top_application_hotspots(folded, 10);

    let compute_in_all = all.iter().find(|h| h.function == "compute").expect("compute");
    let compute_in_application = application
        .iter()
        .find(|h| h.function == "compute")
        .expect("compute");
    assert_eq!(compute_in_all.self_count, 40);
    assert_eq!(compute_in_all.self_pct, compute_in_application.self_pct);
    assert_eq!(compute_in_all.severity, compute_in_application.severity);
}

#[test]
fn top_application_hotspots_frame_with_runtime_descendants_reads_higher_than_the_leaf_ranking() {
    let folded = "main;handler;compute 40\nmain;handler;compute;UNMANAGED_CODE_TIME 360\n";

    let all = arena_profile::render::top_hotspots(folded, 10);
    let application = arena_profile::render::top_application_hotspots(folded, 10);

    let compute_in_all = all.iter().find(|h| h.function == "compute").expect("compute");
    let compute_in_application = application
        .iter()
        .find(|h| h.function == "compute")
        .expect("compute");
    assert_eq!(compute_in_all.self_count, 40);
    assert_eq!(compute_in_application.self_count, 400);
    assert!(compute_in_application.self_pct > compute_in_all.self_pct);
}

#[test]
fn top_application_hotspots_stack_of_only_runtime_frames_is_left_out() {
    let folded = "UNMANAGED_CODE_TIME 400\njava.lang.Thread.run 100\n";

    let application = arena_profile::render::top_application_hotspots(folded, 10);

    assert!(application.is_empty());
}

#[test]
fn top_hotspots_runtime_frames_are_kept() {
    let folded = "main;handler;compute 40\nmain;UNMANAGED_CODE_TIME 400\n";

    let all = arena_profile::render::top_hotspots(folded, 10);

    let functions: Vec<&str> = all.iter().map(|h| h.function.as_str()).collect();
    assert_eq!(functions, vec!["UNMANAGED_CODE_TIME", "compute"]);
}

#[test]
fn render_folded_any_profile_carries_both_rankings() {
    let folded = "main;handler;compute 40\nmain;CPU_TIME 10\n";

    let rendered = arena_profile::render_folded(folded).expect("render");

    let all: Vec<&str> = rendered.hotspots.iter().map(|h| h.function.as_str()).collect();
    let application: Vec<&str> = rendered
        .application_hotspots
        .iter()
        .map(|h| h.function.as_str())
        .collect();
    assert_eq!(all, vec!["compute", "CPU_TIME"]);
    assert_eq!(application, vec!["compute", "main"]);
}

#[test]
fn is_runtime_frame_frames_the_runtimes_report_are_matched() {
    for frame in [
        "UNMANAGED_CODE_TIME",
        "CPU_TIME",
        "System.Threading.LowLevelLifoSemaphore.Wait(int32,bool)",
        "System.IO.FileSystemWatcher+RunningInstance.AddDirectoryWatchUnlocked(class WatchedDirectory)",
        "Thread (1549961) (.NET ThreadPool)",
        "IndexSetIterator::advance_and_next",
        "PhaseChaitin::elide_copy",
        "SymbolTable::do_lookup",
        "java.lang.Thread.run",
        "__tls_get_addr",
        "/usr/lib/x86_64-linux-gnu/libz.so.1.3.1",
        "_worker (concurrent/futures/thread.py:90)",
        "run (asyncio/runners.py:118)",
        "_call_with_frames_removed (<frozen importlib._bootstrap>:488)",
        "read (ssl.py:1103)",
        "readinto (socket.py:720)",
    ] {
        assert!(
            arena_profile::render::is_runtime_frame(frame),
            "should be a runtime frame: {frame}"
        );
    }
}

#[test]
fn is_runtime_frame_application_frames_are_kept() {
    for frame in [
        "compute_reading",
        "arena.examples.readings.ReadingsController.create",
        "ArenaExamples.Readings.ReadingsController.Create()",
        "create_reading (app/api/readings.py:42)",
        "Readings::insert",
        "com.app.OrderService.sortByPrice",
        "__init__ (app/models.py:10)",
        "libcompute_total",
    ] {
        assert!(
            !arena_profile::render::is_runtime_frame(frame),
            "should be an application frame: {frame}"
        );
    }
}

