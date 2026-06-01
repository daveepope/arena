pub(crate) mod container_impl;
mod healthcheck;

use crate::admin_client::admin_api_client;
use crate::builder::HttpDependencyBuilder;
use crate::http_dependency::healthcheck::DefaultHttpReadinessCheck;
use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[async_trait]
pub trait HttpImpl: Send + Sync {
    async fn start(&mut self, port: u16, image_name: &str, image_tag: &str, container_name: &str);
    async fn stop(&mut self);
    fn base_url(&self) -> Option<&str>;
    fn admin_url(&self) -> Option<String>;
    fn https_base_url(&self) -> Option<&str> {
        None
    }
}

pub struct HttpDependency {
    pub identifier: String,
    http_impl: Box<dyn HttpImpl>,
    port: u16,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    needs_teardown: bool,
    children_started: bool,
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    trusted_tls_certificate_pem: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
}

impl HttpDependency {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identifier: String,
        http_impl: Box<dyn HttpImpl>,
        port: u16,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_name: String,
        image_tag: String,
        container_name: Option<String>,
        trusted_tls_certificate_pem: Option<String>,
    ) -> Self {
        let readiness_check: Box<dyn ReadinessCheck> = Box::new(DefaultHttpReadinessCheck::new(
            trusted_tls_certificate_pem.clone(),
        ));
        Self {
            identifier,
            http_impl,
            port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            trusted_tls_certificate_pem,
            running: false,
            needs_teardown: false,
            children_started: false,
            readiness_check,
        }
    }

    pub fn base_url(&self) -> Option<&str> {
        self.http_impl.base_url()
    }

    pub fn admin_url(&self) -> Option<String> {
        self.http_impl.admin_url()
    }

    pub fn https_base_url(&self) -> Option<&str> {
        self.http_impl.https_base_url()
    }

    pub fn trusted_certificate_pem(&self) -> Option<&str> {
        self.trusted_tls_certificate_pem.as_deref()
    }

    pub fn builder(identifier: impl Into<String>) -> HttpDependencyBuilder {
        HttpDependencyBuilder::new(identifier)
    }

    pub fn playbook(&self) -> crate::playbook::Playbook {
        crate::playbook::Playbook::with(self)
    }

    pub async fn reset_journal(&self) {
        if !self.running {
            return;
        }

        let admin_url = self.admin_url_or_panic();

        tracing::debug!(
            dependency = %self.identifier,
            phase = "reset_journal",
            "clearing request journal"
        );

        let client = admin_api_client(&admin_url, self.trusted_tls_certificate_pem.as_deref());
        let response = client
            .delete(format!("{admin_url}/requests"))
            .send()
            .await
            .unwrap_or_else(|e| panic!("[Http-{}] reset_journal failed: {e}", self.identifier));

        if !response.status().is_success() {
            panic!(
                "[Http-{}] reset_journal got HTTP {}",
                self.identifier,
                response.status()
            );
        }
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    fn default_container_name(&self) -> String {
        arena_container::identifier::sanitize_for_container(&self.identifier)
    }

    fn admin_url_or_panic(&self) -> String {
        self.admin_url()
            .unwrap_or_else(|| panic!("[Http-{}] admin url not available yet", self.identifier))
    }

    async fn wait_until_ready(&self) {
        let timeout = Duration::from_secs(15);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let admin_url = loop {
            if start.elapsed() >= timeout {
                panic!(
                    "[Http-{}] did not become ready within {:?}",
                    self.identifier, timeout
                );
            }

            match self.admin_url() {
                Some(url) => break url,
                None => {
                    tracing::debug!(
                        dependency = %self.identifier,
                        phase = "readiness_poll",
                        "admin url not available yet"
                    );
                    futures_timer::Delay::new(poll_every).await;
                }
            }
        };

        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            panic!(
                "[Http-{}] did not become ready within {:?}",
                self.identifier, timeout
            );
        }

        match self
            .readiness_check
            .is_ready(&self.identifier, &admin_url, remaining.as_millis() as u64)
            .await
        {
            Ok(()) => {}
            Err(err) => panic!("[Http-{}] readiness check failed: {}", self.identifier, err),
        }
    }
}

#[async_trait]
impl RunnableDependency for HttpDependency {
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
        let container_name = self
            .container_name
            .clone()
            .unwrap_or_else(|| self.default_container_name());

        let sw_container = Instant::now();
        self.needs_teardown = true;
        self.http_impl
            .start(self.port, &image_name, &image_tag, &container_name)
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

        self.running = true;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started"
        );
    }

    async fn stop(&mut self) {
        self.http_impl.stop().await;
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

        let admin_url = self.admin_url_or_panic();

        tracing::debug!(
            dependency = %self.identifier,
            phase = "soft_reset",
            "reset mappings and request journal"
        );

        let client = admin_api_client(&admin_url, self.trusted_tls_certificate_pem.as_deref());
        client
            .post(format!("{admin_url}/reset"))
            .send()
            .await
            .unwrap_or_else(|e| panic!("[Http-{}] soft reset failed: {e}", self.identifier));
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }

        tracing::debug!(
            dependency = %self.identifier,
            phase = "hard_reset",
            "restarting http container"
        );

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = self
            .container_name
            .clone()
            .unwrap_or_else(|| self.default_container_name());

        self.http_impl.stop().await;
        self.running = false;

        self.http_impl
            .start(self.port, &image_name, &image_tag, &container_name)
            .await;
        self.wait_until_ready().await;
        self.running = true;
    }
}

impl Drop for HttpDependency {
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
