load("@rules_rust//rust:defs.bzl", "rust_library", "rust_shared_library")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_ffi",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        "//arena",
        "//arena-container-component:arena_container_component",
        "//arena-executable-component:arena_executable_component",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_shared_library(
    name = "arena_ffi_shared",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        "//arena",
        "//arena-container-component:arena_container_component",
        "//arena-executable-component:arena_executable_component",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

