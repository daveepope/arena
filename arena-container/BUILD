load("@rules_rust//cargo:defs.bzl", "cargo_build_script")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

cargo_build_script(
    name = "build_script",
    srcs = ["build.rs"],
    data = ["//:container_defaults.toml"],
    compile_data = ["//:container_defaults.toml"],
    build_script_env = {
        "ARENA_CONTAINER_DEFAULTS_TOML": "$(execpath //:container_defaults.toml)",
    },
    deps = all_crate_deps(build = True),
)

rust_library(
    name = "arena_container",
    srcs = glob(["src/**/*.rs"]),
    compile_data = ["//:container_defaults.toml"],
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        ":build_script",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "identifier_test",
    srcs = ["tests/identifier_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_container",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "default_images_test",
    srcs = ["tests/default_images_test.rs"],
    size = "small",
    edition = "2021",
    deps = [":arena_container"],
)

rust_test(
    name = "network_component_test",
    srcs = ["tests/network_component_test.rs"],
    size = "medium",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_container",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
