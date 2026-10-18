load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True),
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "arena_test",
    srcs = [
        "tests/unit/arena_test.rs",
        "tests/unit/subjects.rs",
    ],
    crate_root = "tests/unit/arena_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "matches_test",
    srcs = [
        "tests/unit/matches_test.rs",
        "tests/unit/subjects.rs",
    ],
    crate_root = "tests/unit/matches_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "playbook_test",
    srcs = [
        "tests/unit/playbook_test.rs",
        "tests/unit/subjects.rs",
    ],
    crate_root = "tests/unit/playbook_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "playbook_lifecycle_test",
    srcs = [
        "tests/unit/playbook/lifecycle_test.rs",
        "tests/unit/subjects.rs",
    ],
    crate_root = "tests/unit/playbook/lifecycle_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "lifecycle_bus_test",
    srcs = [
        "tests/unit/lifecycle/bus_test.rs",
    ],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "lifecycle_rule_test",
    srcs = [
        "tests/unit/lifecycle/rule_test.rs",
    ],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "lifecycle_state_test",
    srcs = [
        "tests/unit/lifecycle/state_test.rs",
    ],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "lifecycle_message_test",
    srcs = [
        "tests/unit/lifecycle/message_test.rs",
    ],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "lifecycle_fault_test",
    srcs = [
        "tests/unit/lifecycle/fault_test.rs",
    ],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "lifecycle_snapshot_test",
    srcs = [
        "tests/unit/lifecycle/snapshot_test.rs",
    ],
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)

rust_test(
    name = "lifecycle_publisher_test",
    srcs = [
        "tests/unit/lifecycle/publisher_test.rs",
        "tests/unit/subjects.rs",
    ],
    crate_root = "tests/unit/lifecycle/publisher_test.rs",
    size = "small",
    edition = "2021",
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
