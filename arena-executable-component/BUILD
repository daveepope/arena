load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_executable_component",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        "//arena",
        "//arena-container:arena_container",
        "//arena-profile:arena_profile",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "lifecycle_test",
    srcs = ["tests/component/lifecycle_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        "//arena",
        ":arena_executable_component",
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
    target_compatible_with = select({
        "@platforms//os:linux": [],
        "//conditions:default": ["@platforms//:incompatible"],
    }),
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        "//arena",
        "//arena-profile:arena_profile",
        ":arena_executable_component",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "platform_test",
    srcs = ["tests/platform_test.rs"],
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
    name = "builder_unit_test",
    srcs = ["tests/unit/builder_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_executable_component",
        "//arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "executable_component_unit_test",
    srcs = ["tests/unit/executable_component_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_executable_component",
        "//arena",
        "//arena-profile:arena_profile",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
