load("@rules_python//python:defs.bzl", "py_binary", "py_library", "py_test")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
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

exports_files(
    ["requirements.txt", "requirements_lock.txt"],
    visibility = ["//visibility:public"],
)

compile_pip_requirements(
    name = "pip_requirements",
    src = "requirements.txt",
    requirements_txt = "requirements_lock.txt",
    generate_hashes = False,
)

py_library(
    name = "example_readings_fastapi_web_app_lib",
    srcs = glob(
        ["src/example_readings_fastapi_web_app/**/*.py"],
        exclude = ["src/example_readings_fastapi_web_app/web_server/__main__.py"],
    ),
    imports = ["src"],
    deps = [
        "@pip_examples//asyncpg",
        "@pip_examples//boto3",
        "@pip_examples//fastapi",
        "@pip_examples//httpx",
        "@pip_examples//pydantic_settings",
        "@pip_examples//pyjwt",
        "@pip_examples//fastmssql",
    ],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "example-readings-fastapi-web-app",
    srcs = ["src/example_readings_fastapi_web_app/web_server/__main__.py"],
    imports = ["src"],
    main = "src/example_readings_fastapi_web_app/web_server/__main__.py",
    deps = [
        ":example_readings_fastapi_web_app_lib",
        "@pip_examples//uvicorn",
    ],
    visibility = ["//visibility:public"],
)

rust_library(
    name = "arena_examples",
    srcs = glob(
        ["src/**/*.rs"],
        exclude = [
            "src/example_readings_axum_web_app/web_server/main.rs",
            "src/example-console/main.rs",
        ],
    ),
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
    name = "example-readings-axum-web-app",
    srcs = ["src/example_readings_axum_web_app/web_server/main.rs"],
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
        "src/example_readings_axum_web_app/web_server/Dockerfile",
        "resources/instrument_reading_db_schema.sql",
        "resources/validation_db_schema.sql",
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
        "@pip_examples//boto3",
    ],
    tags = ["requires_docker"],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

py_test(
    name = "pytest_readings_fastapi_component_test",
    size = "small",
    srcs = [
        "tests/example-readings-fastapi-pytest-component-test/conftest.py",
        "tests/example-readings-fastapi-pytest-component-test/readings_fastapi_component_test.py",
    ],
    main = "tests/example-readings-fastapi-pytest-component-test/readings_fastapi_component_test.py",
    data = [
        "//arena-ffi:arena_ffi_shared",
        "//examples:example-readings-fastapi-web-app",
        "//examples:instrument_reading_db_schema",
        "//examples:validation_db_schema",
    ],
    deps = [
        "//arena-pytest:arena_pytest_lib",
        "@pip//pytest",
        "@pip//pytest_asyncio",
        "@pip//requests",
        "@pip_examples//boto3",
    ],
    tags = ["requires_docker"],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

rust_test(
    name = "example-readings-axum-component-test",
    srcs = glob(["tests/example-readings-axum-component-test/**/*.rs"]),
    crate_root = "tests/example-readings-axum-component-test/readings_axum_component_test.rs",
    size = "small",
    edition = "2021",
    tags = ["requires_docker"],
    compile_data = [
        "resources/instrument_reading_db_schema.sql",
        "resources/validation_db_schema.sql",
    ],
    data = [":example-readings-axum-web-app"],
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
