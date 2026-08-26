use arena_oauth::{ensure_scopes, validate_scopes, AccessTokenClaims, TokenError};
use std::error::Error;

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

#[test]
fn token_error_display_formats_each_variant() {
    let missing_scope = TokenError::MissingScope;
    assert_eq!(missing_scope.to_string(), "missing scope claim");

    let not_running = TokenError::NotRunning;
    assert_eq!(not_running.to_string(), "oauth dependency is not running");

    let insufficient = TokenError::InsufficientScope {
        required: vec!["admin".to_string()],
        granted: vec!["openid".to_string()],
    };
    let rendered = insufficient.to_string();
    assert!(rendered.contains("insufficient scope"));
    assert!(rendered.contains("admin"));
    assert!(rendered.contains("openid"));
}

#[test]
fn token_error_source_present_only_for_jwt_variant() {
    assert!(TokenError::NotRunning.source().is_none());
    assert!(TokenError::MissingScope.source().is_none());
    assert!(TokenError::InsufficientScope {
        required: vec![],
        granted: vec![],
    }
    .source()
    .is_none());
}

#[test]
fn token_error_debug_reflects_variant_name() {
    let debug = format!("{:?}", TokenError::NotRunning);
    assert_eq!(debug, "NotRunning");
}
