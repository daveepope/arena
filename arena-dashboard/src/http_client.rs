use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

pub const CLIENT_BUILD_TIMEOUT: Duration = Duration::from_secs(5);

pub fn build_client(request_timeout: Duration) -> Option<reqwest::blocking::Client> {
    let (sender, receiver) = channel();
    thread::spawn(move || {
        let built = reqwest::blocking::Client::builder()
            .connect_timeout(request_timeout)
            .timeout(request_timeout)
            .build();
        let _ = sender.send(built);
    });
    match receiver.recv_timeout(CLIENT_BUILD_TIMEOUT) {
        Ok(Ok(client)) => Some(client),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "arena-dashboard http client could not be built");
            None
        }
        Err(_) => {
            tracing::warn!(
                waited = ?CLIENT_BUILD_TIMEOUT,
                "arena-dashboard http client did not start"
            );
            None
        }
    }
}
