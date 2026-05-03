load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_oauth",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        "//arena",
        "//arena-container:arena_container",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "oauth_https_flow",
    srcs = ["tests/oauth_https_flow.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_oauth",
        "//arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
