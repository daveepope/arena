load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_dashboard",
    srcs = glob(["src/**/*.rs"], exclude = ["src/bin/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + ["//arena"],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "server",
    srcs = ["src/bin/server.rs"],
    data = ["//arena-dashboard/web:dist"],
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        ":arena_dashboard",
        "//arena",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "demo_lifecycle",
    srcs = ["src/bin/demo_lifecycle.rs"],
    edition = "2021",
    tags = ["manual"],
    deps = all_crate_deps(normal = True) + [
        ":arena_dashboard",
        "//arena",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "ingest_test",
    srcs = ["tests/unit/ingest_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_dashboard"],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "launcher_test",
    srcs = [
        "tests/support/temp_search_dir.rs",
        "tests/unit/launcher_test.rs",
    ],
    crate_root = "tests/unit/launcher_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_dashboard"],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "server_test",
    srcs = ["tests/unit/server_test.rs"],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_dashboard"],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "component_test",
    srcs = ["tests/component/component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_dashboard"],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "observer_component_test",
    srcs = ["tests/component/observer_component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_dashboard", "//arena"],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "launcher_component_test",
    srcs = [
        "tests/component/launcher_component_test.rs",
        "tests/support/temp_search_dir.rs",
    ],
    crate_root = "tests/component/launcher_component_test.rs",
    size = "small",
    data = [":server"],
    edition = "2021",
    tags = ["component_test"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [":arena_dashboard"],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
