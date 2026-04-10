//! JSON descriptors for [`arena::healthcheck::ReadinessCheck`] on FFI-built components.
//!
//! ## Extending the contract
//!
//! When adding a new readiness kind that Rust should run for Python/Java/.NET clients:
//!
//! 1. Add a variant to [`crate::parse::ReadinessCheckJson`] (serde `tag = "kind"`).
//! 2. Add a matching arm to [`apply_readiness_check_json`] below (one place).
//! 3. Update each language client that emits `readiness_checks` (e.g. Python
//!    [`arena_pytest::_ffi_readiness`]).
//!
//! Custom checks that cannot be represented in JSON continue to run client-side after
//! `arena_open` (see arena-pytest `Encounter.readiness_hooks`).

use crate::http_readiness::HttpReadinessCheck;
use crate::parse::{ContainerJson, ExecJson, ReadinessCheckJson};
use arena_container_component::builder::ContainerComponentBuilder;
use arena_executable_component::builder::ExecutableComponentBuilder;

/// Collect readiness descriptors for an exec component, including legacy `readiness_check_url`.
pub(crate) fn collect_exec_readiness(json: &ExecJson) -> Vec<ReadinessCheckJson> {
    let mut v = json.readiness_checks.clone().unwrap_or_default();
    if v.is_empty() {
        if let Some(ref url) = json.readiness_check_url {
            v.push(ReadinessCheckJson::Http {
                target: url.clone(),
            });
        }
    }
    v
}

pub(crate) fn collect_container_readiness(json: &ContainerJson) -> Vec<ReadinessCheckJson> {
    json.readiness_checks.clone().unwrap_or_default()
}

pub(crate) fn apply_readiness_checks_to_exec(
    mut builder: ExecutableComponentBuilder,
    checks: &[ReadinessCheckJson],
) -> ExecutableComponentBuilder {
    for c in checks {
        builder = apply_readiness_check_json(builder, c);
    }
    builder
}

pub(crate) fn apply_readiness_checks_to_container(
    mut builder: ContainerComponentBuilder,
    checks: &[ReadinessCheckJson],
) -> ContainerComponentBuilder {
    for c in checks {
        builder = apply_readiness_check_json(builder, c);
    }
    builder
}

/// Single dispatch: JSON descriptor → `with_readiness_check(...)`.
fn apply_readiness_check_json<B: ReadinessCheckAttach>(builder: B, check: &ReadinessCheckJson) -> B {
    match check {
        ReadinessCheckJson::Http { target } => {
            builder.attach_http_readiness(target.as_str())
        }
    }
}

/// Both component builders expose the same attachment API; this keeps new kinds in one match.
trait ReadinessCheckAttach: Sized {
    fn attach_http_readiness(self, target: &str) -> Self;
}

impl ReadinessCheckAttach for ExecutableComponentBuilder {
    fn attach_http_readiness(self, target: &str) -> Self {
        self.with_readiness_check(HttpReadinessCheck::new(), target)
    }
}

impl ReadinessCheckAttach for ContainerComponentBuilder {
    fn attach_http_readiness(self, target: &str) -> Self {
        self.with_readiness_check(HttpReadinessCheck::new(), target)
    }
}
