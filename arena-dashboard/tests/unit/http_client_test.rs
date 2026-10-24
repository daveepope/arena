use std::time::{Duration, Instant};

use arena_dashboard::http_client::{build_client, CLIENT_BUILD_TIMEOUT};

#[test]
fn build_client_any_request_timeout_returns_a_client() {
    for request_timeout in [
        Duration::from_millis(1),
        Duration::from_millis(500),
        Duration::from_secs(2),
    ] {
        assert!(
            build_client(request_timeout).is_some(),
            "no client for {request_timeout:?}"
        );
    }
}

#[test]
fn build_client_reachable_runtime_returns_within_the_build_budget() {
    let started = Instant::now();
    let client = build_client(Duration::from_millis(500));
    let elapsed = started.elapsed();

    assert!(client.is_some());
    assert!(
        elapsed < CLIENT_BUILD_TIMEOUT,
        "build took {elapsed:?}, budget is {CLIENT_BUILD_TIMEOUT:?}"
    );
}
