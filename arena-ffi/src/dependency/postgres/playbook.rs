use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use arena_postgres::ActivePlaybook;
use serde::Deserialize;

use crate::active_playbook::{ActivePlaybookInner, ArenaActivePlaybookHandle};
use crate::error::{clear_error, write_error};
use crate::panic_payload::panic_message;
use crate::strings::c_str_to_string;
use crate::ArenaStatus;

#[derive(Debug, Deserialize)]
struct VerifySpec {
    #[serde(default)]
    dependency_identifier: Option<String>,
    query: String,
    expected_value: i32,
}

#[no_mangle]
pub extern "C" fn arena_postgres_playbook_verify(
    handle: *mut ArenaActivePlaybookHandle,
    verify_spec: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };

    if handle.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_postgres_playbook_verify: handle must not be null",
            )
        };
        return ArenaStatus::InvalidArgument;
    }
    if verify_spec.is_null() {
        unsafe {
            write_error(
                err_out,
                "arena_postgres_playbook_verify: verify_spec must not be null",
            )
        };
        return ArenaStatus::InvalidArgument;
    }

    let spec_str = match unsafe { c_str_to_string(verify_spec) } {
        Some(v) => v,
        None => {
            unsafe {
                write_error(
                    err_out,
                    "arena_postgres_playbook_verify: verify_spec is not valid UTF-8",
                )
            };
            return ArenaStatus::InvalidArgument;
        }
    };

    let parsed: VerifySpec = match serde_json::from_str(&spec_str) {
        Ok(v) => v,
        Err(e) => {
            unsafe {
                write_error(
                    err_out,
                    format!("arena_postgres_playbook_verify: parse failed: {e}"),
                )
            };
            return ArenaStatus::InvalidArgument;
        }
    };
    let _ = parsed.dependency_identifier;

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        let inner = unsafe { ActivePlaybookInner::as_ref(handle) };
        let active = inner
            .active
            .as_ref()
            .ok_or_else(|| "playbook is already dropped".to_string())?;

        let postgres_active = active
            .as_any()
            .downcast_ref::<ActivePlaybook>()
            .ok_or_else(|| "playbook handle is not a Postgres playbook".to_string())?;

        let actual = inner
            .runtime_handle
            .block_on(async { postgres_active.verify(&parsed.query).await });

        if actual != parsed.expected_value {
            return Err(format!(
                "verify failed for query {:?}: expected {}, got {}",
                parsed.query, parsed.expected_value, actual
            ));
        }
        Ok(())
    }));

    match outcome {
        Ok(Ok(())) => ArenaStatus::Ok,
        Ok(Err(msg)) => {
            unsafe { write_error(err_out, format!("arena_postgres_playbook_verify: {msg}")) };
            ArenaStatus::Failed
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(error = %msg, op = "postgres_playbook_verify", "playbook verify failed");
            unsafe { write_error(err_out, format!("arena_postgres_playbook_verify: {msg}")) };
            ArenaStatus::Failed
        }
    }
}
