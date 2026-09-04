use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use async_trait::async_trait;
use serde::Deserialize;

use crate::active_playbook::{ActivePlaybookInner, ArenaActivePlaybookHandle};
use crate::error::write_error;
use crate::panic_payload::panic_message;
use crate::ArenaStatus;

#[async_trait]
pub(crate) trait PlaybookQueryVerify: Send + Sync {
    async fn verify_query(&self, query: &str) -> i32;
}

#[derive(Debug, Deserialize)]
struct VerifySpec {
    #[serde(default)]
    dependency_identifier: Option<String>,
    query: String,
    expected_value: i32,
}

pub(crate) fn verify_playbook_query<T>(
    handle: *mut ArenaActivePlaybookHandle,
    verify_spec: *const c_char,
    err_out: *mut *mut c_char,
    fn_name: &str,
    dependency_label: &str,
) -> ArenaStatus
where
    T: PlaybookQueryVerify + 'static,
{
    use crate::error::clear_error;
    use crate::strings::c_str_to_string;

    unsafe { clear_error(err_out) };

    if handle.is_null() {
        unsafe { write_error(err_out, format!("{fn_name}: handle must not be null")) };
        return ArenaStatus::InvalidArgument;
    }
    if verify_spec.is_null() {
        unsafe { write_error(err_out, format!("{fn_name}: verify_spec must not be null")) };
        return ArenaStatus::InvalidArgument;
    }

    let spec_str = match unsafe { c_str_to_string(verify_spec) } {
        Some(v) => v,
        None => {
            unsafe { write_error(err_out, format!("{fn_name}: verify_spec is not valid UTF-8")) };
            return ArenaStatus::InvalidArgument;
        }
    };

    let parsed: VerifySpec = match catch_unwind(AssertUnwindSafe(|| serde_json::from_str(&spec_str)))
    {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            unsafe { write_error(err_out, format!("{fn_name}: parse failed: {e}")) };
            return ArenaStatus::InvalidArgument;
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            unsafe { write_error(err_out, format!("{fn_name} failed: {msg}")) };
            return ArenaStatus::Failed;
        }
    };
    let _ = parsed.dependency_identifier;

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        let inner = unsafe { ActivePlaybookInner::as_ref(handle) };
        let active = inner
            .active
            .as_ref()
            .ok_or_else(|| "playbook is already dropped".to_string())?;

        let typed_active = active
            .as_any()
            .downcast_ref::<T>()
            .ok_or_else(|| format!("playbook handle is not a {dependency_label} playbook"))?;

        let actual = inner
            .runtime_handle
            .block_on(async { typed_active.verify_query(&parsed.query).await });

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
            unsafe { write_error(err_out, format!("{fn_name}: {msg}")) };
            ArenaStatus::Failed
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            tracing::error!(error = %msg, op = fn_name, "playbook verify failed");
            unsafe { write_error(err_out, format!("{fn_name}: {msg}")) };
            ArenaStatus::Failed
        }
    }
}
