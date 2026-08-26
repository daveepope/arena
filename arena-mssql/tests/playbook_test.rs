use arena_mssql::MssqlDependency;

#[test]
fn playbook_before_start_panics() {
    let dep = MssqlDependency::builder("playbook-before-start").build();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dep.playbook()));

    assert!(outcome.is_err());
}
