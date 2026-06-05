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
    name = "ffi_mapping_unit_test",
    crate = ":arena_ffi",
    size = "small",
    edition = "2021",
    deps = _ARENA_FFI_DEPS,
    proc_macro_deps = _ARENA_FFI_PROC_MACRO_DEPS,
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
