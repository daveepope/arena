use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStderr, Command, Stdio};
use std::time::{Duration, Instant};

use arena_dashboard::launcher::poll_ready;
use arena_dashboard::{CALLER_HEADER, CALLER_HEADER_VALUE};
use futures::StreamExt;

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn announced_port(stderr: ChildStderr) -> u16 {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).expect("read server stderr");
        assert!(read > 0, "server process exited before announcing a port");
        if let Some(port) = line.trim().rsplit(':').next().and_then(|p| p.parse().ok()) {
            return port;
        }
    }
}

async fn spawn_server_process() -> (KillOnDrop, u16) {
    let runfiles = runfiles::Runfiles::create().expect("runfiles available under bazel test");
    let server_path = ["_main/arena-dashboard/server.exe", "_main/arena-dashboard/server"]
        .into_iter()
        .filter_map(|candidate| runfiles.rlocation(candidate))
        .find(|p| p.is_file())
        .expect("arena-dashboard server binary present in runfiles");

    let mut child = Command::new(server_path)
        .arg("0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn arena-dashboard server process");

    let stderr = child.stderr.take().expect("server stderr piped");
    let port = tokio::task::spawn_blocking(move || announced_port(stderr))
        .await
        .expect("join stderr read");

    let url = format!("http://127.0.0.1:{port}");
    let ready = tokio::task::spawn_blocking(move || poll_ready(&url, Duration::from_secs(10)))
        .await
        .expect("join readiness poll");
    assert!(ready, "server process did not become ready");
    (KillOnDrop(child), port)
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait().expect("poll server process") {
            Some(_) => return true,
            None if Instant::now() >= deadline => return false,
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn port_is_free(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_witheventsclientconnected_processexitsandfreesport() {
    let (mut server, port) = spawn_server_process().await;
    let base_url = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();

    let events_response = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");
    let mut stream = events_response.bytes_stream();
    tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("first frame within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");

    client
        .post(format!("{base_url}/shutdown"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE)
        .send()
        .await
        .expect("post shutdown");

    assert!(
        wait_for_exit(&mut server.0, Duration::from_secs(5)),
        "server process still alive while an events client is connected"
    );
    assert!(port_is_free(port), "server process left the port bound");
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_noclientsconnected_processexits() {
    let (mut server, port) = spawn_server_process().await;
    let base_url = format!("http://127.0.0.1:{port}");

    reqwest::Client::new()
        .post(format!("{base_url}/shutdown"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE)
        .send()
        .await
        .expect("post shutdown");

    assert!(
        wait_for_exit(&mut server.0, Duration::from_secs(5)),
        "server process still alive"
    );
    assert!(port_is_free(port), "server process left the port bound");
}
