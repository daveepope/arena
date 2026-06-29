use arbiter::{score, Features};

fn idle() -> Features {
    Features::default()
}

fn agent_code_dump() -> Features {
    Features {
        user_turns: 3,
        max_autonomy_run: 12,
        bytes_delta: 48_000,
        files_delta: 35,
        complexity_introduced: 84,
        read_ops: 38,
        shell_ops: 1,
        ..Features::default()
    }
}

fn after_deletions() -> Features {
    Features {
        user_turns: 3,
        max_autonomy_run: 12,
        bytes_delta: 8_000,
        files_delta: 8,
        complexity_introduced: 24,
        read_ops: 38,
        shell_ops: 1,
        probe_hits: 1,
        ..Features::default()
    }
}

fn after_probes() -> Features {
    Features {
        user_turns: 5,
        prompt_chars: 4_000,
        max_autonomy_run: 12,
        bytes_delta: 8_000,
        files_delta: 8,
        complexity_introduced: 24,
        read_ops: 38,
        shell_ops: 1,
        probe_hits: 8,
        ..Features::default()
    }
}

#[test]
fn trajectory_dump_raises_debt_above_idle() {
    assert!(score(&agent_code_dump()) > score(&idle()));
}

#[test]
fn trajectory_deletions_lower_debt_below_dump() {
    let dump = score(&agent_code_dump());
    let trimmed = score(&after_deletions());
    assert!(trimmed < dump, "trimmed {trimmed} should be below dump {dump}");
}

#[test]
fn trajectory_probes_lower_debt_below_deletions() {
    let trimmed = score(&after_deletions());
    let challenged = score(&after_probes());
    assert!(challenged < trimmed, "challenged {challenged} should be below trimmed {trimmed}");
}

#[test]
fn trajectory_longer_prompts_alone_do_not_reduce_debt() {
    let before = score(&after_deletions());
    let after = score(&after_probes());
    let mut prompts_only = after_deletions();
    prompts_only.prompt_chars = 20_000;
    prompts_only.user_turns = 12;
    let prompts_score = score(&prompts_only);
    assert!(
        (prompts_score - before).abs() < f64::EPSILON,
        "prompt volume alone changed debt: {before} -> {prompts_score}"
    );
    assert!(after < before);
}

#[test]
fn trajectory_cleanup_below_baseline_returns_zero_debt() {
    let cleanup = Features {
        bytes_delta: -20_000,
        files_delta: -30,
        complexity_introduced: -50,
        ..Features::default()
    };
    assert_eq!(score(&cleanup), 0.0);
}

#[test]
fn trajectory_stepwise_debt_series_moves_down() {
    let points = [
        score(&idle()),
        score(&agent_code_dump()),
        score(&after_deletions()),
        score(&after_probes()),
    ];
    assert!(points[1] > points[0]);
    assert!(points[2] < points[1]);
    assert!(points[3] < points[2]);
}
