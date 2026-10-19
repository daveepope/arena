use arena_container::credential::new_password;

#[test]
fn new_password_calledtwice_returnsdifferentpasswords() {
    let first = new_password();
    let second = new_password();

    assert_ne!(first, second);
}

#[test]
fn new_password_anycall_satisfiesthecomplexityadependencydemands() {
    let password = new_password();

    assert!(password.chars().any(|c| c.is_ascii_uppercase()), "got: {password}");
    assert!(password.chars().any(|c| c.is_ascii_lowercase()), "got: {password}");
    assert!(password.chars().any(|c| c.is_ascii_digit()), "got: {password}");
    assert!(password.len() >= 8, "got: {password}");
}

#[test]
fn new_password_anycall_isalphanumericonly() {
    let password = new_password();

    assert!(password.chars().all(|c| c.is_ascii_alphanumeric()), "got: {password}");
}
