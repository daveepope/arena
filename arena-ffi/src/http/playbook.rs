use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use arena_http::{
    ActivePlaybook, HttpDependency, PlaybookSequenceBuilder, ResponseDefinition,
    get_requested_for, post_requested_for, put_requested_for,
    delete_requested_for, RequestCriteria,
};
use serde::Deserialize;

use crate::ffi::error::{clear_error, write_error};
use crate::ffi::{ArenaHandle, ArenaStatus};
use crate::ffi::handle::HandleInner;
use crate::ffi::strings::c_str_to_string;

#[repr(C)]
pub struct ArenaHttpPlaybookHandle {
    _private: [u8; 0],
}

struct PlaybookInner {
    runtime_handle: tokio::runtime::Handle,
    active: Option<ActivePlaybook>,
}

impl PlaybookInner {
    fn into_raw(self) -> *mut ArenaHttpPlaybookHandle {
        Box::into_raw(Box::new(self)) as *mut ArenaHttpPlaybookHandle
    }

    unsafe fn from_raw(ptr: *mut ArenaHttpPlaybookHandle) -> Box<PlaybookInner> {
        unsafe { Box::from_raw(ptr as *mut PlaybookInner) }
    }

    unsafe fn as_ref<'a>(ptr: *mut ArenaHttpPlaybookHandle) -> &'a PlaybookInner {
        unsafe { &*(ptr as *const PlaybookInner) }
    }
}

#[derive(Debug, Deserialize)]
struct PlaybookSpec {
    dependency_identifier: String,
    mappings: Vec<MappingSpec>,
}

#[derive(Debug, Deserialize)]
struct MappingSpec {
    method: String,
    url_path: String,
    #[serde(default)]
    priority: Option<u32>,
    response: ResponseSpec,
    #[serde(default)]
    expect: Option<ExpectSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExpectSpec {
    Exactly { count: u64 },
    AtLeast { count: u64 },
    Never,
}

#[derive(Debug, Deserialize)]
struct ResponseSpec {
    #[serde(default = "default_status")]
    status: u16,
    #[serde(default)]
    json_body: Option<serde_json::Value>,
}

fn default_status() -> u16 {
    200
}

#[derive(Debug, Deserialize)]
struct VerifySpec {
    dependency_identifier: String,
    method: String,
    url_path: String,
    expected_count: u64,
}

fn response_def(spec: &ResponseSpec) -> ResponseDefinition {
    let mut r = ResponseDefinition::new(spec.status);
    if let Some(ref body) = spec.json_body {
        r = r.with_json_body(body.clone());
    }
    r
}

fn apply_expect(seq: PlaybookSequenceBuilder, expect: &Option<ExpectSpec>) -> PlaybookSequenceBuilder {
    match expect {
        Some(ExpectSpec::Exactly { count }) => seq.expect_called(*count),
        Some(ExpectSpec::AtLeast { count }) => seq.expect_called_at_least(*count),
        Some(ExpectSpec::Never) => seq.expect_never_called(),
        None => seq,
    }
}

fn first_sequence(
    http: &HttpDependency,
    m: &MappingSpec,
) -> Result<PlaybookSequenceBuilder, String> {
    let resp = response_def(&m.response);
    let playbook = http.playbook();
    let builder = match m.method.to_ascii_uppercase().as_str() {
        "GET" => playbook.get(&m.url_path),
        "POST" => playbook.post(&m.url_path),
        "PUT" => playbook.put(&m.url_path),
        "DELETE" => playbook.delete(&m.url_path),
        other => return Err(format!("unsupported HTTP method: {other}")),
    };
    let builder = match m.priority {
        Some(p) => builder.with_priority(p),
        None => builder,
    };
    Ok(apply_expect(builder.will_return(resp), &m.expect))
}

fn append_mapping(
    seq: PlaybookSequenceBuilder,
    m: &MappingSpec,
) -> Result<PlaybookSequenceBuilder, String> {
    let resp = response_def(&m.response);
    let builder = match m.method.to_ascii_uppercase().as_str() {
        "GET" => seq.get(&m.url_path),
        "POST" => seq.post(&m.url_path),
        "PUT" => seq.put(&m.url_path),
        "DELETE" => seq.delete(&m.url_path),
        other => return Err(format!("unsupported HTTP method: {other}")),
    };
    let builder = match m.priority {
        Some(p) => builder.with_priority(p),
        None => builder,
    };
    Ok(apply_expect(builder.will_return(resp), &m.expect))
}

fn criteria_for(method: &str, url_path: &str) -> Result<RequestCriteria, String> {
    Ok(match method.to_ascii_uppercase().as_str() {
        "GET" => get_requested_for(url_path),
        "POST" => post_requested_for(url_path),
        "PUT" => put_requested_for(url_path),
        "DELETE" => delete_requested_for(url_path),
        other => return Err(format!("unsupported HTTP method: {other}")),
    })
}

fn with_http_dependency<F, R>(
    inner: &HandleInner,
    identifier: &str,
    f: F,
) -> Result<R, String>
where
    F: FnOnce(&HttpDependency) -> R,
{
    let guard = inner
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let arena = guard
        .as_ref()
        .ok_or_else(|| "arena is already closed".to_string())?;
    let dep = arena
        .dependency(identifier)
        .ok_or_else(|| format!("dependency '{identifier}' not found"))?;
    let http = dep
        .as_any()
        .downcast_ref::<HttpDependency>()
        .ok_or_else(|| format!("dependency '{identifier}' is not an HttpDependency"))?;
    Ok(f(http))
}

#[no_mangle]
pub extern "C" fn arena_http_playbook_open(
    arena_handle: *mut ArenaHandle,
    spec: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut ArenaHttpPlaybookHandle {
    unsafe { clear_error(err_out) };

    if arena_handle.is_null() {
        unsafe { write_error(err_out, "arena_http_playbook_open: arena handle must not be null") };
        return std::ptr::null_mut();
    }
    if spec.is_null() {
        unsafe { write_error(err_out, "arena_http_playbook_open: spec must not be null") };
        return std::ptr::null_mut();
    }

    let spec_str = match unsafe { c_str_to_string(spec) } {
        Some(v) => v,
        None => {
            unsafe { write_error(err_out, "arena_http_playbook_open: spec is not valid UTF-8") };
            return std::ptr::null_mut();
        }
    };

    let parsed: PlaybookSpec = match serde_json::from_str(&spec_str) {
        Ok(v) => v,
        Err(e) => {
            unsafe { write_error(err_out, format!("arena_http_playbook_open: spec parse failed: {e}")) };
            return std::ptr::null_mut();
        }
    };

    if parsed.mappings.is_empty() {
        unsafe { write_error(err_out, "arena_http_playbook_open: mappings must not be empty") };
        return std::ptr::null_mut();
    }

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<PlaybookInner, String> {
        let inner = unsafe { HandleInner::as_ref(arena_handle) };
        let runtime_handle = inner.runtime.handle().clone();

        let seq = with_http_dependency(inner, &parsed.dependency_identifier, |http| {
            let mut seq = first_sequence(http, &parsed.mappings[0])?;
            for m in parsed.mappings.iter().skip(1) {
                seq = append_mapping(seq, m)?;
            }
            Ok::<PlaybookSequenceBuilder, String>(seq)
        })??;

        let active = runtime_handle.block_on(async move { seq.run().await });

        Ok(PlaybookInner {
            runtime_handle,
            active: Some(active),
        })
    }));

    match outcome {
        Ok(Ok(inner)) => inner.into_raw(),
        Ok(Err(msg)) => {
            log::error!("arena_http_playbook_open failed: {msg}");
            unsafe { write_error(err_out, format!("arena_http_playbook_open: {msg}")) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            log::error!("panic in arena_http_playbook_open: {msg}");
            unsafe { write_error(err_out, format!("panic in arena_http_playbook_open: {msg}")) };
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_http_playbook_close(
    handle: *mut ArenaHttpPlaybookHandle,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };
    if handle.is_null() {
        return ArenaStatus::Ok;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _dropped = unsafe { PlaybookInner::from_raw(handle) };
    }));
    match outcome {
        Ok(()) => ArenaStatus::Ok,
        Err(payload) => {
            let msg = panic_message(&payload);
            log::error!("arena_http_playbook_close: {msg}");
            unsafe { write_error(err_out, format!("arena_http_playbook_close: {msg}")) };
            ArenaStatus::Failed
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_http_playbook_verify(
    handle: *mut ArenaHttpPlaybookHandle,
    verify_spec: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };

    if handle.is_null() {
        unsafe { write_error(err_out, "arena_http_playbook_verify: handle must not be null") };
        return ArenaStatus::InvalidArgument;
    }
    if verify_spec.is_null() {
        unsafe { write_error(err_out, "arena_http_playbook_verify: verify_spec must not be null") };
        return ArenaStatus::InvalidArgument;
    }

    let spec_str = match unsafe { c_str_to_string(verify_spec) } {
        Some(v) => v,
        None => {
            unsafe { write_error(err_out, "arena_http_playbook_verify: verify_spec is not valid UTF-8") };
            return ArenaStatus::InvalidArgument;
        }
    };

    let parsed: VerifySpec = match serde_json::from_str(&spec_str) {
        Ok(v) => v,
        Err(e) => {
            unsafe { write_error(err_out, format!("arena_http_playbook_verify: parse failed: {e}")) };
            return ArenaStatus::InvalidArgument;
        }
    };
    let _ = parsed.dependency_identifier;

    let criteria = match criteria_for(&parsed.method, &parsed.url_path) {
        Ok(c) => c,
        Err(e) => {
            unsafe { write_error(err_out, format!("arena_http_playbook_verify: {e}")) };
            return ArenaStatus::InvalidArgument;
        }
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let inner = unsafe { PlaybookInner::as_ref(handle) };
        let active = match inner.active.as_ref() {
            Some(a) => a,
            None => return Err("playbook is already closed".to_string()),
        };
        inner.runtime_handle.block_on(async {
            active.verify(parsed.expected_count, criteria).await;
        });
        Ok(())
    }));

    match outcome {
        Ok(Ok(())) => ArenaStatus::Ok,
        Ok(Err(msg)) => {
            unsafe { write_error(err_out, format!("arena_http_playbook_verify: {msg}")) };
            ArenaStatus::Failed
        }
        Err(payload) => {
            let msg = panic_message(&payload);
            log::error!("arena_http_playbook_verify failed: {msg}");
            unsafe { write_error(err_out, format!("arena_http_playbook_verify: {msg}")) };
            ArenaStatus::Failed
        }
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}
