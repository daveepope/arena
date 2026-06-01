use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Once};

use arc_swap::ArcSwap;
use std::os::raw::c_char;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::reload;
use tracing_subscriber::util::SubscriberInitExt;

use super::env_filter_reload;
use super::severity_level::Level;

#[derive(Clone, Debug)]
pub(crate) struct ArenaEmittedRecord {
    pub severity: Level,
    pub target: String,
    pub payload: String,
    pub unix_timestamp_ns: i64,
    pub caller_file_utf8: Option<String>,
    pub caller_line: u32,
}

pub(crate) trait ArenaLoggingTarget: Send + Sync {
    fn deliver(&self, record: ArenaEmittedRecord);
}

pub(crate) type DispatcherLoggingTargetRef = Arc<dyn ArenaLoggingTarget + Send + Sync>;

#[derive(Clone, Copy)]
struct SlotKey(u64);

#[derive(Clone)]
struct RegisteredTarget {
    key: SlotKey,
    recipient: DispatcherLoggingTargetRef,
}

static REGISTERED_LOG_TARGETS: LazyLock<ArcSwap<Vec<RegisteredTarget>>> =
    LazyLock::new(|| ArcSwap::from_pointee(Vec::new()));
static NEXT_SLOT_KEY: AtomicU64 = AtomicU64::new(1);

static SHARED_DISPATCHER: Once = Once::new();

static DISPATCHER_DEPENDENCY_ALLOW: LazyLock<ArcSwap<Vec<String>>> =
    LazyLock::new(|| ArcSwap::from_pointee(Vec::new()));

static DISPATCHER_COMPONENT_ALLOW: LazyLock<ArcSwap<Vec<String>>> =
    LazyLock::new(|| ArcSwap::from_pointee(Vec::new()));


fn open_slot(recipient: DispatcherLoggingTargetRef) -> SlotKey {
    let key = SlotKey(NEXT_SLOT_KEY.fetch_add(1, Ordering::Relaxed));
    REGISTERED_LOG_TARGETS.rcu(|current| {
        let mut next = (**current).clone();
        next.push(RegisteredTarget {
            key,
            recipient: recipient.clone(),
        });
        next
    });
    key
}

fn release_slot(key: SlotKey) {
    let SlotKey(raw) = key;
    if raw == 0 {
        return;
    }
    REGISTERED_LOG_TARGETS.rcu(|current| {
        let next: Vec<RegisteredTarget> = (**current)
            .iter()
            .filter(|t| t.key.0 != raw)
            .cloned()
            .collect();
        next
    });
}

pub(crate) struct ArenaLogDelivery {
    key: SlotKey,
}

impl ArenaLogDelivery {
    #[must_use]
    pub(crate) fn subscribe(recipient: DispatcherLoggingTargetRef) -> Self {
        Self {
            key: open_slot(recipient),
        }
    }
}

impl Drop for ArenaLogDelivery {
    fn drop(&mut self) {
        let k = std::mem::replace(&mut self.key, SlotKey(0));
        release_slot(k);
    }
}


fn dispatcher_allow_json_bytes_store(bytes: &[u8]) -> Vec<String> {
    serde_json::from_slice::<Vec<String>>(bytes)
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

pub(super) fn dispatcher_dependency_allowlist_store_bytes(bytes: &[u8]) {
    DISPATCHER_DEPENDENCY_ALLOW.store(Arc::new(dispatcher_allow_json_bytes_store(bytes)));
}

pub(super) fn dispatcher_component_allowlist_store_bytes(bytes: &[u8]) {
    DISPATCHER_COMPONENT_ALLOW.store(Arc::new(dispatcher_allow_json_bytes_store(bytes)));
}

pub(super) unsafe fn dispatcher_dependency_allowlist_set_ptr(json_utf8: *const c_char) {
    if json_utf8.is_null() {
        DISPATCHER_DEPENDENCY_ALLOW.store(Arc::new(Vec::new()));
        return;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(json_utf8).to_bytes() };
    dispatcher_dependency_allowlist_store_bytes(bytes);
}

pub(super) unsafe fn dispatcher_component_allowlist_set_ptr(json_utf8: *const c_char) {
    if json_utf8.is_null() {
        DISPATCHER_COMPONENT_ALLOW.store(Arc::new(Vec::new()));
        return;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(json_utf8).to_bytes() };
    dispatcher_component_allowlist_store_bytes(bytes);
}

fn dispatcher_field_kv_tail(payload: &str) -> &str {
    payload
        .rsplit_once('|')
        .map(|(_, rhs)| rhs.trim())
        .unwrap_or("")
}

fn collect_equals_field_values<'a>(tail: &'a str, key_eq: &'a str) -> Vec<&'a str> {
    let mut out = Vec::new();
    for part in tail.split(',') {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix(key_eq) {
            out.push(rest.trim().trim_matches('"'));
        }
    }
    out
}

fn dispatcher_allowlist_always_admits(metadata_target: &str) -> bool {
    metadata_target.starts_with("arena::")
        || metadata_target.starts_with("arena_container::")
        || metadata_target.starts_with("arena_ffi::")
        || metadata_target == "ffi_logging_test"
        || metadata_target.starts_with("ffi_logging_test::")
}

fn allowlist_needles_hit_values<'a>(needles: &[String], vals: &[&'a str]) -> bool {
    needles
        .iter()
        .any(|needle| vals.iter().any(|val| val.contains(needle.as_str())))
}

fn dispatcher_impl_allowlists_allows_delivery(
    metadata_target: &str,
    payload: &str,
    dependency_allowlist: &[String],
    component_allowlist: &[String],
) -> bool {
    if dispatcher_allowlist_always_admits(metadata_target) {
        return true;
    }
    if !metadata_target.starts_with("arena_") {
        return true;
    }
    let tail = dispatcher_field_kv_tail(payload);
    let deps = collect_equals_field_values(tail, "dependency=");
    let comps = collect_equals_field_values(tail, "component=");
    if deps.is_empty() && comps.is_empty() {
        return false;
    }
    match (!deps.is_empty(), !comps.is_empty()) {
        (true, false) => {
            !dependency_allowlist.is_empty()
                && allowlist_needles_hit_values(dependency_allowlist, &deps)
        }
        (false, true) => {
            !component_allowlist.is_empty()
                && allowlist_needles_hit_values(component_allowlist, &comps)
        }
        (true, true) => {
            (!dependency_allowlist.is_empty()
                && allowlist_needles_hit_values(dependency_allowlist, &deps))
                || (!component_allowlist.is_empty()
                    && allowlist_needles_hit_values(component_allowlist, &comps))
        }
        (false, false) => false,
    }
}

pub(crate) fn ensure_shared_tracing_installed() {
    SHARED_DISPATCHER.call_once(|| {
        let env_filter = EnvFilter::new("info");
        let (filter_layer, reload_handle) = reload::Layer::new(env_filter);
        let registry = tracing_subscriber::registry()
            .with(filter_layer)
            .with(DispatcherLayer);

        if registry.try_init().is_ok() {
            env_filter_reload::install_filter_control(Box::new(reload_handle), Level::Info);
        }
    });
}

fn snapshot_log_targets() -> Arc<Vec<RegisteredTarget>> {
    REGISTERED_LOG_TARGETS.load_full()
}

fn host_dispatcher_accepts_metadata_target(target: &str) -> bool {
    target.starts_with("arena::")
        || target.starts_with("arena_")
        || target == "ffi_logging_test"
        || target.starts_with("ffi_logging_test::")
}

fn host_dispatcher_payload_for_host(recording_target: &str, message_body: String) -> String {
    let tag = shorten_tracing_metadata_target_label(recording_target);
    if message_body.is_empty() {
        format!("[{tag}]")
    } else {
        format!("[{tag}] {message_body}")
    }
}

fn shorten_tracing_metadata_target_label(recording_target: &str) -> String {
    const MAX_TARGET_RUNE_LEN: usize = 72;
    if recording_target.chars().count() <= MAX_TARGET_RUNE_LEN {
        return recording_target.to_string();
    }
    let shortened: String = recording_target.chars().take(MAX_TARGET_RUNE_LEN).collect();
    format!("{}…", shortened)
}

fn level_from_event(level: &tracing::Level) -> Level {
    match *level {
        tracing::Level::ERROR => Level::Error,
        tracing::Level::WARN => Level::Warn,
        tracing::Level::INFO => Level::Info,
        tracing::Level::DEBUG => Level::Debug,
        tracing::Level::TRACE => Level::Trace,
    }
}

struct DispatcherLayer;

impl<S: Subscriber> Layer<S> for DispatcherLayer {
    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::TRACE)
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let targets = snapshot_log_targets();
        if targets.is_empty() {
            return;
        }

        let metadata_target = event.metadata().target();
        if !host_dispatcher_accepts_metadata_target(metadata_target) {
            return;
        }

        let severity = level_from_event(event.metadata().level());
        let emitted_at = event
            .metadata()
            .module_path()
            .unwrap_or(metadata_target)
            .to_owned();

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);

        let mut coll = StructuredPayloadCollector::new();
        event.record(&mut coll);
        let payload = host_dispatcher_payload_for_host(metadata_target, coll.into_body());

        let (caller_file_utf8, caller_line) =
            match (event.metadata().file(), event.metadata().line()) {
                (Some(path), Some(line)) => {
                    let basename = std::path::Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned);
                    (basename, line)
                }
                _ => (None, 0),
            };

        let record = ArenaEmittedRecord {
            severity,
            target: emitted_at,
            payload,
            unix_timestamp_ns: ts,
            caller_file_utf8,
            caller_line,
        };

        let dep_allow = DISPATCHER_DEPENDENCY_ALLOW.load_full();
        let comp_allow = DISPATCHER_COMPONENT_ALLOW.load_full();
        if !dispatcher_impl_allowlists_allows_delivery(
            metadata_target,
            &record.payload,
            dep_allow.as_slice(),
            comp_allow.as_slice(),
        ) {
            return;
        }

        for entry in targets.iter() {
            entry.recipient.deliver(record.clone());
        }
    }
}

struct StructuredPayloadCollector {
    message: Option<String>,
    fields: Vec<String>,
}

impl StructuredPayloadCollector {
    fn new() -> Self {
        Self {
            message: None,
            fields: Vec::new(),
        }
    }

    fn append_message_fragment_raw(&mut self, fragment: &str) {
        self.message
            .get_or_insert_with(String::new)
            .push_str(fragment);
    }

    fn append_message_fragment_debug(&mut self, value: &dyn std::fmt::Debug) {
        let piece = format!("{value:?}");
        self.message
            .get_or_insert_with(String::new)
            .push_str(&piece);
    }

    fn push_kv(&mut self, name: &str, formatted: impl std::fmt::Display) {
        self.fields.push(format!("{}={}", name, formatted));
    }

    fn into_body(self) -> String {
        let message = self.message.unwrap_or_default();
        let tail = self.fields.join(", ");
        if message.is_empty() {
            tail
        } else if tail.is_empty() {
            message
        } else {
            format!("{} | {}", message, tail)
        }
    }
}

impl Visit for StructuredPayloadCollector {
    fn record_bool(&mut self, field: &Field, value: bool) {
        if field.name() == "message" {
            self.append_message_fragment_debug(&value);
        } else {
            self.push_kv(field.name(), value);
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if field.name() == "message" {
            self.append_message_fragment_debug(&value);
        } else {
            self.push_kv(field.name(), value);
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "message" {
            self.append_message_fragment_debug(&value);
        } else {
            self.push_kv(field.name(), value);
        }
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        if field.name() == "message" {
            self.append_message_fragment_debug(&value);
        } else {
            self.push_kv(field.name(), value);
        }
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        if field.name() == "message" {
            self.append_message_fragment_debug(&value);
        } else {
            self.push_kv(field.name(), value);
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if field.name() == "message" {
            self.append_message_fragment_debug(&value);
        } else {
            self.push_kv(field.name(), value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.append_message_fragment_raw(value);
        } else {
            self.push_kv(field.name(), format_args!("{value:?}"));
        }
    }

    fn record_bytes(&mut self, field: &Field, value: &[u8]) {
        if field.name() == "message" {
            self.append_message_fragment_debug(&value);
        } else {
            self.push_kv(field.name(), format_args!("{:?}", value));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.append_message_fragment_debug(value);
        } else {
            self.push_kv(field.name(), format_args!("{:?}", value));
        }
    }

    fn record_error(
        &mut self,
        field: &Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if field.name() == "message" {
            self.append_message_fragment_debug(value);
        } else {
            self.push_kv(field.name(), format_args!("{}", value));
        }
    }
}
