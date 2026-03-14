load("@rules_rust//rust:defs.bzl", "rust_library")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_container",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True),
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)