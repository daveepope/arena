pub(crate) mod container_impl;
mod healthcheck;
pub mod resource_creator;

pub use container_impl::LOCALSTACK_INTERNAL_DOCKER_PORT;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures_timer::Delay;

use crate::builder::{
    EventBusSpec, EventRuleSpec, EventTargetKind, LambdaSpec, LocalstackDependencyBuilder,
    QueueSpec,
};
use crate::localstack_dependency::healthcheck::LocalstackHealthReadinessCheck;
use crate::localstack_dependency::resource_creator::ResourceCreator;

#[async_trait]
pub trait LocalstackImpl: Send + Sync {
    async fn start(
        &mut self,
        port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
        services: &[String],
    );
    async fn stop(&mut self);
    fn endpoint_url(&self) -> Option<&str>;
}

pub struct LocalstackDependency {
    pub identifier: String,
    localstack_impl: Box<dyn LocalstackImpl>,
    port: u16,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    needs_teardown: bool,
    children_started: bool,
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
    services: Vec<String>,
    queues: Vec<QueueSpec>,
    lambdas: Vec<LambdaSpec>,
    event_buses: Vec<EventBusSpec>,
    event_rules: Vec<EventRuleSpec>,
    queue_urls: HashMap<String, String>,
    queue_arns: HashMap<String, String>,
    lambda_arns: HashMap<String, String>,
}

impl LocalstackDependency {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identifier: String,
        localstack_impl: Box<dyn LocalstackImpl>,
        port: u16,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_name: String,
        image_tag: String,
        container_name: Option<String>,
        services: Vec<String>,
        queues: Vec<QueueSpec>,
        lambdas: Vec<LambdaSpec>,
        event_buses: Vec<EventBusSpec>,
        event_rules: Vec<EventRuleSpec>,
    ) -> Self {
        LocalstackDependency {
            identifier,
            localstack_impl,
            port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            running: false,
            needs_teardown: false,
            children_started: false,
            readiness_check: Box::new(LocalstackHealthReadinessCheck::new(services.clone())),
            services,
            queues,
            lambdas,
            event_buses,
            event_rules,
            queue_urls: HashMap::new(),
            queue_arns: HashMap::new(),
            lambda_arns: HashMap::new(),
        }
    }

    pub fn endpoint_url(&self) -> Option<&str> {
        self.localstack_impl.endpoint_url()
    }

    pub fn queue_url(&self, name: &str) -> Option<&str> {
        self.queue_urls.get(name).map(String::as_str)
    }

    pub fn queue_arn(&self, name: &str) -> Option<&str> {
        self.queue_arns.get(name).map(String::as_str)
    }

    pub fn lambda_arn(&self, name: &str) -> Option<&str> {
        self.lambda_arns.get(name).map(String::as_str)
    }

    pub fn queue_urls_snapshot(&self) -> Vec<(String, String)> {
        self.queue_urls
            .iter()
            .map(|(name, url)| (name.clone(), url.clone()))
            .collect()
    }

    pub fn builder(identifier: impl Into<String>) -> LocalstackDependencyBuilder {
        LocalstackDependencyBuilder::new(identifier)
    }

    pub fn playbook(&self) -> crate::playbook::Playbook {
        crate::playbook::Playbook::with(self)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    fn endpoint_on_host(&self) -> Result<&str, String> {
        self.endpoint_url()
            .ok_or_else(|| "localstack endpoint not available yet".to_string())
    }

    async fn wait_until_ready(&self) {
        let timeout = Duration::from_secs(45);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let endpoint = loop {
            if start.elapsed() >= timeout {
                panic!(
                    "[Localstack-{}] localstack did not become ready within {:?}",
                    self.identifier, timeout
                );
            }

            match self.endpoint_on_host() {
                Ok(v) => break v.to_string(),
                Err(err) => {
                    tracing::debug!(
                        dependency = %self.identifier,
                        reason = %err,
                        "localstack endpoint not ready yet"
                    );
                    Delay::new(poll_every).await;
                }
            }
        };

        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            panic!(
                "[Localstack-{}] localstack did not become ready within {:?}",
                self.identifier, timeout
            );
        }

        match self
            .readiness_check
            .is_ready(&self.identifier, &endpoint, remaining.as_millis() as u64)
            .await
        {
            Ok(()) => {}
            Err(err) => panic!(
                "[Localstack-{}] readiness check failed: {}",
                self.identifier, err
            ),
        }
    }

    async fn provision_resources(&mut self) {
        let endpoint = self
            .endpoint_on_host()
            .expect("endpoint for resource provisioning")
            .to_string();

        self.queue_urls.clear();
        self.queue_arns.clear();
        self.lambda_arns.clear();

        for spec in &self.queues {
            let url = ResourceCreator::create_queue(&endpoint, spec)
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "[Localstack-{}] queue create failed for {}: {e}",
                        self.identifier, spec.name
                    )
                });
            let arn = ResourceCreator::get_queue_arn(&endpoint, &url)
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "[Localstack-{}] queue arn lookup failed for {}: {e}",
                        self.identifier, spec.name
                    )
                });
            self.queue_urls.insert(spec.name.clone(), url);
            self.queue_arns.insert(spec.name.clone(), arn);
        }

        for bus in &self.event_buses {
            ResourceCreator::create_event_bus(&endpoint, &bus.name)
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "[Localstack-{}] event bus create failed for {}: {e}",
                        self.identifier, bus.name
                    )
                });
        }

        for spec in &self.lambdas {
            let arn = ResourceCreator::create_lambda(&endpoint, spec)
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "[Localstack-{}] lambda create failed for {}: {e}",
                        self.identifier, spec.name
                    )
                });
            self.lambda_arns.insert(spec.name.clone(), arn);
        }

        for rule in &self.event_rules {
            let target_arns = rule
                .targets
                .iter()
                .map(|t| {
                    let arn = match &t.kind {
                        EventTargetKind::SqsQueue { queue_name } => {
                            self.queue_arns.get(queue_name).cloned().unwrap_or_else(|| {
                                panic!(
                                    "[Localstack-{}] event rule {} references unknown queue {}",
                                    self.identifier, rule.name, queue_name
                                )
                            })
                        }
                        EventTargetKind::Lambda { function_name } => self
                            .lambda_arns
                            .get(function_name)
                            .cloned()
                            .unwrap_or_else(|| {
                                panic!(
                                    "[Localstack-{}] event rule {} references unknown lambda {}",
                                    self.identifier, rule.name, function_name
                                )
                            }),
                    };
                    (t.target_id.clone(), arn)
                })
                .collect();

            ResourceCreator::create_event_rule(&endpoint, rule, target_arns)
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "[Localstack-{}] event rule create failed for {}: {e}",
                        self.identifier, rule.name
                    )
                });
        }

        if !self.queues.is_empty()
            || !self.lambdas.is_empty()
            || !self.event_buses.is_empty()
            || !self.event_rules.is_empty()
        {
            tracing::debug!(
                dependency = %self.identifier,
                queue_count = self.queues.len(),
                lambda_count = self.lambdas.len(),
                bus_count = self.event_buses.len(),
                rule_count = self.event_rules.len(),
                "provisioned localstack resources"
            );
        }
    }
}

#[async_trait]
impl RunnableDependency for LocalstackDependency {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn start(&mut self) {
        if self.running {
            return;
        }

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();

        if let Some(children) = self.dependencies.as_mut() {
            if !children.is_empty() {
                self.children_started = true;
                for dep in children.iter_mut() {
                    dep.start().await;
                }
            }
        }

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );
        let services = self.services.clone();

        let sw_container = Instant::now();
        self.needs_teardown = true;
        self.localstack_impl
            .start(
                self.port,
                &image_name,
                &image_tag,
                &container_name,
                &services,
            )
            .await;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_container.elapsed(),
            "container start finished"
        );

        let sw_ready = Instant::now();
        self.wait_until_ready().await;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_ready.elapsed(),
            "readiness wait finished"
        );

        self.provision_resources().await;

        self.running = true;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started"
        );
    }

    async fn stop(&mut self) {
        self.localstack_impl.stop().await;
        self.needs_teardown = false;

        if !self.running {
            if self.children_started {
                for dep in self.dependencies.iter_mut().flatten().rev() {
                    dep.stop().await;
                }
                self.children_started = false;
            }
            return;
        }

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.children_started = false;
        self.running = false;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }

    async fn soft_reset(&self) {
        if !self.running {
            return;
        }

        let endpoint = self
            .endpoint_on_host()
            .expect("endpoint for soft reset")
            .to_string();

        for (name, url) in &self.queue_urls {
            if let Err(e) = ResourceCreator::purge_queue(&endpoint, url).await {
                tracing::warn!(
                    dependency = %self.identifier,
                    queue = %name,
                    error = %e,
                    "soft reset: queue purge failed"
                );
            } else {
                tracing::debug!(
                    dependency = %self.identifier,
                    queue = %name,
                    "soft reset: queue purged"
                );
            }
        }
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }

        tracing::debug!(
            dependency = %self.identifier,
            phase = "hard_reset",
            "restarting localstack container"
        );
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );
        let services = self.services.clone();

        self.localstack_impl.stop().await;
        self.running = false;

        self.localstack_impl
            .start(
                self.port,
                &image_name,
                &image_tag,
                &container_name,
                &services,
            )
            .await;
        self.wait_until_ready().await;
        self.provision_resources().await;
        self.running = true;
    }
}

impl Drop for LocalstackDependency {
    fn drop(&mut self) {
        if self.running {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; forcing stop"
            );
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        } else if self.needs_teardown || self.children_started {
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        }
    }
}
