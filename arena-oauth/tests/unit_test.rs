use arena::dependency::{Dependency, RunnableDependency};
use arena_oauth::OauthDependency;

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

#[test]
fn ephemeral_tls_hosts_ipv6_loopback_listen_ip_includes_that_address() {
    let hosts = arena_oauth::ephemeral_tls_hosts(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST));

    assert_eq!(hosts, vec!["localhost", "127.0.0.1", "::1"]);
}

#[test]
fn ephemeral_tls_hosts_loopback_and_unspecified_listen_ips_stay_localhost_only() {
    let cases = [
        std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        std::net::IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
    ];

    for listen_ip in cases {
        assert_eq!(
            arena_oauth::ephemeral_tls_hosts(listen_ip),
            vec!["localhost", "127.0.0.1"],
            "listen ip: {listen_ip}"
        );
    }
}
