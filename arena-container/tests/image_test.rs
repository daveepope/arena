use arena_container::image::{pull_image, pull_status_line, ImagePullClient};
use futures::stream::{self, BoxStream, StreamExt};

struct FakeImagePullClient {
    events: Vec<Result<String, String>>,
}

impl ImagePullClient for FakeImagePullClient {
    fn pull(&self, _image: &str, _platform: &str) -> BoxStream<'_, Result<String, String>> {
        stream::iter(self.events.clone()).boxed()
    }
}

#[tokio::test]
async fn pull_image_ok_events_completes_without_panic() {
    let client = FakeImagePullClient {
        events: vec![Ok("Pulling fs layer".to_string()), Ok(String::new())],
    };

    pull_image("web", "redis:8-alpine", "linux/amd64", &client).await;
}

#[tokio::test]
#[should_panic(expected = "web: image pull failed: daemon unreachable")]
async fn pull_image_error_event_panics_with_identifier_and_message() {
    let client = FakeImagePullClient {
        events: vec![Err("daemon unreachable".to_string())],
    };

    pull_image("web", "redis:8-alpine", "linux/amd64", &client).await;
}

#[test]
fn pull_status_line_none_returns_none() {
    assert_eq!(pull_status_line(None), None);
}

#[test]
fn pull_status_line_blank_returns_none() {
    assert_eq!(pull_status_line(Some("   ")), None);
}

#[test]
fn pull_status_line_trailing_whitespace_returns_trimmed() {
    assert_eq!(pull_status_line(Some("Pulling fs layer\n")), Some("Pulling fs layer"));
}
