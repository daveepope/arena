use arbiter::{grade, report, score, Features, Grade};

const DEBT_CEILING: f64 = 100.0;

fn heavy_dump() -> Features {
    Features {
        bytes_delta: 50_000,
        files_delta: 20,
        complexity_introduced: 50,
        max_autonomy_run: 8,
        ..Features::default()
    }
}

#[test]
fn score_high_autonomy_low_engagement_returns_high() {
    let debt = score(&heavy_dump());
    assert!(debt > 30.0, "expected high debt, got {debt}");
}

fn moderate_dump() -> Features {
    Features {
        bytes_delta: 5_000,
        files_delta: 3,
        complexity_introduced: 5,
        max_autonomy_run: 4,
        ..Features::default()
    }
}

#[test]
fn score_engagement_reduces_debt() {
    let bare = moderate_dump();
    let mut engaged = bare.clone();
    engaged.probe_hits = 5;
    engaged.read_ops = 10;
    assert!(score(&engaged) < score(&bare));
}

#[test]
fn score_clamps_at_ceiling() {
    let debt = score(&Features {
        bytes_delta: 5_000_000,
        files_delta: 500,
        complexity_introduced: 500,
        max_autonomy_run: 50,
        ..Features::default()
    });
    assert_eq!(debt, DEBT_CEILING);
}

#[test]
fn score_no_activity_returns_zero() {
    assert_eq!(score(&Features::default()), 0.0);
}

#[test]
fn score_negative_volume_returns_zero() {
    let debt = score(&Features {
        bytes_delta: -20_000,
        files_delta: -30,
        complexity_introduced: -50,
        ..Features::default()
    });
    assert_eq!(debt, 0.0);
}

#[test]
fn score_deletions_reduce_debt() {
    let dump = score(&Features {
        bytes_delta: 48_000,
        files_delta: 35,
        complexity_introduced: 84,
        max_autonomy_run: 12,
        read_ops: 38,
        shell_ops: 1,
        ..Features::default()
    });
    let trimmed = score(&Features {
        bytes_delta: 8_000,
        files_delta: 8,
        complexity_introduced: 24,
        max_autonomy_run: 12,
        read_ops: 38,
        shell_ops: 1,
        probe_hits: 1,
        ..Features::default()
    });
    assert!(trimmed < dump, "trimmed {trimmed} should be below dump {dump}");
}

#[test]
fn grade_thresholds_return_expected_bands() {
    let cases = [
        (0.0, Grade::Low),
        (9.9, Grade::Low),
        (10.0, Grade::Moderate),
        (29.9, Grade::Moderate),
        (30.0, Grade::High),
        (59.9, Grade::High),
        (60.0, Grade::Severe),
        (100.0, Grade::Severe),
    ];
    for (debt, expected) in cases {
        assert_eq!(grade(debt), expected, "debt {debt}");
    }
}

#[test]
fn report_consistent_with_score_and_grade() {
    let built = report(heavy_dump());
    assert_eq!(built.debt, score(&built.features));
    assert_eq!(built.grade, grade(built.debt));
}
