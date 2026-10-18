use arena_container::credential::new_password;

#[test]
fn new_password_sameprefixtwice_returnsdifferentpasswords() {
    let first = new_password("arena");
    let second = new_password("arena");

    assert_ne!(first, second);
}

#[test]
fn new_password_anyprefix_carriesitandthecomplexitycharacters() {
    let password = new_password("arenaadmin");

    assert!(password.starts_with("arenaadmin"), "got: {password}");
    assert!(password.ends_with("Aa1!"), "got: {password}");
    assert!(password.len() > "arenaadmin".len() + 4, "got: {password}");
}

#[test]
fn new_password_anyprefix_isnotaliteralinthetree() {
    let password = new_password("arena");

    assert!(password.chars().any(|c| c.is_ascii_digit()));
    assert!(password.chars().any(|c| c.is_ascii_uppercase()));
    assert!(password.chars().any(|c| c.is_ascii_lowercase()));
}
