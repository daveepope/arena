use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_eventbridge as eventbridge;
use aws_sdk_lambda as lambda;
use aws_sdk_sqs as sqs;
use aws_smithy_http_client::tls::rustls_provider::CryptoMode;
use aws_smithy_http_client::{tls, Builder as HttpClientBuilder};
use aws_smithy_types::Blob;
use futures_timer::Delay;

use crate::builder::{EventRuleSpec, LambdaSpec, QueueSpec};

const LOCALSTACK_ACCESS_KEY: &str = "test";
const LOCALSTACK_SECRET_KEY: &str = "test";
const LOCALSTACK_REGION: &str = "us-east-1";
const LOCALSTACK_LAMBDA_ROLE_ARN: &str = "arn:aws:iam::000000000000:role/lambda-role";

pub struct ResourceCreator;

impl ResourceCreator {
    pub async fn create_queue(endpoint: &str, spec: &QueueSpec) -> Result<String, String> {
        let config = sdk_config(endpoint).await;
        let client = sqs::Client::new(&config);

        let mut req = client.create_queue().queue_name(&spec.name);
        if spec.fifo {
            req = req
                .attributes(sqs::types::QueueAttributeName::FifoQueue, "true")
                .attributes(
                    sqs::types::QueueAttributeName::ContentBasedDeduplication,
                    "true",
                );
        }

        let out = req
            .send()
            .await
            .map_err(|e| format!("sqs create queue failed: {}", service_err(&e)))?;

        out.queue_url()
            .map(|s| s.to_string())
            .ok_or_else(|| "sqs create queue response missing queue_url".to_string())
    }

    pub async fn get_queue_arn(endpoint: &str, queue_url: &str) -> Result<String, String> {
        let config = sdk_config(endpoint).await;
        let client = sqs::Client::new(&config);

        let out = client
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(sqs::types::QueueAttributeName::QueueArn)
            .send()
            .await
            .map_err(|e| format!("sqs get_queue_attributes failed: {}", service_err(&e)))?;

        out.attributes()
            .and_then(|m| m.get(&sqs::types::QueueAttributeName::QueueArn))
            .cloned()
            .ok_or_else(|| "sqs get_queue_attributes missing QueueArn".to_string())
    }

    pub async fn purge_queue(endpoint: &str, queue_url: &str) -> Result<(), String> {
        let config = sdk_config(endpoint).await;
        let client = sqs::Client::new(&config);

        client
            .purge_queue()
            .queue_url(queue_url)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| format!("sqs purge queue failed: {}", service_err(&e)))
    }

    pub async fn create_event_bus(endpoint: &str, name: &str) -> Result<(), String> {
        let config = sdk_config(endpoint).await;
        let client = eventbridge::Client::new(&config);

        match client.create_event_bus().name(name).send().await {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = service_err(&e);
                if msg.to_lowercase().contains("already exists") {
                    Ok(())
                } else {
                    Err(format!("eventbridge create event bus failed: {msg}"))
                }
            }
        }
    }

    pub async fn create_lambda(endpoint: &str, spec: &LambdaSpec) -> Result<String, String> {
        let config = sdk_config(endpoint).await;
        let client = lambda::Client::new(&config);

        let zip_bytes = zip_directory(&spec.source_dir)?;

        let code = lambda::types::FunctionCode::builder()
            .zip_file(Blob::new(zip_bytes))
            .build();

        let mut req = client
            .create_function()
            .function_name(&spec.name)
            .runtime(lambda::types::Runtime::from(spec.runtime.as_str()))
            .handler(&spec.handler)
            .role(LOCALSTACK_LAMBDA_ROLE_ARN)
            .code(code);

        if !spec.environment.is_empty() {
            let env_map: std::collections::HashMap<String, String> = spec
                .environment
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let env = lambda::types::Environment::builder()
                .set_variables(Some(env_map))
                .build();
            req = req.environment(env);
        }

        let out = req
            .send()
            .await
            .map_err(|e| format!("lambda create_function failed: {}", service_err(&e)))?;

        let arn = out
            .function_arn()
            .map(|s| s.to_string())
            .ok_or_else(|| "lambda create_function response missing function_arn".to_string())?;

        wait_until_lambda_active(&client, &spec.name).await?;

        Ok(arn)
    }

    pub async fn create_event_rule(
        endpoint: &str,
        rule: &EventRuleSpec,
        target_arns: Vec<(String, String)>,
    ) -> Result<(), String> {
        let config = sdk_config(endpoint).await;
        let client = eventbridge::Client::new(&config);

        let mut put_rule = client
            .put_rule()
            .name(&rule.name)
            .event_pattern(&rule.event_pattern)
            .state(eventbridge::types::RuleState::Enabled);
        if let Some(bus) = &rule.event_bus {
            put_rule = put_rule.event_bus_name(bus);
        }
        put_rule
            .send()
            .await
            .map_err(|e| format!("eventbridge put_rule failed: {}", service_err(&e)))?;

        let mut targets: Vec<eventbridge::types::Target> = Vec::with_capacity(target_arns.len());
        for (id, arn) in target_arns {
            let target = eventbridge::types::Target::builder()
                .id(id)
                .arn(arn)
                .build()
                .map_err(|e| format!("eventbridge target build failed: {e}"))?;
            targets.push(target);
        }

        let mut put_targets = client
            .put_targets()
            .rule(&rule.name)
            .set_targets(Some(targets));
        if let Some(bus) = &rule.event_bus {
            put_targets = put_targets.event_bus_name(bus);
        }
        put_targets
            .send()
            .await
            .map_err(|e| format!("eventbridge put_targets failed: {}", service_err(&e)))?;

        Ok(())
    }
}

async fn sdk_config(endpoint: &str) -> SdkConfig {
    let creds = Credentials::new(
        LOCALSTACK_ACCESS_KEY,
        LOCALSTACK_SECRET_KEY,
        None,
        None,
        "arena-localstack",
    );

    let http_client = HttpClientBuilder::new()
        .tls_provider(tls::Provider::Rustls(CryptoMode::Ring))
        .build_https();

    aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(LOCALSTACK_REGION))
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .http_client(http_client)
        .load()
        .await
}

async fn wait_until_lambda_active(client: &lambda::Client, name: &str) -> Result<(), String> {
    let timeout = Duration::from_secs(30);
    let poll_every = Duration::from_millis(200);
    let start = Instant::now();

    loop {
        if start.elapsed() >= timeout {
            return Err(format!(
                "lambda {name} did not become active within {timeout:?}"
            ));
        }

        match client.get_function().function_name(name).send().await {
            Ok(out) => {
                let state = out.configuration().and_then(|c| c.state()).cloned();
                match state {
                    Some(lambda::types::State::Active) => return Ok(()),
                    Some(lambda::types::State::Failed) => {
                        let detail = out
                            .configuration()
                            .and_then(|c| c.state_reason())
                            .unwrap_or("no state_reason from get_function");
                        return Err(format!("lambda {name} entered Failed state: {detail}"));
                    }
                    _ => {}
                }
            }
            Err(e) => {
                tracing::debug!(
                    resource = %name,
                    error = %service_err(&e),
                    "lambda get_function transient error while waiting"
                );
            }
        }

        Delay::new(poll_every).await;
    }
}

fn service_err<E: std::fmt::Display>(err: &E) -> String {
    err.to_string()
}

fn zip_directory(src: &Path) -> Result<Vec<u8>, String> {
    let files = collect_files(src)?;
    if files.is_empty() {
        return Err(format!(
            "lambda source directory {} is empty",
            src.display()
        ));
    }

    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        for (rel, abs) in files {
            writer
                .start_file(rel.clone(), options)
                .map_err(|e| format!("zip start_file {rel}: {e}"))?;
            let mut f =
                std::fs::File::open(&abs).map_err(|e| format!("open {}: {e}", abs.display()))?;
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes)
                .map_err(|e| format!("read {}: {e}", abs.display()))?;
            writer
                .write_all(&bytes)
                .map_err(|e| format!("zip write {rel}: {e}"))?;
        }

        writer.finish().map_err(|e| format!("zip finish: {e}"))?;
    }

    Ok(buf)
}

fn collect_files(src: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    if !src.is_dir() {
        return Err(format!(
            "lambda source path {} is not a directory",
            src.display()
        ));
    }

    let mut out = Vec::new();
    let mut stack = vec![src.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rd = std::fs::read_dir(&dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
        for entry in rd {
            let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                let rel = p
                    .strip_prefix(src)
                    .map_err(|e| format!("strip_prefix: {e}"))?;
                out.push((path_to_zip_name(rel), p));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn path_to_zip_name(rel: &Path) -> String {
    let mut parts = Vec::new();
    for c in rel.components() {
        if let std::path::Component::Normal(s) = c {
            parts.push(s.to_string_lossy().to_string());
        }
    }
    parts.join("/")
}
