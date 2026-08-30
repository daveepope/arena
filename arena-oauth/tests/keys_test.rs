use arena_oauth::{IssuerConfig, OauthDependency, Provider};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use rsa::pkcs8::{DecodePrivateKey, EncodePublicKey, LineEnding};
use rsa::RsaPrivateKey;
use serde_json::{json, Value};

fn default_provider() -> Provider {
    Provider::Custom {
        issuer_path: Some(String::new()),
    }
}

fn decoding_key_for(dep: &OauthDependency, provider: &Provider) -> DecodingKey {
    let pem = dep.signing_key_pem_for(provider).expect("signing key pem");
    let private_key = RsaPrivateKey::from_pkcs8_pem(&pem).expect("parse pkcs8 pem");
    let public_pem = private_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("public key pem");
    DecodingKey::from_rsa_pem(public_pem.as_bytes()).expect("build decoding key")
}

#[test]
fn sign_claims_with_arbitrary_claims_produces_verifiable_token() {
    let dep = OauthDependency::builder("keys-sign-claims-verifiable")
        .with_http()
        .build();
    let claims = json!({
        "iss": "https://issuer.example",
        "sub": "test-subject",
        "custom_claim": "custom-value",
        "exp": 9_999_999_999u64,
    });
    let token = dep
        .sign_claims(&default_provider(), &claims)
        .expect("sign claims");

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&["https://issuer.example"]);
    let decoded = decode::<Value>(&token, &decoding_key_for(&dep, &default_provider()), &validation)
        .expect("decode signed token");
    assert_eq!(decoded.claims["custom_claim"], "custom-value");
}

#[test]
fn sign_claims_with_claims_omitting_iss_still_signs() {
    let dep = OauthDependency::builder("keys-sign-claims-no-iss")
        .with_http()
        .build();
    let claims = json!({ "sub": "test-subject", "exp": 9_999_999_999u64 });
    let token = dep
        .sign_claims(&default_provider(), &claims)
        .expect("sign claims without iss");

    let validation = Validation::new(Algorithm::RS256);
    let decoded = decode::<Value>(&token, &decoding_key_for(&dep, &default_provider()), &validation)
        .expect("decode signed token without iss");
    assert!(decoded.claims.get("iss").is_none());
}

#[test]
fn sign_claims_unregistered_provider_returns_err() {
    let dep = OauthDependency::builder("keys-sign-claims-unregistered-provider")
        .with_http()
        .build();
    let claims = json!({ "sub": "test-subject" });
    let err = dep
        .sign_claims(&Provider::Okta, &claims)
        .expect_err("okta is not registered on a default single-issuer dependency");
    assert!(err.contains("no issuer registered"));
}

#[test]
fn private_key_pkcs8_pem_roundtrips_through_from_pkcs8_pem() {
    let original = OauthDependency::builder("keys-pem-roundtrip-source")
        .with_http()
        .build();
    let pem = original
        .signing_key_pem_for(&default_provider())
        .expect("signing key pem");

    let roundtripped = OauthDependency::builder("keys-pem-roundtrip-target")
        .with_http()
        .with_issuer(IssuerConfig::new().with_rsa_pkcs8_pem(pem.clone()))
        .build();
    let pem_again = roundtripped
        .signing_key_pem_for(&default_provider())
        .expect("signing key pem again");

    assert_eq!(pem, pem_again);
}
