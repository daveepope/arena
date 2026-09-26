use bollard::query_parameters::ListContainersOptionsBuilder;
use bollard::Docker;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DEFAULT_EXPIRY: Duration = Duration::from_secs(300);
pub const MODULE_LABEL: &str = "dev.arena.module";
pub const EXPIRES_AT_LABEL: &str = "dev.arena.expires-at";

pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0)
}

pub fn expiry_labels(module: &str, expiry: Duration) -> Vec<(String, String)> {
    vec![
        (MODULE_LABEL.to_string(), module.to_string()),
        (
            EXPIRES_AT_LABEL.to_string(),
            (now_millis().saturating_add(expiry.as_millis())).to_string(),
        ),
    ]
}

pub fn is_expired(expires_at: Option<&String>, now: u128) -> bool {
    match expires_at.and_then(|value| value.parse::<u128>().ok()) {
        Some(deadline) => deadline <= now,
        None => false,
    }
}

pub async fn remove_expired_containers(module: &str) {
    let Some(docker) = Docker::connect_with_defaults().ok() else {
        tracing::warn!(module = %module, "container runtime unavailable; skip expiry sweep");
        return;
    };

    let filters = HashMap::from([(
        "label".to_string(),
        vec![format!("{MODULE_LABEL}={module}")],
    )]);
    let options = ListContainersOptionsBuilder::new()
        .all(true)
        .filters(&filters)
        .build();

    let containers = match docker.list_containers(Some(options)).await {
        Ok(containers) => containers,
        Err(e) => {
            tracing::warn!(module = %module, error = %e, "expiry sweep listing failed");
            return;
        }
    };

    let now = now_millis();
    for container in containers {
        let labels = container.labels.unwrap_or_default();
        if !is_expired(labels.get(EXPIRES_AT_LABEL), now) {
            continue;
        }
        let Some(id) = container.id else {
            continue;
        };
        match docker
            .remove_container(
                &id,
                Some(
                    bollard::query_parameters::RemoveContainerOptionsBuilder::default()
                        .force(true)
                        .build(),
                ),
            )
            .await
        {
            Ok(_) => tracing::debug!(module = %module, container = %id, "removed expired container"),
            Err(e) => tracing::warn!(
                module = %module,
                container = %id,
                error = %e,
                "expired container remove failed"
            ),
        }
    }
}

pub fn expiry_labels_for(module: &str, expiry: Option<Duration>) -> Vec<(String, String)> {
    match expiry {
        Some(expiry) if !expiry.is_zero() => expiry_labels(module, expiry),
        _ => Vec::new(),
    }
}

pub const SWEEP_INTERVAL: Duration = Duration::from_secs(60);

static SWEPT_MODULES: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn claim_sweep(module: &str, now: Instant) -> bool {
    let mut swept = SWEPT_MODULES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match swept.get(module) {
        Some(last) if now.duration_since(*last) < SWEEP_INTERVAL => false,
        _ => {
            swept.insert(module.to_string(), now);
            true
        }
    }
}

pub async fn remove_expired_containers_if_enabled(module: &str, expiry: Option<Duration>) {
    if expiry.is_some_and(|expiry| !expiry.is_zero()) && claim_sweep(module, Instant::now()) {
        remove_expired_containers(module).await;
    }
}
