load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

genrule(
    name = "default_images_rs",
    srcs = ["//:container_defaults.toml"],
    outs = ["src/default_images.rs"],
    cmd = "$(location //scripts:write_default_images_rs) $(location //:container_defaults.toml) $@",
    tools = ["//scripts:write_default_images_rs"],
)

rust_library(
    name = "arena_container",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = ["src/default_images.rs"],
    ) + [":default_images_rs"],
    edition = "2021",
    deps = all_crate_deps(normal = True),
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
