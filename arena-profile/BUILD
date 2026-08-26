load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

config_setting(
    name = "linux_x86_64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ],
)

config_setting(
    name = "linux_aarch64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:aarch64",
    ],
)

rust_library(
    name = "arena_profile",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    compile_data = ["//:arena-logo.png"],
    data = select({
        ":linux_x86_64": ["@async_profiler_linux_x64//:lib/libasyncProfiler.so"],
        ":linux_aarch64": ["@async_profiler_linux_arm64//:lib/libasyncProfiler.so"],
        "//conditions:default": [],
    }) + [
        "@profiling_tools//:py-spy__py-spy",
    ],
    deps = all_crate_deps(normal = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "arena_profile_lib_unit_test",
    crate = ":arena_profile",
    size = "small",
    edition = "2021",
)

rust_test(
    name = "render_test",
    srcs = ["tests/render_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
    ],
)

rust_test(
    name = "profiler_test",
    srcs = ["tests/profiler_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
    ],
)

rust_test(
    name = "backend_test",
    srcs = ["tests/backend/mod_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
    ],
)

rust_test(
    name = "perf_component_test",
    srcs = ["tests/perf_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
    ],
)

rust_test(
    name = "async_profiler_component_test",
    srcs = ["tests/async_profiler_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
    ],
)

rust_test(
    name = "pyspy_component_test",
    srcs = ["tests/pyspy_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
    ],
)
