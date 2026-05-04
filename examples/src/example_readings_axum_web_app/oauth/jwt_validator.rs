use std::collections::HashMap;

use arena_oauth::AccessTokenClaims;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

pub struct JwksValidator {
    issuer: String,
    keys_by_kid: HashMap<String, DecodingKey>,
}

impl JwksValidator {
    pub async fn from_issuer(http: &reqwest::Client, issuer_base: &str) -> Result<Self, String> {
        let base = issuer_base.trim_end_matches('/');
        let jwks_url = format!("{base}/.well-known/jwks.json");
        let v: serde_json::Value = http
            .get(&jwks_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        let keys = v["keys"].as_array().ok_or("jwks: missing keys array")?;
        let mut keys_by_kid = HashMap::new();
        for k in keys {
            if k["kty"].as_str() != Some("RSA") {
                continue;
            }
            let kid = k["kid"].as_str().ok_or("jwks: rsa key missing kid")?;
            let n = k["n"].as_str().ok_or("jwks: rsa key missing n")?;
            let e = k["e"].as_str().ok_or("jwks: rsa key missing e")?;
            let dk = DecodingKey::from_rsa_components(n, e).map_err(|err| err.to_string())?;
            keys_by_kid.insert(kid.to_string(), dk);
        }
        if keys_by_kid.is_empty() {
            return Err("jwks: no usable RSA keys".into());
        }
        Ok(Self {
            issuer: base.to_string(),
            keys_by_kid,
        })
    }

    pub fn verify_access_token(&self, bearer_token: &str) -> Result<AccessTokenClaims, String> {
        let header = decode_header(bearer_token).map_err(|e| e.to_string())?;
        let kid = header.kid.as_deref().ok_or("jwt: missing kid header")?;
        let key = self
            .keys_by_kid
            .get(kid)
            .ok_or_else(|| format!("jwt: unknown kid {kid}"))?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        decode::<AccessTokenClaims>(bearer_token, key, &validation)
            .map(|d| d.claims)
            .map_err(|e| e.to_string())
    }
}
