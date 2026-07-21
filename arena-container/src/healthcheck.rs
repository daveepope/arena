use std::time::Duration;
use testcontainers_modules::testcontainers::core::Healthcheck;

pub fn tcp_healthcheck(port: u16) -> Healthcheck {
    Healthcheck::cmd_shell(format!("nc -z 127.0.0.1 {port}"))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        .with_retries(40u32)
}
