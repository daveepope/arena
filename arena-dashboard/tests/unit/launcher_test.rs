use std::time::Duration;

use arena_dashboard::launcher::{find_executable, poll_ready, resolve_port};
use arena_dashboard::{app, AppState, DEFAULT_PORT};
use axum::routing::get;
use axum::Router;

#[test]
fn resolve_port_zero_returns_default_port() {
    assert_eq!(resolve_port(0), DEFAULT_PORT);
}

#[test]
fn resolve_port_nonzero_returns_given_port() {
    assert_eq!(resolve_port(9999), 9999);
}

#[path = "../support/temp_search_dir.rs"]
mod temp_search_dir;
use temp_search_dir::TempSearchDir;

#[test]
fn find_executable_search_dir_with_matching_file_returns_it() {
    let dir = TempSearchDir::create("arena-dashboard-launcher-match");
    let candidate = dir.path().join("arena-dashboard-server-linux");
    std::fs::write(&candidate, b"stub").expect("write stub executable");

    let found = find_executable(Some(dir.path())).expect("executable should be found");

    assert_eq!(found, candidate);
}

#[test]
fn find_executable_search_dir_without_match_returns_none() {
    let dir = TempSearchDir::create("arena-dashboard-launcher-nomatch");

    let found = find_executable(Some(dir.path()));

    assert!(found.is_none());
}

#[test]
fn find_executable_no_search_dir_and_no_runfiles_returns_none() {
    let found = find_executable(None);

    assert!(found.is_none());
}

async fn spawn_plain_http_server() -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let router = Router::new().route("/", get(|| async { "ok" }));
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    format!("http://{addr}")
}

async fn spawn_dashboard_server() -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app(AppState::new())).await;
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn poll_ready_non_dashboard_server_returns_false() {
    let url = spawn_plain_http_server().await;

    let ready = tokio::task::spawn_blocking(move || poll_ready(&url, Duration::from_millis(300)))
        .await
        .expect("join blocking poll");

    assert!(!ready, "an unrelated 2xx responder must not be treated as the dashboard");
}

#[tokio::test]
async fn poll_ready_real_dashboard_server_returns_true() {
    let url = spawn_dashboard_server().await;

    let ready = tokio::task::spawn_blocking(move || poll_ready(&url, Duration::from_millis(300)))
        .await
        .expect("join blocking poll");

    assert!(ready, "the real dashboard server must be recognized as ready");
}
