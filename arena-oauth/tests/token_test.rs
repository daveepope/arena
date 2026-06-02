use arena_oauth::{ensure_scopes, validate_scopes, AccessTokenClaims, TokenError};

fn sample_claims(scope: Option<&str>) -> AccessTokenClaims {
    AccessTokenClaims {
        iss: "https://issuer.example".into(),
        sub: "sub-1".into(),
        scope: scope.map(String::from),
        exp: 9_999_999_999,
        iat: 1,
    }
}

#[test]
fn validate_scopes_empty_required_always_ok() {
    assert!(validate_scopes("openid profile", &[]));
    assert!(validate_scopes("", &[]));
}

#[test]
fn validate_scopes_single_required() {
    assert!(validate_scopes("openid profile", &["openid"]));
    assert!(!validate_scopes("profile", &["openid"]));
    assert!(validate_scopes("a b c", &["b"]));
}

#[test]
fn validate_scopes_all_required_must_be_present() {
    assert!(validate_scopes("read write", &["read", "write"]));
    assert!(!validate_scopes("read", &["read", "write"]));
}

#[test]
fn ensure_scopes_err_when_scope_claim_missing() {
    let claims = sample_claims(None);
    let e = ensure_scopes(&claims, &["openid"]).unwrap_err();
    assert!(matches!(e, TokenError::MissingScope));
}

#[test]
fn ensure_scopes_err_when_granted_does_not_include_required() {
    let claims = sample_claims(Some("openid"));
    let e = ensure_scopes(&claims, &["readings"]).unwrap_err();
    match e {
        TokenError::InsufficientScope { required, granted } => {
            assert_eq!(required, vec!["readings".to_string()]);
            assert!(granted.contains(&"openid".to_string()));
        }
        other => panic!("expected InsufficientScope, got {other:?}"),
    }
}

#[test]
fn ensure_scopes_ok_when_granted_superset() {
    let claims = sample_claims(Some("openid profile email"));
    ensure_scopes(&claims, &["openid", "profile"]).unwrap();
}
