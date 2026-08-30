use jsonwebtoken::{encode, Algorithm, DecodingKey, EncodingKey, Header};
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, RsaPrivateKey, RsaPublicKey};
use serde_json::json;

#[derive(Clone)]
pub(crate) struct RsaKeyPair {
    private_key: RsaPrivateKey,
    encoding: EncodingKey,
    decoding: DecodingKey,
    kid: String,
}

impl RsaKeyPair {
    pub(crate) const DEFAULT_KID: &'static str = "arena-oauth-1";

    pub(crate) fn generate() -> Result<Self, String> {
        Self::generate_with_kid(Self::DEFAULT_KID)
    }

    pub(crate) fn generate_with_kid(kid: impl Into<String>) -> Result<Self, String> {
        let mut rng = rand::thread_rng();
        let private_key =
            RsaPrivateKey::new(&mut rng, 2048).map_err(|e: rsa::Error| e.to_string())?;
        Self::from_private_key(private_key, kid)
    }

    pub(crate) fn from_pkcs8_pem(pem: &str, kid: impl Into<String>) -> Result<Self, String> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(pem).map_err(|e| e.to_string())?;
        Self::from_private_key(private_key, kid)
    }

    fn from_private_key(
        private_key: RsaPrivateKey,
        kid: impl Into<String>,
    ) -> Result<Self, String> {
        let pem = private_key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| e.to_string())?;
        let encoding = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| e.to_string())?;
        let public_pem = private_key
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| e.to_string())?;
        let decoding =
            DecodingKey::from_rsa_pem(public_pem.as_bytes()).map_err(|e| e.to_string())?;
        Ok(Self {
            private_key,
            encoding,
            decoding,
            kid: kid.into(),
        })
    }

    pub(crate) fn encoding_key(&self) -> &EncodingKey {
        &self.encoding
    }

    pub(crate) fn decoding_key(&self) -> &DecodingKey {
        &self.decoding
    }

    pub(crate) fn kid(&self) -> &str {
        &self.kid
    }

    pub(crate) fn public_key(&self) -> RsaPublicKey {
        self.private_key.to_public_key()
    }

    pub fn private_key_pkcs8_pem(&self) -> Result<String, String> {
        self.private_key
            .to_pkcs8_pem(LineEnding::LF)
            .map(|pem| pem.as_str().to_owned())
            .map_err(|e| e.to_string())
    }

    pub fn sign_claims(&self, claims: &serde_json::Value) -> Result<String, String> {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());
        encode(&header, claims, &self.encoding).map_err(|e| e.to_string())
    }

    pub(crate) fn jwks_json(&self) -> serde_json::Value {
        let pk = self.public_key();
        let n = bigint_to_b64url(pk.n());
        let e = bigint_to_b64url(pk.e());
        json!({
            "keys": [{
                "kty": "RSA",
                "kid": self.kid,
                "use": "sig",
                "alg": "RS256",
                "n": n,
                "e": e,
            }]
        })
    }
}

fn bigint_to_b64url(v: &BigUint) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let bytes = v.to_bytes_be();
    URL_SAFE_NO_PAD.encode(bytes)
}
