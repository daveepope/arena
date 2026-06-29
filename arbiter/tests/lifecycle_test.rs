mod harness_factory;

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

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

fn workspace_with_src(tag: &str) -> PathBuf {
    let root = empty_workspace(tag);
    fs::create_dir_all(root.join("src")).unwrap();
    root
}

fn wait_for_disk() {
    std::thread::sleep(Duration::from_millis(200));
}

#[test]
fn calculate_disk_dump_raises_metrics_then_deletions_restore_baseline() {
    let path = HarnessFactory::cursor().session().to_file();
    let workspace = workspace_with_src("disk-traj");
    let src = workspace.join("src");

    let mut harness = AgentHarness::Cursor.open_in(path.clone(), workspace.clone());
    harness.start().unwrap();

    let idle = harness.calculate();
    assert_eq!(idle.debt, 0.0);
    assert_eq!(idle.features.bytes_delta, 0);
    assert_eq!(idle.features.files_delta, 0);
    assert_eq!(idle.features.complexity_introduced, 0);

    fs::write(
        src.join("Alpha.java"),
        "class Alpha { void a(int x) { if (x > 0) {} } }",
    )
    .unwrap();
    fs::write(
        src.join("Beta.java"),
        "class Beta { void b(int x) { if (x > 0) {} if (x < 0) {} } }",
    )
    .unwrap();
    fs::write(
        src.join("Gamma.java"),
        "class Gamma { void c(boolean x) { if (x) {} } }",
    )
    .unwrap();
    wait_for_disk();

    let dump = harness.calculate();
    assert!(dump.features.bytes_delta > 0, "bytes_delta {}", dump.features.bytes_delta);
    assert_eq!(dump.features.files_delta, 3);
    assert!(dump.features.complexity_introduced > 0);
    assert!(dump.debt > idle.debt);
    assert!(!harness.complexity_deltas().is_empty());

    fs::remove_file(src.join("Alpha.java")).unwrap();
    fs::remove_file(src.join("Beta.java")).unwrap();
    fs::remove_file(src.join("Gamma.java")).unwrap();
    wait_for_disk();

    let trimmed = harness.calculate();
    assert!(trimmed.features.bytes_delta < dump.features.bytes_delta);
    assert!(trimmed.features.files_delta < dump.features.files_delta);
    assert!(trimmed.features.complexity_introduced < dump.features.complexity_introduced);
    assert!(trimmed.debt < dump.debt);
    assert_eq!(trimmed.features.bytes_delta, 0);
    assert_eq!(trimmed.features.files_delta, 0);
    assert_eq!(trimmed.features.complexity_introduced, 0);
    assert_eq!(trimmed.debt, 0.0);
    assert!(harness.complexity_deltas().is_empty());

    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();
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
    let source = workspace.join("app.rs");
    fs::write(&source, "fn main() {}").unwrap();

    let mut harness = selector.open_in(path.clone(), workspace.clone());
    harness.start().unwrap();
    fs::write(
        &source,
        "fn main() { if true {} if false {} if true {} }",
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(150));
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
fn calculate_picks_up_appended_user_turn_without_notify() {
    use std::io::Write;

    let path = HarnessFactory::cursor().session().user("first").to_file();
    let workspace = empty_workspace("append");

    let mut harness = AgentHarness::Cursor.open_in(path.clone(), workspace.clone());
    harness.start().unwrap();
    assert_eq!(harness.calculate().features.user_turns, 1);

    let line = HarnessFactory::cursor().user_line("second why");
    let mut file = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
    write!(file, "\n{line}\n").unwrap();

    let report = harness.calculate();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(report.features.user_turns, 2);
    assert!(report.features.probe_hits >= 1);
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
