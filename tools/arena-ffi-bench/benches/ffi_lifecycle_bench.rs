use std::ffi::{CStr, CString};
use std::hint::black_box;
use std::os::raw::c_char;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use arena_ffi::{arena_close, arena_free_string, arena_open, parse_config_for_bench};

const MINIMAL_CONFIG: &str = "{}";

const SINGLE_DEPENDENCY_CONFIG: &str = r#"{
    "dependencies": [
        {"type": "http", "identifier": "http"}
    ]
}"#;

const MULTI_DEPENDENCY_WITH_PLAYBOOK_CONFIG: &str = r#"{
    "match_name": "bench-match",
    "network": "arena-net",
    "dependencies": [
        {"type": "postgres", "identifier": "pg"},
        {"type": "mssql", "identifier": "mssql"},
        {"type": "kafka", "identifier": "kafka"},
        {"type": "http", "identifier": "http"},
        {"type": "localstack", "identifier": "localstack"},
        {"type": "oauth", "identifier": "oauth"},
        {"type": "temporal", "identifier": "temporal"},
        {"type": "smtp", "identifier": "smtp"}
    ],
    "components": [
        {"type": "exec", "identifier": "exec", "executable_path": "/bin/true"}
    ],
    "playbooks": [{
        "identifier": "pb",
        "kind": "http",
        "dependency_identifier": "http",
        "mappings": []
    }]
}"#;

fn realistic_large_config() -> String {
    let instrument_reading_sql = "CREATE TABLE instrument_reading (id BIGSERIAL PRIMARY KEY, instrument_id TEXT NOT NULL, reading_value DOUBLE PRECISION NOT NULL, recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()); ".repeat(11);
    let validation_sql = "CREATE TABLE validation_run (id BIGSERIAL PRIMARY KEY, status TEXT NOT NULL); ".repeat(5);
    format!(
        r#"{{
        "match_name": "example-api-chained-match",
        "network": "arena-net",
        "dependencies": [
            {{"type": "oauth", "identifier": "example-api-chained-oauth"}},
            {{"type": "mssql", "identifier": "example-api-chained-mssql", "startup_sql_scripts": [{:?}]}},
            {{"type": "http", "identifier": "example-api-chained-calibration"}},
            {{"type": "localstack", "identifier": "example-api-chained-localstack"}},
            {{
                "type": "temporal",
                "identifier": "example-api-chained-temporal",
                "children": [
                    {{"type": "postgres", "identifier": "example-api-chained-postgres", "startup_sql_scripts": [{:?}]}}
                ]
            }},
            {{"type": "smtp", "identifier": "example-api-chained-smtp"}}
        ],
        "components": [
            {{"type": "exec", "identifier": "example-api-chained-web-app", "executable_path": "/bin/true"}}
        ],
        "playbooks": [
            {{"identifier": "calibration-happy-path", "kind": "http", "dependency_identifier": "example-api-chained-calibration", "mappings": []}},
            {{"identifier": "events-purge", "kind": "http", "dependency_identifier": "example-api-chained-localstack", "mappings": []}},
            {{"identifier": "reset-readings-db", "kind": "http", "dependency_identifier": "example-api-chained-postgres", "mappings": []}}
        ]
    }}"#,
        instrument_reading_sql, validation_sql
    )
}

fn bench_parse_config(c: &mut Criterion) {
    let realistic_large = realistic_large_config();
    let mut group = c.benchmark_group("parse_config");
    for (label, payload) in [
        ("minimal", MINIMAL_CONFIG),
        ("single_dependency", SINGLE_DEPENDENCY_CONFIG),
        (
            "multi_dependency_with_playbook",
            MULTI_DEPENDENCY_WITH_PLAYBOOK_CONFIG,
        ),
        ("realistic_large_example", realistic_large.as_str()),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(label), payload, |b, payload| {
            b.iter(|| parse_config_for_bench(black_box(payload)).unwrap());
        });
    }
    group.finish();
}

fn bench_open_close_round_trip(c: &mut Criterion) {
    c.bench_function("arena_open_close_round_trip_empty_config", |b| {
        b.iter(|| {
            let name = CString::new("bench-arena").unwrap();
            let mut err: *mut c_char = std::ptr::null_mut();
            let handle = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _, std::ptr::null_mut());
            assert!(!handle.is_null(), "expected handle, got error: {:?}", unsafe {
                if err.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(err).to_string_lossy().into_owned())
                }
            });
            arena_close(handle, std::ptr::null_mut(), std::ptr::null_mut());
            arena_free_string(err);
        });
    });
}

criterion_group!(benches, bench_parse_config, bench_open_close_round_trip);
criterion_main!(benches);
