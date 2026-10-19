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

filegroup(
    name = "async_profiler_library",
    srcs = select({
        ":linux_x86_64": ["@async_profiler_linux_x64//:lib/libasyncProfiler.so"],
        ":linux_aarch64": ["@async_profiler_linux_arm64//:lib/libasyncProfiler.so"],
        "@platforms//os:macos": ["@async_profiler_macos//:lib/libasyncProfiler.dylib"],
        "//conditions:default": [],
    }),
    visibility = ["//visibility:public"],
)

rust_library(
    name = "arena_profile",
    srcs = glob(["src/**/*.rs"]),
    compile_data = ["src/backend/python_site_payload.py"],
    edition = "2021",
    deps = all_crate_deps(normal = True) + ["//arena"],
    visibility = ["//visibility:public"],
)

rust_test(
    name = "render_test",
    srcs = ["tests/unit/render_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "observer_test",
    srcs = ["tests/unit/observer_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "profiler_test",
    srcs = [
        "tests/support/recorded_profiles.rs",
        "tests/unit/profiler_test.rs",
    ],
    crate_root = "tests/unit/profiler_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "wrapped_test",
    srcs = [
        "tests/support/recorded_profiles.rs",
        "tests/unit/wrapped_test.rs",
    ],
    crate_root = "tests/unit/wrapped_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "augmented_test",
    srcs = [
        "tests/support/recorded_profiles.rs",
        "tests/unit/augmented_test.rs",
    ],
    crate_root = "tests/unit/augmented_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "backend_test",
    srcs = ["tests/unit/backend/mod_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "pyspy_unit_test",
    srcs = ["tests/unit/backend/pyspy_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "perf_unit_test",
    srcs = ["tests/unit/backend/perf_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "dotnet_trace_unit_test",
    srcs = ["tests/unit/backend/dotnet_trace_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "perf_component_test",
    srcs = [
        "tests/component/perf_component_test.rs",
        "tests/support/profile_subjects.rs",
        "tests/support/recorded_profiles.rs",
    ],
    crate_root = "tests/component/perf_component_test.rs",
    size = "small",
    edition = "2021",
    tags = [
        "component_test",
        "cpu_profile_test",
        "no-windows",
    ],
    data = ["//tools/profile_subjects:busy_loop_native"],
    target_compatible_with = select({
        "@platforms//os:linux": [],
        "//conditions:default": ["@platforms//:incompatible"],
    }),
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "async_profiler_component_test",
    srcs = [
        "tests/component/async_profiler_component_test.rs",
        "tests/support/profile_subjects.rs",
        "tests/support/recorded_profiles.rs",
    ],
    crate_root = "tests/component/async_profiler_component_test.rs",
    size = "small",
    edition = "2021",
    tags = [
        "component_test",
        "cpu_profile_test",
        "no-windows",
    ],
    data = [
        ":async_profiler_library",
        "//tools/profile_subjects:BusyLoop.java",
    ],
    target_compatible_with = select({
        "@platforms//os:linux": [],
        "@platforms//os:macos": [],
        "//conditions:default": ["@platforms//:incompatible"],
    }),
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "python_site_unit_test",
    srcs = ["tests/unit/backend/python_site_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "python_site_component_test",
    srcs = [
        "tests/component/python_site_component_test.rs",
        "tests/support/profile_subjects.rs",
        "tests/support/recorded_profiles.rs",
    ],
    crate_root = "tests/component/python_site_component_test.rs",
    size = "small",
    edition = "2021",
    tags = [
        "component_test",
        "cpu_profile_test",
        "no-windows",
    ],
    data = ["//tools/profile_subjects:busy_loop.py"],
    target_compatible_with = select({
        "@platforms//os:linux": [],
        "@platforms//os:macos": [],
        "//conditions:default": ["@platforms//:incompatible"],
    }),
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "pyspy_component_test",
    srcs = [
        "tests/component/pyspy_component_test.rs",
        "tests/support/profile_subjects.rs",
        "tests/support/recorded_profiles.rs",
    ],
    crate_root = "tests/component/pyspy_component_test.rs",
    size = "small",
    edition = "2021",
    tags = [
        "component_test",
        "cpu_profile_test",
        "no-windows",
    ],
    data = [
        "//tools/profile_subjects:busy_loop.py",
        "@profiling_tools//:py-spy__py-spy",
    ],
    target_compatible_with = select({
        "@platforms//os:linux": [],
        "//conditions:default": ["@platforms//:incompatible"],
    }),
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)

rust_test(
    name = "dotnet_trace_component_test",
    srcs = [
        "tests/component/dotnet_trace_component_test.rs",
        "tests/support/profile_subjects.rs",
        "tests/support/recorded_profiles.rs",
    ],
    crate_root = "tests/component/dotnet_trace_component_test.rs",
    size = "small",
    edition = "2021",
    tags = [
        "component_test",
        "cpu_profile_test",
    ],
    data = [
        "//tools/dotnet_trace:dotnet_trace_tool",
        "//tools/profile_subjects:busy_loop_dotnet",
    ],
    target_compatible_with = select({
        "@platforms//os:linux": [],
        "@platforms//os:macos": [],
        "//conditions:default": ["@platforms//:incompatible"],
    }),
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_profile",
        "//arena",
    ],
)
