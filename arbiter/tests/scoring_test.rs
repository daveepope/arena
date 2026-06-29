use arbiter::{grade, report, score, Features, Grade};

const DEBT_CEILING: f64 = 100.0;

fn features(spec_gap: f64, autonomy: usize) -> Features {
    Features {
        spec_gap,
        max_autonomy_run: autonomy,
        ..Features::default()
    }
}

#[test]
fn score_high_autonomy_low_engagement_returns_high() {
    let debt = score(&features(5.0, 8));
    assert!(debt > 30.0, "expected high debt, got {debt}");
}

#[test]
fn score_engagement_reduces_debt() {
    let bare = features(5.0, 8);
    let mut engaged = bare.clone();
    engaged.probe_hits = 5;
    engaged.read_ops = 10;
    assert!(score(&engaged) < score(&bare));
}

#[test]
fn score_clamps_at_ceiling() {
    let debt = score(&features(1_000.0, 50));
    assert_eq!(debt, DEBT_CEILING);
}

#[test]
fn score_no_activity_returns_zero() {
    assert_eq!(score(&Features::default()), 0.0);
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
    let built = report(features(5.0, 8));
    assert_eq!(built.debt, score(&built.features));
    assert_eq!(built.grade, grade(built.debt));
}
