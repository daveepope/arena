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
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = _ARENA_FFI_DEPS,
    proc_macro_deps = _ARENA_FFI_PROC_MACRO_DEPS,
    visibility = ["//visibility:public"],
)

rust_test(
    name = "arena_ffi_test",
    crate = ":arena_ffi",
    size = "small",
    deps = all_crate_deps(normal_dev = True),
    proc_macro_deps = all_crate_deps(proc_macro_dev = True),
)
