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
    name = "ffi_lib_unit_test",
    crate = ":arena_ffi",
    size = "small",
    edition = "2021",
)

rust_test(
    name = "ffi_healthcheck_test",
    srcs = ["tests/healthcheck_test.rs"],
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
    srcs = ["tests/containerized_component_test.rs"],
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
    name = "ffi_http_mapping_spec_test",
    srcs = ["tests/playbook/http_mapping_spec.rs"],
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
    srcs = ["tests/playbook/http_playbook_ffi_validation.rs"],
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
    srcs = ["tests/playbook/mssql_playbook_ffi_validation.rs"],
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
    srcs = ["tests/playbook/postgres_playbook_ffi_validation.rs"],
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
    name = "ffi_test",
    srcs = ["tests/lifecycle/open_close_dependency_reset.rs"],
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
    name = "ffi_children_test",
    srcs = ["tests/lifecycle/open_close_with_children.rs"],
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
    srcs = ["tests/dependency/temporal/temporal_dependency_test.rs"],
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
    srcs = ["tests/dependency/smtp/smtp_dependency_test.rs"],
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
    srcs = ["tests/logging/logging_ffi_callbacks.rs"],
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
