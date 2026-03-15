load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

rust_library(
    name = "arena_examples",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        "//arena",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
        "//arena-executable-component:arena_executable_component",
        "//arena-container-component:arena_container_component",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "arena-examples-bin",
    srcs = ["src/example-arena-console/main.rs"],
    compile_data = [
        "src/example-web-app/Dockerfile",
        "resources/instrument_reading_db_schema.sql",
    ],
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        ":arena_examples",
        "//arena",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
        "//arena-executable-component:arena_executable_component",
        "//arena-container-component:arena_container_component",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
)

rust_test(
    name = "component_test",
    srcs = ["tests/example-arena-component-test/component_test.rs"],
    size = "medium",
    edition = "2021",
    tags = ["local"],
    compile_data = [
        "resources/instrument_reading_db_schema.sql",
        "src/example-web-app/Dockerfile",
        "Cargo.toml",
    ],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_examples",
        "//arena",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
        "//arena-executable-component:arena_executable_component",
        "//arena-container-component:arena_container_component",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)