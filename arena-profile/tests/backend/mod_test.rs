use arena_profile::wait_bounded;
use std::process::Command;
use std::time::Duration;

#[test]
fn wait_bounded_child_outlives_budget_kills_and_returns_timed_out() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    let result = wait_bounded(&mut child, Duration::from_millis(100));

    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::TimedOut);
    assert!(child.try_wait().unwrap().is_some());
}