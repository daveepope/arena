load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_executable_component",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        "//arena",
        "//arena-container:arena_container",
        "//arena-profiler:arena_profiler",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "platform_test",
    srcs = ["tests/unit/platform_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_executable_component",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "builder_test",
    srcs = ["tests/unit/builder_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_executable_component",
        "//arena",
        "//arena-profiler:arena_profiler",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "executable_component_test",
    srcs = ["tests/unit/executable_component_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_executable_component",
        "//arena",
        "//arena-profiler:arena_profiler",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "cpu_profile_component_test",
    srcs = ["tests/component/cpu_profile_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    data = [
        "@bazel_tools//tools/jdk:current_java_runtime",
        "@rules_python//python:current_py_toolchain",
    ],
    toolchains = [
        "@bazel_tools//tools/jdk:current_java_runtime",
        "@rules_python//python:current_py_toolchain",
    ],
    env = {
        "ARENA_JAVA_RUNTIME": "$(JAVABASE)",
        "ARENA_PYTHON_RUNTIME": "$(PYTHON3)",
    },
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_executable_component",
        "//arena",
        "//arena-profiler:arena_profiler",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
