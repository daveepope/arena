use std::io::ErrorKind;
use std::process::ExitCode;

use arena_dashboard::{app, AppState};

#[tokio::main]
async fn main() -> ExitCode {
    let port: u16 = std::env::var("ARENA_DASHBOARD_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4873);
    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => listener,
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            eprintln!(
                "arena-dashboard already running on http://127.0.0.1:{port} - nothing to do"
            );
            return ExitCode::SUCCESS;
        }
        Err(e) => panic!("bind dashboard port: {e}"),
    };
    eprintln!("arena-dashboard listening on http://127.0.0.1:{port}");
    axum::serve(listener, app(AppState::new()))
        .await
        .expect("serve dashboard");
    ExitCode::SUCCESS
}
