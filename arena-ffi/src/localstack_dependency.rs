use std::path::PathBuf;

use arena::Dependency;
use arena_localstack::{
    EventRuleSpec, EventRuleTarget, EventTargetKind, LambdaSpec, LocalstackDependency,
    QueueSpec,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct LocalstackDependencyConfig {
    pub identifier: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub image_name: Option<String>,
    #[serde(default)]
    pub image_tag: Option<String>,
    #[serde(default)]
    pub container_name: Option<String>,
    #[serde(default)]
    pub services: Option<Vec<String>>,
    #[serde(default)]
    pub queues: Option<Vec<QueueSpecConfig>>,
    #[serde(default)]
    pub lambdas: Option<Vec<LambdaSpecConfig>>,
    #[serde(default)]
    pub event_buses: Option<Vec<EventBusSpecConfig>>,
    #[serde(default)]
    pub event_rules: Option<Vec<EventRuleSpecConfig>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct QueueSpecConfig {
    pub name: String,
    #[serde(default)]
    pub fifo: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LambdaSpecConfig {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub source_dir: String,
    #[serde(default)]
    pub environment: Option<Vec<(String, String)>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventBusSpecConfig {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventRuleSpecConfig {
    pub name: String,
    #[serde(default)]
    pub event_bus: Option<String>,
    pub event_pattern: String,
    pub targets: Vec<EventRuleTargetConfig>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventRuleTargetConfig {
    pub target_id: String,
    #[serde(flatten)]
    pub kind: EventTargetKindConfig,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum EventTargetKindConfig {
    SqsQueue { queue_name: String },
    Lambda { function_name: String },
}

pub(crate) fn build(
    config: &LocalstackDependencyConfig,
    network: &str,
) -> Result<Dependency, String> {
    let mut builder = LocalstackDependency::builder(&config.identifier).with_network(network);

    if let Some(port) = config.port {
        builder = builder.with_port(port);
    }
    if let Some(ref name) = config.image_name {
        builder = builder.with_image_name(name);
    }
    if let Some(ref tag) = config.image_tag {
        builder = builder.with_image_tag(tag);
    }
    if let Some(ref name) = config.container_name {
        builder = builder.with_container_name(name);
    }

    for svc in config.services.as_deref().unwrap_or(&[]) {
        builder = builder.with_service(svc);
    }
    for q in config.queues.as_deref().unwrap_or(&[]) {
        builder = builder.with_queue_spec(QueueSpec {
            name: q.name.clone(),
            fifo: q.fifo,
        });
    }
    for lam in config.lambdas.as_deref().unwrap_or(&[]) {
        let source_dir = PathBuf::from(&lam.source_dir);
        if !source_dir.is_dir() {
            return Err(format!(
                "localstack lambda '{}': source_dir '{}' does not exist or is not a directory",
                lam.name,
                source_dir.display()
            ));
        }
        builder = builder.with_lambda(LambdaSpec {
            name: lam.name.clone(),
            runtime: lam.runtime.clone(),
            handler: lam.handler.clone(),
            source_dir,
            environment: lam.environment.clone().unwrap_or_default(),
        });
    }
    for bus in config.event_buses.as_deref().unwrap_or(&[]) {
        builder = builder.with_event_bus(&bus.name);
    }
    for rule in config.event_rules.as_deref().unwrap_or(&[]) {
        let targets = rule
            .targets
            .iter()
            .map(|t| EventRuleTarget {
                target_id: t.target_id.clone(),
                kind: match &t.kind {
                    EventTargetKindConfig::SqsQueue { queue_name } => EventTargetKind::SqsQueue {
                        queue_name: queue_name.clone(),
                    },
                    EventTargetKindConfig::Lambda { function_name } => EventTargetKind::Lambda {
                        function_name: function_name.clone(),
                    },
                },
            })
            .collect();
        builder = builder.with_event_rule(EventRuleSpec {
            name: rule.name.clone(),
            event_bus: rule.event_bus.clone(),
            event_pattern: rule.event_pattern.clone(),
            targets,
        });
    }

    Ok(Box::new(builder.build()))
}
