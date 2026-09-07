use arena::dependency::{Dependency, RunnableDependency};
use arena_ffi::managed_playbook::{build, ManagedPlaybookConfig};
use arena_http::{HttpDependency, HttpImpl};
use async_trait::async_trait;

fn playbook_config(json: &str) -> ManagedPlaybookConfig {
    serde_json::from_str(json).expect("valid managed playbook config")
}

struct StubHttpImpl {
    base_url: Option<String>,
}

#[async_trait]
impl HttpImpl for StubHttpImpl {
    async fn start(
        &mut self,
        _port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) -> Result<(), String> {
        self.base_url = Some("http://127.0.0.1:8080".to_string());
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.base_url = None;
        Ok(())
    }

    async fn force_stop(&mut self) -> bool {
        true
    }

    fn release(&mut self) {}

    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        self.base_url.as_deref().map(|url| format!("{url}/__admin"))
    }
}

struct ImmediateReadinessCheck;

#[async_trait]
impl arena::healthcheck::ReadinessCheck for ImmediateReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _admin_url: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn build_http_empty_mappings_run_returns_registration_fault() {
    let mut dep = HttpDependency::builder("ffi-managed-empty-mappings")
        .with_impl(StubHttpImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(ImmediateReadinessCheck)
        .build()
        .expect("build http dependency");
    dep.start().await.expect("start http dependency");
    let dependency_identifier = dep.identifier().to_string();
    let deps: Vec<Dependency> = vec![Box::new(dep)];

    let config = playbook_config(&format!(
        r#"{{"identifier": "pb-http-empty", "kind": "http", "dependency_identifier": "{dependency_identifier}", "mappings": []}}"#,
    ));
    let playbook = build(config);

    let Err(fault) = playbook.run(&deps).await else {
        panic!("empty mappings must fault");
    };

    assert_eq!(fault.id, "pb-http-empty");
    assert!(fault.message.contains("http playbook registration failed"));
    assert!(fault.message.contains("mappings must not be empty"));
}

#[test]
fn build_all_kinds_dispatches_to_each_builder() {
    let configs = [
        playbook_config(
            r#"{"identifier": "pb-http", "kind": "http", "dependency_identifier": "http", "mappings": []}"#,
        ),
        playbook_config(r#"{"identifier": "pb-mssql", "kind": "mssql", "dependency_identifier": "mssql"}"#),
        playbook_config(r#"{"identifier": "pb-oracle", "kind": "oracledb", "dependency_identifier": "oracle"}"#),
        playbook_config(
            r#"{"identifier": "pb-localstack", "kind": "localstack", "dependency_identifier": "localstack"}"#,
        ),
        playbook_config(r#"{"identifier": "pb-postgres", "kind": "postgres", "dependency_identifier": "postgres"}"#),
    ];

    let identifiers: Vec<String> = configs.into_iter().map(|config| build(config).identifier().to_string()).collect();

    assert_eq!(identifiers, vec!["pb-http", "pb-mssql", "pb-oracle", "pb-localstack", "pb-postgres"]);
}
