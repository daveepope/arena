use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OAuthAuthorizationServerMetadata {
    pub(crate) issuer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) authorization_endpoint: Option<String>,
    pub(crate) token_endpoint: String,
    pub(crate) jwks_uri: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) scopes_supported: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) response_types_supported: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) grant_types_supported: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) token_endpoint_auth_methods_supported: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) introspection_endpoint: Option<String>,
}

impl OAuthAuthorizationServerMetadata {
    pub(crate) fn for_base_url(base: &str, scopes_supported: Vec<String>) -> Self {
        let base = base.trim_end_matches('/');
        Self {
            issuer: base.to_string(),
            authorization_endpoint: None,
            token_endpoint: format!("{base}/oauth/token"),
            jwks_uri: format!("{base}/.well-known/jwks.json"),
            scopes_supported,
            response_types_supported: vec!["token".into()],
            grant_types_supported: vec!["client_credentials".into()],
            token_endpoint_auth_methods_supported: vec![
                "client_secret_post".into(),
                "none".into(),
            ],
            introspection_endpoint: Some(format!("{base}/oauth/introspect")),
        }
    }
}
