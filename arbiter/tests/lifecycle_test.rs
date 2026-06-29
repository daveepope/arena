mod harness_factory;

use std::fs;
use std::path::PathBuf;

use arbiter::AgentHarness;
use harness_factory::HarnessFactory;

fn empty_workspace(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "arbiter-workspace-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn start_then_calculate_reflects_session() {
    let selector = AgentHarness::Cursor;
    let path = HarnessFactory::new(selector)
        .session()
        .user("why this approach")
        .write(300)
        .to_file();
    let workspace = empty_workspace("reflects");

    let mut harness = selector.open_in(path.clone(), workspace.clone());
    harness.start().unwrap();
    let report = harness.calculate();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(report.features.user_turns, 1);
    assert_eq!(report.features.edit_bytes, 300);
    assert!(report.features.probe_hits >= 1);
    assert!(report.debt > 0.0);
}

#[test]
fn calculate_empty_session_returns_zero_debt() {
    let path = HarnessFactory::cursor().session().to_file();
    let workspace = empty_workspace("empty");

    let mut harness = AgentHarness::Cursor.open_in(path.clone(), workspace.clone());
    harness.start().unwrap();
    let report = harness.calculate();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(report.debt, 0.0);
    assert_eq!(report.features.user_turns, 0);
}

#[test]
fn calculate_appends_timestamped_debt_samples() {
    let selector = AgentHarness::Cursor;
    let path = HarnessFactory::new(selector)
        .session()
        .user("why this approach")
        .write(300)
        .to_file();
    let workspace = empty_workspace("series");

    let mut harness = selector.open_in(path.clone(), workspace.clone());
    harness.start().unwrap();
    let first = harness.calculate();
    let second = harness.calculate();
    let series = harness.debt_series();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(series.len(), 2);
    assert_eq!(series[0].debt, first.debt);
    assert_eq!(series[1].debt, second.debt);
    assert!(series[1].at_ms >= series[0].at_ms);
}
