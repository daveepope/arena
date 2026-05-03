use oauth2::basic::BasicClient;
use oauth2::{ClientId, Scope, TokenResponse, TokenUrl};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AuthorizationServerMetadata {
    token_endpoint: String,
}

pub async fn fetch_client_credentials_access_token(
    http: &reqwest::Client,
    issuer_base: &str,
    scope: Option<&str>,
) -> Result<String, String> {
    let base = issuer_base.trim_end_matches('/');
    let disc_url = format!("{base}/.well-known/oauth-authorization-server");
    let meta: AuthorizationServerMetadata = http
        .get(&disc_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let token_url = TokenUrl::new(meta.token_endpoint).map_err(|e| e.to_string())?;
    let client = BasicClient::new(ClientId::new("arena-examples".into())).set_token_uri(token_url);

    let mut req = client.exchange_client_credentials();
    if let Some(s) = scope {
        req = req.add_scope(Scope::new(s.to_string()));
    }

    let token = req.request_async(http).await.map_err(|e| e.to_string())?;

    Ok(token.access_token().secret().to_string())
}
