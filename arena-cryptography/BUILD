load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_cryptography",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True),
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "ephemeral_tls_test",
    srcs = ["tests/ephemeral_tls_test.rs"],
    size = "small",
    edition = "2021",
    deps = [":arena_cryptography"],
)
