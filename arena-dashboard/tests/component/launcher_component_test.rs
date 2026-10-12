use arena_dashboard::{app, ensure_running, AppState};

#[path = "../support/temp_search_dir.rs"]
mod temp_search_dir;
use temp_search_dir::TempSearchDir;

struct KillStagedServerOnDrop(std::path::PathBuf);

impl Drop for KillStagedServerOnDrop {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("pkill").arg("-f").arg(&self.0).status();
        }
    }
}

fn platform_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "arena-dashboard-server.exe"
    } else if cfg!(target_os = "macos") {
        "arena-dashboard-server-macos"
    } else {
        "arena-dashboard-server-linux"
    }
}

async fn spawn_server_on(port: u16) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("bind target port");
    let state = AppState::new();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app(state)).await;
    });
}

async fn ephemeral_port() -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

#[tokio::test]
async fn ensure_running_server_already_listening_reuses_it_without_search_dir() {
    let port = ephemeral_port().await;
    spawn_server_on(port).await;

    let url = tokio::task::spawn_blocking(move || ensure_running(port, None))
        .await
        .expect("join blocking task")
        .expect("ensure_running should succeed");

    assert_eq!(url, format!("http://127.0.0.1:{port}"));
}

#[tokio::test]
async fn ensure_running_called_twice_against_listening_does_not_respawn() {
    let port = ephemeral_port().await;
    spawn_server_on(port).await;

    let first = tokio::task::spawn_blocking(move || ensure_running(port, None))
        .await
        .expect("join blocking task")
        .expect("first call should succeed");
    let second = tokio::task::spawn_blocking(move || ensure_running(port, None))
        .await
        .expect("join blocking task")
        .expect("second call should succeed");

    assert_eq!(first, second);
}

#[test]
fn ensure_running_executable_found_in_search_dir_spawns_and_becomes_ready() {
    let runfiles = runfiles::Runfiles::create().expect("runfiles available under bazel test");
    let server_path = runfiles
        .rlocation("_main/arena-dashboard/server")
        .filter(|p| p.is_file())
        .expect("arena-dashboard server binary present in runfiles");

    let search_dir = TempSearchDir::create("arena-dashboard-launcher-spawn");
    let staged_path = search_dir.path().join(platform_executable_name());
    std::fs::copy(&server_path, &staged_path).expect("stage server binary into search dir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&staged_path)
            .expect("read staged binary metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&staged_path, perms).expect("make staged binary executable");
    }
    let _kill_staged_server = KillStagedServerOnDrop(staged_path.clone());

    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("local addr")
        .port();

    let url = ensure_running(port, Some(search_dir.path())).expect("ensure_running should succeed");

    assert_eq!(url, format!("http://127.0.0.1:{port}"));
    let response = reqwest::blocking::get(&url).expect("request spawned server");
    assert!(response.status().is_success());
}
