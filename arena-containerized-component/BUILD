load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_containerized_component",
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
    name = "bind_mount_test",
    srcs = [
        "tests/bind_mount_test.rs",
        "tests/common/mod.rs",
    ],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_containerized_component",
        "//arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "volume_mount_test",
    srcs = [
        "tests/common/mod.rs",
        "tests/volume_mount_test.rs",
    ],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_containerized_component",
        "//arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "tmpfs_mount_test",
    srcs = [
        "tests/common/mod.rs",
        "tests/tmpfs_mount_test.rs",
    ],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_containerized_component",
        "//arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "mount_validation_test",
    srcs = ["tests/mount_validation_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_containerized_component",
        "//arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
