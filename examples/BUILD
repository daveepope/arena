load("@rules_python//python:defs.bzl", "py_test")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@crates//:defs.bzl", "all_crate_deps")

filegroup(
    name = "instrument_reading_db_schema",
    srcs = ["resources/instrument_reading_db_schema.sql"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "validation_db_schema",
    srcs = ["resources/validation_db_schema.sql"],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "oauth_tls_resources",
    srcs = [
        "resources/oauth_tls_cert.pem",
        "resources/oauth_tls_key.pem",
    ],
    visibility = ["//visibility:public"],
)

rust_library(
    name = "arena_examples",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        "//arena",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
        "//arena-mssql:arena_mssql",
        "//arena-http:arena_http",
        "//arena-oauth:arena_oauth",
        "//arena-executable-component:arena_executable_component",
        "//arena-containerized-component:arena_containerized_component",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "web-app",
    srcs = glob(["src/example-web-app/**/*.rs"]),
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        ":arena_examples",
        "//arena",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
        "//arena-mssql:arena_mssql",
        "//arena-oauth:arena_oauth",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "example-console",
    srcs = ["src/example-console/main.rs"],
    compile_data = [
        "src/example-web-app/Dockerfile",
        "resources/instrument_reading_db_schema.sql",
        "resources/validation_db_schema.sql",
        "resources/oauth_tls_cert.pem",
        "resources/oauth_tls_key.pem",
    ],
    edition = "2021",
    deps = all_crate_deps(normal = True) + [
        ":arena_examples",
        "//arena",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
        "//arena-mssql:arena_mssql",
        "//arena-oauth:arena_oauth",
        "//arena-executable-component:arena_executable_component",
        "//arena-containerized-component:arena_containerized_component",
    ],
    proc_macro_deps = all_crate_deps(proc_macro = True),
)

py_test(
    name = "pytest_localstack_component_test",
    size = "small",
    srcs = [
        "tests/example-arena-pytest-localstack-test/test_localstack.py",
    ],
    main = "tests/example-arena-pytest-localstack-test/test_localstack.py",
    data = [
        "//arena-ffi:arena_ffi_shared",
    ],
    deps = [
        "//arena-pytest:arena_pytest_lib",
        "@pip//pytest",
        "@pip//pytest_asyncio",
    ],
    tags = ["local"],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

rust_test(
    name = "component_test",
    srcs = ["tests/example-arena-component-test/component_test.rs"],
    size = "small",
    edition = "2021",
    tags = ["local"],
    compile_data = [
        "resources/instrument_reading_db_schema.sql",
        "resources/validation_db_schema.sql",
        "resources/oauth_tls_cert.pem",
        "resources/oauth_tls_key.pem",
    ],
    data = [":web-app"],
    deps = all_crate_deps(
        normal = True,
        normal_dev = True,
    ) + [
        ":arena_examples",
        "//arena",
        "//arena-http:arena_http",
        "//arena-kafka:arena_kafka",
        "//arena-postgres:arena_postgres",
        "//arena-mssql:arena_mssql",
        "//arena-oauth:arena_oauth",
        "//arena-executable-component:arena_executable_component",
        "//arena-containerized-component:arena_containerized_component",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
        proc_macro_dev = True,
    ),
)
