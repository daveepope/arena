load("@rules_rust//rust:defs.bzl", "rust_library", "rust_shared_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

_ARENA_FFI_DEPS = all_crate_deps(normal = True) + [
    "//arena",
    "//arena-containerized-component:arena_containerized_component",
    "//arena-executable-component:arena_executable_component",
    "//arena-http:arena_http",
    "//arena-kafka:arena_kafka",
    "//arena-localstack:arena_localstack",
    "//arena-mssql:arena_mssql",
    "//arena-oauth:arena_oauth",
    "//arena-oracledb:arena_oracledb",
    "//arena-postgres:arena_postgres",
    "//arena-temporal:arena_temporal",
    "//arena-smtp:arena_smtp",
]

_ARENA_FFI_PROC_MACRO_DEPS = all_crate_deps(proc_macro = True)

rust_library(
    name = "arena_ffi",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = _ARENA_FFI_DEPS,
    proc_macro_deps = _ARENA_FFI_PROC_MACRO_DEPS,
    visibility = ["//visibility:public"],
)

rust_shared_library(
    name = "arena_ffi_shared",
    crate_name = "arena_ffi",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = _ARENA_FFI_DEPS,
    proc_macro_deps = _ARENA_FFI_PROC_MACRO_DEPS,
    visibility = ["//visibility:public"],
)

rust_test(
    name = "ffi_healthcheck_test",
    srcs = ["tests/unit/healthcheck_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        "//arena",
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_containerized_component_test",
    srcs = ["tests/component/containerized/containerized_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        "//arena",
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_matches_test",
    srcs = ["tests/unit/matches_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_managed_playbook_test",
    srcs = ["tests/unit/managed_playbook_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_postgres_dependency_test",
    srcs = ["tests/unit/dependency/postgres/postgres_dependency_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_http_mapping_spec_test",
    srcs = ["tests/unit/dependency/http/http_mapping_spec.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_playbook_validation_test",
    srcs = [
        "tests/unit/dependency/http/http_playbook_ffi_validation.rs",
        "tests/unit/ffi_error_text.rs",
    ],
    crate_root = "tests/unit/dependency/http/http_playbook_ffi_validation.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_mssql_dependency_test",
    srcs = ["tests/unit/dependency/mssql/mssql_dependency_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_mssql_playbook_validation_test",
    srcs = [
        "tests/unit/dependency/mssql/mssql_playbook_ffi_validation.rs",
        "tests/unit/ffi_error_text.rs",
    ],
    crate_root = "tests/unit/dependency/mssql/mssql_playbook_ffi_validation.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_oracle_playbook_validation_test",
    srcs = [
        "tests/unit/dependency/oracle/oracle_playbook_ffi_validation.rs",
        "tests/unit/ffi_error_text.rs",
    ],
    crate_root = "tests/unit/dependency/oracle/oracle_playbook_ffi_validation.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_oracle_dependency_test",
    srcs = ["tests/unit/dependency/oracle/oracle_dependency_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_postgres_playbook_validation_test",
    srcs = [
        "tests/unit/dependency/postgres/postgres_playbook_ffi_validation.rs",
        "tests/unit/ffi_error_text.rs",
    ],
    crate_root = "tests/unit/dependency/postgres/postgres_playbook_ffi_validation.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_executable_component_test",
    srcs = ["tests/unit/component/executable/executable_component_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_containerized_component_unit_test",
    srcs = ["tests/unit/component/containerized/containerized_component_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_error_test",
    srcs = ["tests/unit/error_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_strings_test",
    srcs = ["tests/unit/strings_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_panic_payload_test",
    srcs = ["tests/unit/panic_payload_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_runtime_args_test",
    srcs = ["tests/unit/runtime_args_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_active_playbook_test",
    srcs = [
        "tests/unit/active_playbook_test.rs",
        "tests/unit/ffi_error_text.rs",
    ],
    crate_root = "tests/unit/active_playbook_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_loopback_tls_test",
    srcs = ["tests/unit/loopback_tls_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_arena_test",
    srcs = [
        "tests/unit/arena_test.rs",
        "tests/unit/ffi_error_text.rs",
    ],
    crate_root = "tests/unit/arena_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_temporal_dependency_test",
    srcs = ["tests/unit/dependency/temporal/temporal_dependency_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_smtp_dependency_test",
    srcs = ["tests/unit/dependency/smtp/smtp_dependency_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "ffi_logging_test",
    srcs = ["tests/unit/logging/logging_ffi_callbacks.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_ffi",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
