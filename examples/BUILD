load("@rules_python//python:defs.bzl", "py_binary", "py_library", "py_test")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@rules_java//java:defs.bzl", "java_binary", "java_library", "java_test")
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
    name = "readings_arena_config",
    srcs = ["resources/readings_arena_config.json"],
    visibility = ["//visibility:public"],
)

exports_files(
    [
        "requirements.txt",
        "requirements_lock.txt",
    ],
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
    tags = [
        "requires_docker",
        "exclusive",
    ],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

rust_test(
    name = "example-readings-axum-component-test",
    srcs = glob(["tests/example-readings-axum-component-test/**/*.rs"]),
    crate_root = "tests/example-readings-axum-component-test/readings_axum_component_test.rs",
    size = "small",
    edition = "2021",
    tags = [
        "requires_docker",
        "exclusive",
    ],
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

java_library(
    name = "example_readings_spring_boot_lib",
    srcs = glob(["src/example_readings_spring_boot_web_app/src/main/java/**/*.java"]),
    resources = glob(["src/example_readings_spring_boot_web_app/src/main/resources/**"]),
    javacopts = [
        "--release",
        "21",
    ],
    visibility = ["//visibility:public"],
    deps = [
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_annotations",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_core",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:com_microsoft_sqlserver_mssql_jdbc",
        "@arena_java_maven//:jakarta_validation_jakarta_validation_api",
        "@arena_java_maven//:org_postgresql_postgresql",
        "@arena_java_maven//:org_springframework_boot_spring_boot",
        "@arena_java_maven//:org_springframework_boot_spring_boot_autoconfigure",
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_jdbc",
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_oauth2_resource_server",
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_validation",
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_web",
        "@arena_java_maven//:org_springframework_security_spring_security_config",
        "@arena_java_maven//:org_springframework_security_spring_security_core",
        "@arena_java_maven//:org_springframework_security_spring_security_oauth2_core",
        "@arena_java_maven//:org_springframework_security_spring_security_oauth2_jose",
        "@arena_java_maven//:org_springframework_security_spring_security_oauth2_resource_server",
        "@arena_java_maven//:org_springframework_security_spring_security_web",
        "@arena_java_maven//:org_springframework_spring_beans",
        "@arena_java_maven//:org_springframework_spring_context",
        "@arena_java_maven//:org_springframework_spring_jdbc",
        "@arena_java_maven//:org_springframework_spring_web",
        "@arena_java_maven//:software_amazon_awssdk_auth",
        "@arena_java_maven//:software_amazon_awssdk_eventbridge",
        "@arena_java_maven//:software_amazon_awssdk_regions",
        "@arena_java_maven//:software_amazon_awssdk_url_connection_client",
    ],
)

java_binary(
    name = "example-readings-spring-boot-web-app",
    javacopts = [
        "--release",
        "21",
    ],
    main_class = "arena.examples.readings.springboot.ReadingsApplication",
    visibility = ["//visibility:public"],
    runtime_deps = [":example_readings_spring_boot_lib"],
)

java_library(
    name = "example_readings_spring_boot_component_test_logging_resources",
    resources = glob(["tests/example-readings-spring-boot-component-test/resources/**"]),
    resource_strip_prefix = "examples/tests/example-readings-spring-boot-component-test/resources",
    visibility = ["//visibility:private"],
)

java_test(
    name = "example-readings-spring-boot-component-test",
    size = "small",
    srcs = glob(["tests/example-readings-spring-boot-component-test/**/*.java"]),
    javacopts = [
        "--release",
        "21",
    ],
    tags = [
        "requires_docker",
        "exclusive",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-package=arena.examples.readings.component",
    ],
    data = [
        ":example-readings-spring-boot-web-app",
        "//arena-ffi:arena_ffi_shared",
        ":instrument_reading_db_schema",
        ":validation_db_schema",
    ],
    jvm_flags = ["-Djna.nosys=true"],
    runtime_deps = [
        ":example_readings_spring_boot_component_test_logging_resources",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        "//arena-junit:arena_junit_lib",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:org_apache_kafka_kafka_clients",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_slf4j_slf4j_api",
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_logging",
        "@arena_java_maven//:software_amazon_awssdk_auth",
        "@arena_java_maven//:software_amazon_awssdk_regions",
        "@arena_java_maven//:software_amazon_awssdk_sqs",
        "@arena_java_maven//:software_amazon_awssdk_url_connection_client",
    ],
)
