#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Cognito { pool_id: String },
    Okta,
    EntraId { tenant_id: String },
}

impl Provider {
    pub(crate) fn issuer_path(&self) -> String {
        match self {
            Provider::Cognito { pool_id } => format!("/{pool_id}"),
            Provider::Okta => String::new(),
            Provider::EntraId { tenant_id } => format!("/{tenant_id}/v2.0"),
        }
    }

    pub(crate) fn jwks_path(&self) -> String {
        match self {
            Provider::Cognito { pool_id } => format!("/{pool_id}/.well-known/jwks.json"),
            Provider::Okta => "/v1/keys".to_string(),
            Provider::EntraId { tenant_id } => format!("/{tenant_id}/discovery/v2.0/keys"),
        }
    }
}
