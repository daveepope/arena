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
    EventBusSpec, EventRuleSpec, EventTargetKind, LambdaSpec, LocalstackDependencyBuilder, QueueSpec,
};
use crate::localstack_dependency::healthcheck::DefaultLocalstackReadinessCheck;
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
    pub fn new(
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
            readiness_check: Box::new(DefaultLocalstackReadinessCheck),
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

    fn set_container_name(&self) -> String {
        arena_container::identifier::sanitize_for_container(&self.identifier)
    }

    fn endpoint_on_host(&self) -> Result<&str, String> {
        self.endpoint_url()
            .ok_or_else(|| "localstack endpoint not available yet".to_string())
    }

    async fn wait_until_ready(&self) {
        let timeout = Duration::from_secs(60);
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
                    log::debug!(
                        "[Localstack-{}] readiness endpoint missing: {}",
                        self.identifier, err
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
                        EventTargetKind::SqsQueue { queue_name } => self
                            .queue_arns
                            .get(queue_name)
                            .cloned()
                            .unwrap_or_else(|| {
                                panic!(
                                    "[Localstack-{}] event rule {} references unknown queue {}",
                                    self.identifier, rule.name, queue_name
                                )
                            }),
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
            log::info!(
                "[Localstack-{}] provisioned {} queue(s), {} lambda(s), {} bus(es), {} rule(s)",
                self.identifier,
                self.queues.len(),
                self.lambdas.len(),
                self.event_buses.len(),
                self.event_rules.len()
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

        log::info!("[Localstack-{}] starting.", self.identifier);
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten() {
            dep.start().await;
        }

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = self
            .container_name
            .clone()
            .unwrap_or_else(|| self.set_container_name());
        let services = self.services.clone();

        let sw_container = Instant::now();
        self.localstack_impl
            .start(self.port, &image_name, &image_tag, &container_name, &services)
            .await;
        log::debug!(
            "[Localstack-{}] container start in {:?}.",
            self.identifier,
            sw_container.elapsed()
        );

        let sw_ready = Instant::now();
        self.wait_until_ready().await;
        log::debug!(
            "[Localstack-{}] readiness in {:?}.",
            self.identifier,
            sw_ready.elapsed()
        );

        self.provision_resources().await;

        self.running = true;
        log::debug!(
            "[Localstack-{}] start complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[Localstack-{}] started.", self.identifier);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[Localstack-{}] stopping.", self.identifier);
        let sw = Instant::now();

        self.localstack_impl.stop().await;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::debug!(
            "[Localstack-{}] stop complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[Localstack-{}] stopped.", self.identifier);
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
                log::warn!(
                    "[Localstack-{}] soft reset: purge queue {name} failed: {e}",
                    self.identifier
                );
            } else {
                log::info!(
                    "[Localstack-{}] soft reset: purged queue {name}",
                    self.identifier
                );
            }
        }
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }

        log::info!(
            "[Localstack-{}] hard reset: restarting container",
            self.identifier
        );
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = self
            .container_name
            .clone()
            .unwrap_or_else(|| self.set_container_name());
        let services = self.services.clone();

        self.localstack_impl.stop().await;
        self.running = false;

        self.localstack_impl
            .start(self.port, &image_name, &image_tag, &container_name, &services)
            .await;
        self.wait_until_ready().await;
        self.provision_resources().await;
        self.running = true;
    }
}

impl Drop for LocalstackDependency {
    fn drop(&mut self) {
        if !self.running {
            return;
        }
        log::warn!(
            "[Localstack-{}] dropped while still running; stopping container.",
            self.identifier
        );
        futures::executor::block_on(<Self as RunnableDependency>::stop(self));
    }
}
