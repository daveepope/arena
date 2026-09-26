pub(crate) mod container_impl;
mod healthcheck;
pub mod resource_creator;

pub use container_impl::LOCALSTACK_INTERNAL_DOCKER_PORT;

use arena::lifecycle::message;
use arena::lifecycle::Subject;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{Fault, RunnableState};
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
    fn set_expiry(&mut self, _expiry: Option<Duration>) {}
    async fn start(
        &mut self,
        port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
        services: &[String],
    ) -> Result<(), String>;
    async fn stop(&mut self) -> Result<(), String>;
    async fn force_stop(&mut self) -> bool;
    fn release(&mut self);
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
    state: RunnableState,
    faults: Vec<Fault>,
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
            state: RunnableState::NotStarted,
            faults: Vec::new(),
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

    async fn wait_until_ready(&self) -> Result<(), String> {
        let timeout = Duration::from_secs(45);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let endpoint = loop {
            if start.elapsed() >= timeout {
                return Err(format!("localstack did not become ready within {timeout:?}"));
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
            return Err(format!("localstack did not become ready within {timeout:?}"));
        }

        self.readiness_check
            .is_ready(&self.identifier, &endpoint, remaining.as_millis() as u64)
            .await
            .map_err(message::readiness_failed)
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::dependency(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableDependency>::force_stop(self).await;
        fault
    }

    async fn provision_resources(&mut self) -> Result<(), String> {
        let endpoint = self.endpoint_on_host()?.to_string();

        self.queue_urls.clear();
        self.queue_arns.clear();
        self.lambda_arns.clear();

        for spec in &self.queues {
            let url = ResourceCreator::create_queue(&endpoint, spec)
                .await
                .map_err(|e| format!("queue create failed for {}: {e}", spec.name))?;
            let arn = ResourceCreator::get_queue_arn(&endpoint, &url)
                .await
                .map_err(|e| format!("queue arn lookup failed for {}: {e}", spec.name))?;
            self.queue_urls.insert(spec.name.clone(), url);
            self.queue_arns.insert(spec.name.clone(), arn);
        }

        for bus in &self.event_buses {
            ResourceCreator::create_event_bus(&endpoint, &bus.name)
                .await
                .map_err(|e| format!("event bus create failed for {}: {e}", bus.name))?;
        }

        for spec in &self.lambdas {
            let arn = ResourceCreator::create_lambda(&endpoint, spec)
                .await
                .map_err(|e| format!("lambda create failed for {}: {e}", spec.name))?;
            self.lambda_arns.insert(spec.name.clone(), arn);
        }

        for rule in &self.event_rules {
            let mut target_arns = Vec::with_capacity(rule.targets.len());
            for t in &rule.targets {
                let arn = match &t.kind {
                    EventTargetKind::SqsQueue { queue_name } => {
                        self.queue_arns.get(queue_name).cloned().ok_or_else(|| {
                            format!(
                                "event rule {} references unknown queue {queue_name}",
                                rule.name
                            )
                        })?
                    }
                    EventTargetKind::Lambda { function_name } => {
                        self.lambda_arns.get(function_name).cloned().ok_or_else(|| {
                            format!(
                                "event rule {} references unknown lambda {function_name}",
                                rule.name
                            )
                        })?
                    }
                };
                target_arns.push((t.target_id.clone(), arn));
            }

            ResourceCreator::create_event_rule(&endpoint, rule, target_arns)
                .await
                .map_err(|e| format!("event rule create failed for {}: {e}", rule.name))?;
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
        Ok(())
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

    fn state(&self) -> RunnableState {
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        if self.running {
            return Ok(());
        }
        self.state = RunnableState::Starting;

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();

        if let Some(children) = self.dependencies.as_mut() {
            if !children.is_empty() {
                self.children_started = true;
                let mut child_faults = Vec::new();
                for dep in children.iter_mut() {
                    if let Err(fault) = arena::dependency::start_child(dep).await {
                        child_faults.push(fault);
                    }
                }
                if !child_faults.is_empty() {
                    return Err(self.fail(message::child_start_failed(Subject::Dependency), child_faults).await);
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
        if let Err(message) = self
            .localstack_impl
            .start(
                self.port,
                &image_name,
                &image_tag,
                &container_name,
                &services,
            )
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_container.elapsed(),
            "container start finished"
        );

        let sw_ready = Instant::now();
        self.state = RunnableState::ReadinessCheck;
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_ready.elapsed(),
            "readiness wait finished"
        );

        if let Err(message) = self.provision_resources().await {
            return Err(self.fail(message, Vec::new()).await);
        }

        self.running = true;
        self.state = RunnableState::Started;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started"
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopping;
        let mut causes = Vec::new();

        if let Err(message) = self.localstack_impl.stop().await {
            causes.push(Fault::dependency(&self.identifier, message));
        }
        self.needs_teardown = false;

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten().rev() {
            if let Err(fault) = arena::dependency::stop_child(dep).await {
                causes.push(fault);
            }
        }

        self.children_started = false;
        self.running = false;

        if !causes.is_empty() {
            let fault =
                Fault::dependency(&self.identifier, message::stop_did_not_complete()).caused_by_all(causes);
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }

        self.state = RunnableState::Stopped;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
        Ok(())
    }

    fn release(&mut self) {
        self.localstack_impl.release();
        self.running = false;
        self.needs_teardown = false;
        self.children_started = false;
        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::release_child(dep);
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        let removed = self.localstack_impl.force_stop().await;
        self.needs_teardown = false;
        self.running = false;
        self.children_started = false;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::force_stop_child(dep).await;
        }

        if removed {
            self.state = RunnableState::Stopped;
            return;
        }

        let unconfirmed = Fault::dependency(
            &self.identifier,
            message::forced_teardown_unconfirmed(),
        );
        if !self
            .faults
            .iter()
            .any(|f| f.message == unconfirmed.message && f.id == unconfirmed.id)
        {
            self.faults.push(unconfirmed);
        }
        self.state = RunnableState::Faulted;
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }

    fn children(&self) -> &[Dependency] {
        self.dependencies.as_deref().unwrap_or(&[])
    }

    fn children_mut(&mut self) -> &mut [Dependency] {
        self.dependencies.as_deref_mut().unwrap_or(&mut [])
    }

    async fn soft_reset(&self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
        }

        let endpoint = self
            .endpoint_on_host()
            .map_err(|message| Fault::dependency(&self.identifier, message))?
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
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
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

        if let Err(message) = self.localstack_impl.stop().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = false;

        if let Err(message) = self
            .localstack_impl
            .start(
                self.port,
                &image_name,
                &image_tag,
                &container_name,
                &services,
            )
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        if let Err(message) = self.provision_resources().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = true;
        self.state = RunnableState::Started;
        Ok(())
    }
}

impl Drop for LocalstackDependency {
    fn drop(&mut self) {
        if self.running || self.needs_teardown || self.children_started {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; releasing container"
            );
            self.localstack_impl.release();
            self.running = false;
            self.needs_teardown = false;
            self.children_started = false;
            self.state = RunnableState::Stopped;
        }
    }
}
