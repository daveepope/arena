use std::net::IpAddr;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;

use crate::discovery::OAuthAuthorizationServerMetadata;
use crate::keys::RsaKeyPair;
use crate::token::AccessTokenClaims;

#[derive(Clone, Copy, Debug)]
pub(crate) struct OauthListenAddr {
    pub(crate) ip: IpAddr,
    pub(crate) port: u16,
}

pub(crate) struct OAuthSigningState {
    pub(crate) metadata: Arc<OAuthAuthorizationServerMetadata>,
    pub(crate) keys: Arc<RsaKeyPair>,
    pub(crate) token_ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenForm {
    pub(crate) grant_type: String,
    pub(crate) scope: Option<String>,
    pub(crate) client_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IntrospectForm {
    pub(crate) token: String,
}

pub(crate) fn introspection_active(claims: &AccessTokenClaims) -> serde_json::Value {
    json!({
        "active": true,
        "scope": claims.scope.clone().unwrap_or_default(),
        "client_id": claims.sub,
        "token_type": "Bearer",
        "iss": claims.iss,
    })
}
