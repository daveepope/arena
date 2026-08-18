use bollard::query_parameters::CreateImageOptionsBuilder;
use bollard::Docker;
use futures::StreamExt;

pub async fn pull_image(identifier: &str, image: &str, platform: &str, runtime_client: &Docker) {
    tracing::debug!(
        component = %identifier,
        image = %image,
        phase = "image_pull_begin",
        "pulling container image",
    );

    let options = CreateImageOptionsBuilder::default()
        .from_image(image)
        .platform(platform)
        .build();

    let mut stream = runtime_client.create_image(Some(options), None, None);

    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                if let Some(ref status) = info.status {
                    let msg = status.trim_end();
                    if !msg.is_empty() {
                        tracing::debug!(
                            component = %identifier,
                            text = %msg,
                            phase = "image_pull_stream",
                            "image pull output line",
                        );
                    }
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
