use std::io::{ErrorKind, IsTerminal};
use std::process::ExitCode;

use arena_dashboard::{app, AppState, ServerShutdown, DEFAULT_PORT};

#[tokio::main]
async fn main() -> ExitCode {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT);
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
    let bound_port = listener
        .local_addr()
        .map(|addr| addr.port())
        .unwrap_or(port);
    eprintln!("arena-dashboard listening on http://127.0.0.1:{bound_port}");
    if std::io::stdin().is_terminal() {
        eprintln!("press enter to shut down the server");
    }
    let state = AppState::new();
    let shutdown = state.shutdown_handle();
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal(shutdown))
        .await
        .expect("serve dashboard");
    ExitCode::SUCCESS
}

async fn shutdown_signal(shutdown: std::sync::Arc<ServerShutdown>) {
    let stdin_enter = async {
        if !std::io::stdin().is_terminal() {
            std::future::pending::<()>().await;
            return;
        }
        let _ = tokio::task::spawn_blocking(|| {
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
        })
        .await;
    };
    tokio::select! {
        _ = stdin_enter => shutdown.trigger(),
        _ = shutdown.wait() => {},
    }
}
