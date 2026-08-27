use crate::builder::DEFAULT_JWKS_PATH;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum Provider {
    Cognito { pool_id: String },
    Okta,
    EntraId { tenant_id: String },
    Custom {
        #[serde(default)]
        issuer_path: Option<String>,
    },
}

impl PartialEq for Provider {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Provider::Cognito { pool_id: a }, Provider::Cognito { pool_id: b }) => a == b,
            (Provider::Okta, Provider::Okta) => true,
            (Provider::EntraId { tenant_id: a }, Provider::EntraId { tenant_id: b }) => a == b,
            (Provider::Custom { .. }, Provider::Custom { .. }) => {
                self.issuer_path() == other.issuer_path()
            }
            _ => false,
        }
    }
}

impl Eq for Provider {}

impl Provider {
    pub(crate) fn issuer_path(&self) -> String {
        match self {
            Provider::Cognito { pool_id } => format!("/{pool_id}"),
            Provider::Okta => String::new(),
            Provider::EntraId { tenant_id } => format!("/{tenant_id}/v2.0"),
            Provider::Custom { issuer_path } => issuer_path.clone().unwrap_or_default(),
        }
    }

    pub(crate) fn jwks_path(&self) -> String {
        match self {
            Provider::Cognito { pool_id } => format!("/{pool_id}/.well-known/jwks.json"),
            Provider::Okta => "/v1/keys".to_string(),
            Provider::EntraId { tenant_id } => format!("/{tenant_id}/discovery/v2.0/keys"),
            Provider::Custom { issuer_path } => {
                let issuer_path = issuer_path.clone().unwrap_or_default();
                if issuer_path.is_empty() {
                    DEFAULT_JWKS_PATH.to_string()
                } else {
                    format!("{issuer_path}{DEFAULT_JWKS_PATH}")
                }
            }
        }
    }
}
