use bollard::query_parameters::CreateImageOptionsBuilder;
use bollard::Docker;
use futures::stream::BoxStream;
use futures::StreamExt;

pub trait ImagePullClient: Send + Sync {
    fn pull(&self, image: &str, platform: &str) -> BoxStream<'_, Result<String, String>>;
}

impl ImagePullClient for Docker {
    fn pull(&self, image: &str, platform: &str) -> BoxStream<'_, Result<String, String>> {
        let options = CreateImageOptionsBuilder::default()
            .from_image(image)
            .platform(platform)
            .build();

        self.create_image(Some(options), None, None)
            .map(|result| {
                result
                    .map(|info| info.status.unwrap_or_default())
                    .map_err(|e| e.to_string())
            })
            .boxed()
    }
}

pub async fn pull_image(
    identifier: &str,
    image: &str,
    platform: &str,
    runtime_client: &impl ImagePullClient,
) {
    tracing::debug!(
        component = %identifier,
        image = %image,
        phase = "image_pull_begin",
        "pulling container image",
    );

    let mut stream = runtime_client.pull(image, platform);

    while let Some(result) = stream.next().await {
        match result {
            Ok(status) => {
                if let Some(line) = pull_status_line(Some(&status)) {
                    tracing::debug!(
                        component = %identifier,
                        text = %line,
                        phase = "image_pull_stream",
                        "image pull output line",
                    );
                }
            }
            Err(e) => {
                panic!("{}: image pull failed: {}", identifier, e);
            }
        }
    }

    tracing::debug!(
        component = %identifier,
        image = %image,
        phase = "image_pull_done",
        "container image pulled",
    );
}

pub fn pull_status_line(status: Option<&str>) -> Option<&str> {
    let line = status?.trim_end();
    (!line.is_empty()).then_some(line)
}
