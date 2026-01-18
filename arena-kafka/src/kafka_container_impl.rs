use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::{kafka, testcontainers, testcontainers::runners::AsyncRunner};
use testcontainers_modules::testcontainers::core::{CmdWaitFor, ContainerPort, ExecCommand, Healthcheck};
use testcontainers_modules::testcontainers::ImageExt;

#[async_trait]
pub trait KafkaImpl: Send + Sync {
    async fn start(&mut self, port: u16, container_tag: &str);
    async fn stop(&mut self);

    async fn exec(&self, cmd: Vec<String>) -> Result<KafkaExecOutput, String>;

    fn bootstrap_servers(&self) -> Option<&str>;
}

pub struct KafkaExecOutput {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}

fn shell_single_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn build_bash_script(cmd: &[String]) -> String {
    let mut joined = String::new();
    for (i, part) in cmd.iter().enumerate() {
        if i > 0 {
            joined.push(' ');
        }
        joined.push_str(&shell_single_quote(part));
    }

    format!(
        "set +e; {joined}; ec=$?; echo __ARENA_EXIT_CODE__=$ec; exit 0",
        joined = joined
    )
}

fn parse_exit_code_marker(stdout: &str) -> (i64, String) {
    let mut lines: Vec<&str> = stdout.lines().collect();
    if let Some(last) = lines.last().copied() {
        const PREFIX: &str = "__ARENA_EXIT_CODE__=";
        if let Some(raw) = last.strip_prefix(PREFIX) {
            if let Ok(code) = raw.trim().parse::<i64>() {
                lines.pop();
                let cleaned = lines.join("\n");
                return (code, cleaned);
            }
        }
    }

    (-1, stdout.to_string())
}

pub(crate) struct KafkaContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<kafka::apache::Kafka>>,
    bootstrap: Option<String>,
}

impl KafkaContainerImpl {
    pub(crate) fn new() -> Self {
        Self {
            container: None,
            bootstrap: None,
        }
    }
}

#[async_trait]
impl KafkaImpl for KafkaContainerImpl {
    async fn start(&mut self, port: u16, container_tag: &str) {
        if self.container.is_some() {
            return;
        }

        const DEFAULT_CONTAINER_PORT: ContainerPort = kafka::apache::KAFKA_PORT;

        // Internal "good enough" healthcheck for now. We'll make it configurable later.
        //
        // This relies on bash's /dev/tcp support. If this ever fails on a future image,
        // we'll swap to a more explicit Kafka readiness command.
        let healthcheck = Healthcheck::cmd_shell(format!(
            "bash -lc 'echo > /dev/tcp/127.0.0.1/{port}'",
            port = DEFAULT_CONTAINER_PORT.as_u16()
        ))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        // 10s / 250ms = 40 attempts (ish)
        .with_retries(40u32);

        let container = kafka::apache::Kafka::default()
            .with_tag(container_tag)
            .with_mapped_port(port, DEFAULT_CONTAINER_PORT)
            .with_health_check(healthcheck)
            .start()
            .await
            .expect("start kafka container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .expect("Failed to get port")
            .to_string();

        self.bootstrap = Some(format!("{host}:{port}"));
        self.container = Some(container);

        log::info!("[KafkaImpl] started container.");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.bootstrap = None;
        log::info!("[KafkaImpl] stopped container.");
    }

    async fn exec(&self, cmd: Vec<String>) -> Result<KafkaExecOutput, String> {
        let container = self
            .container
            .as_ref()
            .ok_or_else(|| "kafka container not started".to_string())?;

        let script = build_bash_script(&cmd);
        let mut res = container
            .exec(
                ExecCommand::new(["bash", "-lc", &script])
                    .with_cmd_ready_condition(CmdWaitFor::exit()),
            )
            .await
            .map_err(|e| format!("exec failed: {e:?}"))?;

        let stdout = String::from_utf8_lossy(
            &res.stdout_to_vec().await.map_err(|e| format!("exec stdout failed: {e:?}"))?,
        )
        .to_string();
        let stderr = String::from_utf8_lossy(
            &res.stderr_to_vec().await.map_err(|e| format!("exec stderr failed: {e:?}"))?,
        )
        .to_string();

        let (exit_code, stdout) = parse_exit_code_marker(&stdout);

        Ok(KafkaExecOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}

pub(crate) struct ConfluentKafkaContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<kafka::confluent::Kafka>>,
    bootstrap: Option<String>,
}

impl ConfluentKafkaContainerImpl {
    pub(crate) fn new() -> Self {
        Self {
            container: None,
            bootstrap: None,
        }
    }
}

#[async_trait]
impl KafkaImpl for ConfluentKafkaContainerImpl {
    async fn start(&mut self, port: u16, container_tag: &str) {
        if self.container.is_some() {
            return;
        }

        const DEFAULT_CONTAINER_PORT: ContainerPort = kafka::confluent::KAFKA_PORT;

        // Internal "good enough" healthcheck for now. We'll make it configurable later.
        let healthcheck = Healthcheck::cmd_shell(format!(
            "bash -lc 'echo > /dev/tcp/127.0.0.1/{port}'",
            port = DEFAULT_CONTAINER_PORT.as_u16()
        ))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        .with_retries(40u32);

        let container = kafka::confluent::Kafka::default()
            .with_tag(container_tag)
            .with_mapped_port(port, DEFAULT_CONTAINER_PORT)
            .with_health_check(healthcheck)
            .start()
            .await
            .expect("start kafka container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .expect("Failed to get port")
            .to_string();

        self.bootstrap = Some(format!("{host}:{port}"));
        self.container = Some(container);

        log::info!("[KafkaImpl] started container.");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.bootstrap = None;
        log::info!("[KafkaImpl] stopped container.");
    }

    async fn exec(&self, cmd: Vec<String>) -> Result<KafkaExecOutput, String> {
        let container = self
            .container
            .as_ref()
            .ok_or_else(|| "kafka container not started".to_string())?;

        let script = build_bash_script(&cmd);
        let mut res = container
            .exec(
                ExecCommand::new(["bash", "-lc", &script])
                    .with_cmd_ready_condition(CmdWaitFor::exit()),
            )
            .await
            .map_err(|e| format!("exec failed: {e:?}"))?;

        let stdout = String::from_utf8_lossy(
            &res.stdout_to_vec().await.map_err(|e| format!("exec stdout failed: {e:?}"))?,
        )
        .to_string();
        let stderr = String::from_utf8_lossy(
            &res.stderr_to_vec().await.map_err(|e| format!("exec stderr failed: {e:?}"))?,
        )
        .to_string();

        let (exit_code, stdout) = parse_exit_code_marker(&stdout);

        Ok(KafkaExecOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}

