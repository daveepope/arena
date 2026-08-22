use arena::component::RunnableComponent;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

fn tcp_reachable(addr: SocketAddr) -> bool {
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

async fn wait_for_tcp_port(port: u16, timeout: Duration) -> bool {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let deadline = Instant::now() + timeout;
    loop {
        if tcp_reachable(addr) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn start_from_image_pulls_and_runs_prebuilt_image() {
    const HOST_PORT: u16 = 16379;

    let mut component = ContainerizedComponent::from_image("from-image-probe", "redis:8-alpine")
        .with_platform(arena_container::platform::docker_platform())
        .with_port_mapping(HOST_PORT, 6379)
        .build()
        .await
        .expect("build containerized component");

    component.start().await;

    let reachable = wait_for_tcp_port(HOST_PORT, Duration::from_secs(15)).await;

    component.stop().await;

    assert!(
        reachable,
        "expected container started from a pre-built image to be reachable on its mapped port"
    );
}
