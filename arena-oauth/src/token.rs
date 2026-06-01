use jsonwebtoken::{decode, encode, Algorithm, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::keys::RsaKeyPair;

#[derive(Debug)]
pub enum TokenError {
    Jwt(jsonwebtoken::errors::Error),
    MissingScope,
    InsufficientScope {
        required: Vec<String>,
        granted: Vec<String>,
    },
    NotRunning,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::Jwt(e) => write!(f, "jwt: {e}"),
            TokenError::MissingScope => write!(f, "missing scope claim"),
            TokenError::InsufficientScope { required, granted } => {
                write!(f, "insufficient scope: need {required:?}, have {granted:?}")
            }
            TokenError::NotRunning => write!(f, "oauth dependency is not running"),
        }
    }
}

impl std::error::Error for TokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TokenError::Jwt(e) => Some(e),
            _ => None,
        }
    }
}

impl From<jsonwebtoken::errors::Error> for TokenError {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        TokenError::Jwt(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub iss: String,
    pub sub: String,
    pub scope: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

pub fn issue_access_token(
    keys: &RsaKeyPair,
    issuer: &str,
    sub: &str,
    scopes: &[String],
    ttl_secs: u64,
) -> Result<String, TokenError> {
    let now = jsonwebtoken::get_current_timestamp();
    let exp = now.saturating_add(ttl_secs);
    let scope = if scopes.is_empty() {
        None
    } else {
        Some(scopes.join(" "))
    };
    let claims = AccessTokenClaims {
        iss: issuer.to_string(),
        sub: sub.to_string(),
        scope,
        exp: exp as usize,
        iat: now as usize,
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(keys.kid().to_string());
    let token = encode(&header, &claims, keys.encoding_key())?;
    Ok(token)
}

pub fn verify_access_token(
    token: &str,
    keys: &RsaKeyPair,
    issuer: &str,
) -> Result<AccessTokenClaims, TokenError> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[issuer]);
    validation.validate_exp = true;
    let data = decode::<AccessTokenClaims>(token, keys.decoding_key(), &validation)?;
    Ok(data.claims)
}

pub fn validate_scopes(granted: &str, required: &[&str]) -> bool {
    if required.is_empty() {
        return true;
    }
    let set: std::collections::HashSet<&str> = granted.split_whitespace().collect();
    required.iter().all(|s| set.contains(s))
}

pub fn ensure_scopes(claims: &AccessTokenClaims, required: &[&str]) -> Result<(), TokenError> {
    let granted = claims.scope.as_deref().ok_or(TokenError::MissingScope)?;
    if validate_scopes(granted, required) {
        return Ok(());
    }
    let granted_list: Vec<String> = granted.split_whitespace().map(String::from).collect();
    let required_list: Vec<String> = required.iter().map(|s| (*s).to_string()).collect();
    Err(TokenError::InsufficientScope {
        required: required_list,
        granted: granted_list,
    })
}
