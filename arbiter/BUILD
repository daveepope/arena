load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arbiter",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = ["src/main.rs"],
    ),
    edition = "2021",
    deps = all_crate_deps(normal = True),
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "arbiter_cli",
    srcs = ["src/main.rs"],
    edition = "2021",
    deps = all_crate_deps(normal = True) + [":arbiter"],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

_FACTORY = "tests/harness_factory/mod.rs"

[
    rust_test(
        name = name,
        srcs = [
            _FACTORY,
            "tests/{}.rs".format(name),
        ],
        size = "small",
        edition = "2021",
        deps = all_crate_deps(
            normal = True,
            normal_dev = True,
        ) + [":arbiter"],
        proc_macro_deps = all_crate_deps(
            proc_macro = True,
            proc_macro_dev = True,
        ),
    )
    for name in [
        "transcript_cursor_test",
        "lifecycle_test",
    ]
]

[
    rust_test(
        name = name,
        srcs = ["tests/{}.rs".format(name)],
        size = "small",
        edition = "2021",
        deps = all_crate_deps(
            normal = True,
            normal_dev = True,
        ) + [":arbiter"],
        proc_macro_deps = all_crate_deps(
            proc_macro = True,
            proc_macro_dev = True,
        ),
    )
    for name in [
        "probes_test",
        "features_test",
        "scoring_test",
        "harness_test",
        "sessions_test",
        "complexity_test",
        "trajectory_test",
    ]
]
