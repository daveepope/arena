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
    name = "arena_profiler",
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
    name = "backend_test",
    srcs = ["tests/unit/backend/mod_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_profiler"],
)

rust_test(
    name = "perf_test",
    srcs = ["tests/unit/backend/perf_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_profiler"],
)

rust_test(
    name = "sampler_test",
    srcs = ["tests/unit/sampler_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_profiler"],
)

rust_test(
    name = "profiler_test",
    srcs = ["tests/unit/profiler_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_profiler"],
)

rust_test(
    name = "render_test",
    srcs = ["tests/unit/render_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_profiler"],
)

rust_test(
    name = "perf_component_test",
    srcs = ["tests/component/perf_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_profiler"],
)

rust_test(
    name = "pyspy_component_test",
    srcs = ["tests/component/pyspy_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_profiler"],
)

rust_test(
    name = "async_profiler_component_test",
    srcs = ["tests/component/async_profiler_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    data = ["@bazel_tools//tools/jdk:current_java_runtime"],
    toolchains = ["@bazel_tools//tools/jdk:current_java_runtime"],
    env = {"ARENA_JAVA_RUNTIME": "$(JAVABASE)"},
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_profiler"],
)
