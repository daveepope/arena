load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "arena_host",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    visibility = ["//visibility:public"],
)

rust_test(
    name = "find_available_port_test",
    srcs = ["tests/unit/find_available_port_test.rs"],
    size = "small",
    edition = "2021",
    deps = [":arena_host"],
)
