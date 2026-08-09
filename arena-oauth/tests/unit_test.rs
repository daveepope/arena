use arena::dependency::{Dependency, RunnableDependency};
use arena_oauth::{localhost_self_signed_pem_pair, OauthDependency, TokenError};
use serde_json::Value;

struct NoopChildDependency;

#[async_trait::async_trait]
impl RunnableDependency for NoopChildDependency {
    fn identifier(&self) -> &str {
        "oauth-child"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn start(&mut self) {}

    async fn stop(&mut self) {}

    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }

    async fn soft_reset(&self) {}

    async fn hard_reset(&mut self) {}
}

#[test]
fn identifier_as_any_and_children_reflect_dependency_state() {
    let mut dep = OauthDependency::builder("oauth-accessors")
        .with_http()
        .build();

    assert!(dep.identifier().contains("oauth-accessors"));
    assert!(dep.as_any().downcast_ref::<OauthDependency>().is_some());
    assert!(dep.as_any_mut().downcast_mut::<OauthDependency>().is_some());
    assert!(dep.children().is_empty());

    dep.add_child(Box::new(NoopChildDependency));

    assert_eq!(dep.children().len(), 1);
    assert_eq!(dep.children_mut().len(), 1);
}

async fn fetch_access_token(client: &reqwest::Client, base: &str) -> String {
    let disc_url = format!("{base}/.well-known/oauth-authorization-server");
    let disc: Value = client
        .get(&disc_url)
        .send()
        .await
        .expect("discovery GET")
        .error_for_status()
        .expect("discovery status")
        .json()
        .await
        .expect("discovery JSON");

    let token_endpoint = disc["token_endpoint"]
        .as_str()
        .expect("token_endpoint string");

    let token_resp = client
        .post(token_endpoint)
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .expect("token POST")
        .error_for_status()
        .expect("token status");

    let token_json: Value = token_resp.json().await.expect("token JSON");
    token_json["access_token"]
        .as_str()
        .expect("access_token string")
        .to_string()
}

#[tokio::test]
async fn http_transport_serves_and_verifies_tokens_without_tls() {
    let mut dep = OauthDependency::builder("oauth-http").with_http().build();

    dep.start().await;
    assert!(dep.server_tls_certificate_pem().is_none());

    let base = dep
        .base_url()
        .expect("base_url after start")
        .trim_end_matches('/')
        .to_string();
    assert!(base.starts_with("http://"), "expected http base_url, got {base}");
    assert_eq!(dep.issuer(), Some(base.clone()));

    let client = reqwest::Client::new();
    let access_token = fetch_access_token(&client, &base).await;

    dep.verify_access_token(&access_token)
        .expect("dependency JWT verify while running");

    dep.soft_reset().await;

    dep.hard_reset().await;
    let base_after_reset = dep
        .base_url()
        .expect("base_url after hard_reset")
        .trim_end_matches('/')
        .to_string();
    let access_token_after_reset = fetch_access_token(&client, &base_after_reset).await;
    dep.verify_access_token(&access_token_after_reset)
        .expect("dependency JWT verify after hard_reset");

    dep.stop().await;

    let err = dep
        .verify_access_token(&access_token_after_reset)
        .expect_err("verify should fail once server is stopped");
    assert!(matches!(err, TokenError::NotRunning));
}

#[tokio::test]
async fn custom_pem_transport_exposes_provided_certificate() {
    let (cert_pem, key_pem) = localhost_self_signed_pem_pair().expect("generate test TLS pair");

    let mut dep = OauthDependency::builder("oauth-custom-pem")
        .with_server_tls_pem(cert_pem.clone(), key_pem)
        .build();

    dep.start().await;

    assert_eq!(dep.server_tls_certificate_pem(), Some(cert_pem.as_str()));
    assert!(dep
        .base_url()
        .expect("base_url after start")
        .starts_with("https://"));

    dep.stop().await;
}
