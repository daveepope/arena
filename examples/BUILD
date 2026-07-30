load("@rules_python//python:defs.bzl", "py_binary", "py_library", "py_test")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@rules_java//java:defs.bzl", "java_binary", "java_library", "java_test")
load("@rules_dotnet//dotnet:defs.bzl", "csharp_library", "csharp_binary", "csharp_test", "import_library")
load("@crates//:defs.bzl", "all_crate_deps")

import_library(
    name = "xunit_core_lib",
    library_name = "xunit.core",
    libs = ["@xunit_core//:libs"],
    refs = ["@xunit_core//:refs"],
)

import_library(
    name = "xunit_assert_lib",
    library_name = "xunit.assert",
    libs = ["@xunit_assert//:libs"],
    refs = ["@xunit_assert//:refs"],
)

import_library(
    name = "xunit_abstractions_lib",
    library_name = "xunit.abstractions",
    libs = ["@xunit_abstractions//:libs"],
    refs = ["@xunit_abstractions//:refs"],
)

filegroup(
    name = "xunit_execution_netstandard",
    srcs = ["@xunit_extensibility//:lib/netstandard1.1/xunit.execution.dotnet.dll"],
)

import_library(
    name = "xunit_extensibility_lib",
    library_name = "xunit.execution.dotnet",
    libs = [":xunit_execution_netstandard"],
    refs = [":xunit_execution_netstandard"],
)

filegroup(
    name = "xunit_runner_utility_netstandard",
    srcs = ["@xunit_runner_utility//:lib/netstandard1.5/xunit.runner.utility.netstandard15.dll"],
)

import_library(
    name = "xunit_runner_utility_lib",
    library_name = "xunit.runner.utility.netstandard15",
    libs = [":xunit_runner_utility_netstandard"],
    refs = [":xunit_runner_utility_netstandard"],
)

filegroup(
    name = "xunit_runner_reporters_netstandard",
    srcs = ["@xunit_runner_reporters//:lib/netstandard1.1/xunit.runner.reporters.netstandard11.dll"],
)

import_library(
    name = "xunit_runner_reporters_lib",
    library_name = "xunit.runner.reporters.netstandard11",
    libs = [":xunit_runner_reporters_netstandard"],
    refs = [":xunit_runner_reporters_netstandard"],
)

filegroup(
    name = "xunit_core_netstandard",
    srcs = ["@xunit_extensibility_core//:lib/netstandard1.1/xunit.core.dll"],
)

import_library(
    name = "xunit_extensibility_core_lib",
    library_name = "xunit.core",
    libs = [":xunit_core_netstandard"],
    refs = [":xunit_core_netstandard"],
)

import_library(
    name = "newtonsoft_json_lib",
    library_name = "Newtonsoft.Json",
    libs = ["@newtonsoft_json//:libs"],
    refs = ["@newtonsoft_json//:refs"],
)

import_library(
    name = "ms_logging_abstractions_lib",
    library_name = "Microsoft.Extensions.Logging.Abstractions",
    libs = ["@ms_logging_abstractions//:libs"],
    refs = ["@ms_logging_abstractions//:refs"],
)

import_library(
    name = "npgsql_lib",
    library_name = "Npgsql",
    libs = ["@npgsql//:libs"],
    refs = ["@npgsql//:refs"],
)

import_library(
    name = "sqlclient_lib",
    library_name = "Microsoft.Data.SqlClient",
    libs = ["@sqlclient//:libs"],
    refs = ["@sqlclient//:refs"],
)

import_library(
    name = "aws_core_lib",
    library_name = "AWSSDK.Core",
    libs = ["@aws_core//:libs"],
    refs = ["@aws_core//:refs"],
)

import_library(
    name = "aws_eventbridge_lib",
    library_name = "AWSSDK.EventBridge",
    libs = ["@aws_eventbridge//:libs"],
    refs = ["@aws_eventbridge//:refs"],
)

import_library(
    name = "aws_sqs_lib",
    library_name = "AWSSDK.SQS",
    libs = ["@aws_sqs//:libs"],
    refs = ["@aws_sqs//:refs"],
)

import_library(
    name = "jwt_lib",
    library_name = "System.IdentityModel.Tokens.Jwt",
    libs = ["@jwt//:libs"],
    refs = ["@jwt//:refs"],
)

import_library(
    name = "ms_identitymodel_tokens_lib",
    library_name = "Microsoft.IdentityModel.Tokens",
    libs = ["@ms_identitymodel_tokens//:libs"],
    refs = ["@ms_identitymodel_tokens//:refs"],
)

import_library(
    name = "ms_extensions_configuration_lib",
    library_name = "Microsoft.Extensions.Configuration",
    libs = ["@ms_extensions_configuration//:libs"],
    refs = ["@ms_extensions_configuration//:refs"],
)

import_library(
    name = "ms_extensions_configuration_abstractions_lib",
    library_name = "Microsoft.Extensions.Configuration.Abstractions",
    libs = ["@ms_extensions_configuration_abstractions//:libs"],
    refs = ["@ms_extensions_configuration_abstractions//:refs"],
)

import_library(
    name = "ms_extensions_configuration_binder_lib",
    library_name = "Microsoft.Extensions.Configuration.Binder",
    libs = ["@ms_extensions_configuration_binder//:libs"],
    refs = ["@ms_extensions_configuration_binder//:refs"],
)

import_library(
    name = "ms_extensions_dependency_injection_lib",
    library_name = "Microsoft.Extensions.DependencyInjection",
    libs = ["@ms_extensions_dependency_injection//:libs"],
    refs = ["@ms_extensions_dependency_injection//:refs"],
)

import_library(
    name = "ms_extensions_dependency_injection_abstractions_lib",
    library_name = "Microsoft.Extensions.DependencyInjection.Abstractions",
    libs = ["@ms_extensions_dependency_injection_abstractions//:libs"],
    refs = ["@ms_extensions_dependency_injection_abstractions//:refs"],
)

import_library(
    name = "ms_extensions_hosting_lib",
    library_name = "Microsoft.Extensions.Hosting",
    libs = ["@ms_extensions_hosting//:libs"],
    refs = ["@ms_extensions_hosting//:refs"],
)

import_library(
    name = "ms_extensions_hosting_abstractions_lib",
    library_name = "Microsoft.Extensions.Hosting.Abstractions",
    libs = ["@ms_extensions_hosting_abstractions//:libs"],
    refs = ["@ms_extensions_hosting_abstractions//:refs"],
)

import_library(
    name = "ms_extensions_http_lib",
    library_name = "Microsoft.Extensions.Http",
    libs = ["@ms_extensions_http//:libs"],
    refs = ["@ms_extensions_http//:refs"],
)

import_library(
    name = "temporalio_lib",
    library_name = "Temporalio",
    libs = ["@temporalio//:libs"],
    refs = ["@temporalio//:refs"],
)

import_library(
    name = "ms_aspnetcore_app_ref_lib",
    library_name = "Microsoft.AspNetCore.App.Ref",
    libs = ["@ms_aspnetcore_app_ref//:libs"],
    refs = ["@ms_aspnetcore_app_ref//:refs"],
)

import_library(
    name = "ms_netcore_app_ref_lib",
    library_name = "Microsoft.NETCore.App.Ref",
    libs = ["@ms_netcore_app_ref//:libs"],
    refs = ["@ms_netcore_app_ref//:refs"],
)

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
    name = "arena_config",
    srcs = ["resources/arena_config.json"],
    visibility = ["//visibility:public"],
)

py_library(
    name = "ephemeral_test_runtime",
    srcs = ["tests/pytest-shared/ephemeral_test_runtime.py"],
    imports = ["tests/pytest-shared"],
    visibility = ["//visibility:public"],
)

py_test(
    name = "ephemeral_test_runtime_test",
    size = "small",
    srcs = ["tests/pytest-shared/ephemeral_test_runtime_test.py"],
    main = "tests/pytest-shared/ephemeral_test_runtime_test.py",
    imports = ["tests/pytest-shared"],
    deps = [
        ":ephemeral_test_runtime",
        "@pip//pytest",
    ],
    python_version = "PY3",
)

java_library(
    name = "junit_shared",
    srcs = glob(["tests/junit-shared/**/*.java"]),
    visibility = ["//visibility:public"],
    deps = [
        "//arena-junit:arena_junit_lib",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
    ],
)

java_library(
    name = "ephemeral_test_runtime_java",
    srcs = [
        "tests/junit-shared/arena/examples/testruntime/EphemeralTestRuntime.java",
    ],
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
    name = "arena_config_py",
    srcs = ["tests/pytest-shared/arena_config.py"],
    imports = ["tests/pytest-shared"],
    data = [":arena_config"],
    deps = [":ephemeral_test_runtime"],
    visibility = ["//visibility:public"],
)

py_library(
    name = "pytest_shared",
    srcs = [
        "tests/pytest-shared/api_http.py",
        "tests/pytest-shared/playbook_fixtures.py",
        "tests/pytest-shared/playbooks.py",
    ],
    imports = ["tests/pytest-shared"],
    deps = [
        ":arena_config_py",
        "//arena-pytest:arena_pytest_lib",
        "@pip//requests",
    ],
    visibility = ["//visibility:public"],
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
        "@pip_examples//temporalio",
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
    name = "pytest_axum_component_test",
    size = "medium",
    srcs = glob(["tests/example-axum-pytest-component-test/**/*.py"]),
    main = "tests/example-axum-pytest-component-test/axum_component_test.py",
    imports = [
        "tests/example-axum-pytest-component-test",
        "tests/pytest-shared",
    ],
    data = [
        "//arena-ffi:arena_ffi_shared",
        "//examples:example-readings-axum-web-app",
        "//examples:instrument_reading_db_schema",
        "//examples:arena_config",
        "//examples:validation_db_schema",
    ],
    deps = [
        ":pytest_shared",
        "//arena-pytest:arena_pytest_lib",
        "@pip//kafka_python_ng",
        "@pip//pytest",
        "@pip//pytest_asyncio",
        "@pip//requests",
    ],
    tags = [
        "component_test",
        "example_component",
    ],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

py_test(
    name = "pytest_fastapi_component_test",
    size = "medium",
    srcs = glob(["tests/example-fastapi-pytest-component-test/**/*.py"]),
    main = "tests/example-fastapi-pytest-component-test/fastapi_component_test.py",
    imports = [
        "tests/example-fastapi-pytest-component-test",
        "tests/pytest-shared",
    ],
    data = [
        "//arena-ffi:arena_ffi_shared",
        "//examples:example-readings-fastapi-web-app",
        "//examples:instrument_reading_db_schema",
        "//examples:arena_config",
        "//examples:validation_db_schema",
    ],
    deps = [
        ":pytest_shared",
        "//arena-pytest:arena_pytest_lib",
        "@pip//pytest",
        "@pip//pytest_asyncio",
        "@pip//requests",
        "@pip_examples//boto3",
    ],
    tags = [
        "component_test",
        "example_component",
    ],
    python_version = "PY3",
    visibility = ["//visibility:public"],
)

rust_test(
    name = "example-axum-component-test",
    srcs = glob(["tests/example-axum-component-test/**/*.rs"]),
    crate_root = "tests/example-axum-component-test/axum_component_test.rs",
    size = "medium",
    edition = "2021",
    tags = [
        "component_test",
        "example_component",
    ],
    compile_data = [
        "resources/instrument_reading_db_schema.sql",
        "resources/validation_db_schema.sql",
    ],
    data = [
        ":example-readings-axum-web-app",
        ":arena_config",
    ],
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
        "25",
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
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_mail",
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
        "@arena_java_maven//:org_springframework_spring_context_support",
        "@arena_java_maven//:org_springframework_spring_jdbc",
        "@arena_java_maven//:org_springframework_spring_web",
        "@arena_java_maven//:software_amazon_awssdk_auth",
        "@arena_java_maven//:software_amazon_awssdk_eventbridge",
        "@arena_java_maven//:software_amazon_awssdk_regions",
        "@arena_java_maven//:software_amazon_awssdk_url_connection_client",
        "@arena_java_maven//:io_temporal_temporal_sdk",
        "@arena_java_maven//:io_temporal_temporal_serviceclient",
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
)

java_binary(
    name = "example-readings-spring-boot-web-app",
    javacopts = [
        "--release",
        "25",
    ],
    main_class = "arena.examples.readings.springboot.ReadingsApplication",
    visibility = ["//visibility:public"],
    runtime_deps = [":example_readings_spring_boot_lib"],
)

java_library(
    name = "example_spring_boot_component_test_logging_resources",
    resources = glob(["tests/example-spring-boot-component-test/resources/**"]),
    resource_strip_prefix = "examples/tests/example-spring-boot-component-test/resources",
    visibility = ["//visibility:private"],
)

java_test(
    name = "example-spring-boot-component-test",
    size = "medium",
    srcs = glob(["tests/example-spring-boot-component-test/**/*.java"]),
    javacopts = [
        "--release",
        "25",
    ],
    tags = [
        "component_test",
        "example_component",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-package=arena.examples.component",
    ],
    data = [
        ":example-readings-spring-boot-web-app",
        "//arena-ffi:arena_ffi_shared",
        ":instrument_reading_db_schema",
        ":arena_config",
        ":validation_db_schema",
    ],
    jvm_flags = ["-Djna.nosys=true"],
    runtime_deps = [
        ":example_spring_boot_component_test_logging_resources",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":ephemeral_test_runtime_java",
        ":junit_shared",
        "//arena-junit:arena_junit_lib",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_slf4j_slf4j_api",
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_logging",
        "@arena_java_maven//:software_amazon_awssdk_auth",
        "@arena_java_maven//:software_amazon_awssdk_regions",
        "@arena_java_maven//:software_amazon_awssdk_sqs",
        "@arena_java_maven//:software_amazon_awssdk_url_connection_client",
    ],
)

java_test(
    name = "example-axum-junit-component-test",
    size = "medium",
    srcs = glob(["tests/example-axum-junit-component-test/**/*.java"]),
    javacopts = [
        "--release",
        "25",
    ],
    tags = [
        "component_test",
        "example_component",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-package=arena.examples.component",
    ],
    data = [
        ":example-readings-axum-web-app",
        "//arena-ffi:arena_ffi_shared",
        ":instrument_reading_db_schema",
        ":arena_config",
        ":validation_db_schema",
    ],
    jvm_flags = ["-Djna.nosys=true"],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":ephemeral_test_runtime_java",
        ":junit_shared",
        "//arena-junit:arena_junit_lib",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:org_apache_kafka_kafka_clients",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
)

csharp_library(
    name = "xunit_shared",
    target_frameworks = ["net8.0"],
    srcs = glob(["tests/xunit-shared/**/*.cs"]),
    deps = [
        "//arena-xunit:arena_xunit_lib",
        ":xunit_core_lib",
        ":xunit_assert_lib",
        ":xunit_abstractions_lib",
        ":xunit_extensibility_lib",
        ":aws_sqs_lib",
    ],
    visibility = ["//visibility:public"],
)

csharp_library(
    name = "ephemeral_test_runtime_dotnet",
    target_frameworks = ["net8.0"],
    srcs = ["tests/xunit-shared/EphemeralTestRuntime.cs"],
    visibility = ["//visibility:public"],
)

csharp_library(
    name = "example_readings_aspnet_web_app_lib",
    target_frameworks = ["net8.0"],
    srcs = glob(
        ["src/example_readings_aspnet_web_app/**/*.cs"],
        exclude = ["src/example_readings_aspnet_web_app/Program.cs"],
    ),
    deps = [
        ":xunit_shared",
        "//arena-xunit:arena_xunit_lib",
        ":newtonsoft_json_lib",
        ":npgsql_lib",
        ":sqlclient_lib",
        ":aws_core_lib",
        ":aws_eventbridge_lib",
        ":aws_sqs_lib",
        ":ms_identitymodel_tokens_lib",
        ":jwt_lib",
        ":ms_logging_abstractions_lib",
        ":ms_aspnetcore_app_ref_lib",
        ":ms_netcore_app_ref_lib",
        ":ms_extensions_configuration_lib",
        ":ms_extensions_configuration_abstractions_lib",
        ":ms_extensions_configuration_binder_lib",
        ":ms_extensions_dependency_injection_lib",
        ":ms_extensions_dependency_injection_abstractions_lib",
        ":ms_extensions_hosting_lib",
        ":ms_extensions_hosting_abstractions_lib",
        ":ms_extensions_http_lib",
        ":temporalio_lib",
    ],
    visibility = ["//visibility:public"],
)

csharp_binary(
    name = "example-readings-aspnet-web-app",
    target_frameworks = ["net8.0"],
    project_sdk = "web",
    srcs = ["src/example_readings_aspnet_web_app/Program.cs"],
    appsetting_files = ["src/example_readings_aspnet_web_app/appsettings.json"],
    deps = [
        ":example_readings_aspnet_web_app_lib",
        ":temporalio_lib",
    ],
    visibility = ["//visibility:public"],
)

csharp_test(
    name = "example-aspnet-xunit-component-test",
    size = "medium",
    tags = [
        "component_test",
        "example_component",
    ],
    target_frameworks = ["net8.0"],
    srcs = glob(["tests/example-aspnet-xunit-component-test/**/*.cs"]),
    data = [
        ":example-readings-aspnet-web-app",
        "//arena-ffi:arena_ffi_shared",
        ":instrument_reading_db_schema",
        ":arena_config",
        ":validation_db_schema",
        "//arena-xunit:xunit_core_netstandard",
    ],
    deps = [
        ":xunit_shared",
        ":xunit_core_lib",
        ":xunit_assert_lib",
        ":xunit_abstractions_lib",
        ":xunit_extensibility_lib",
        ":xunit_extensibility_core_lib",
        ":xunit_runner_utility_lib",
        ":xunit_runner_reporters_lib",
        ":example_readings_aspnet_web_app_lib",
        "//arena-xunit:arena_xunit_lib",
        ":aws_sqs_lib",
        ":aws_core_lib",
    ],
    visibility = ["//visibility:public"],
)

test_suite(
    name = "example_component_tests",
    tags = ["component_test"],
    tests = [
        ":pytest_axum_component_test",
        ":pytest_fastapi_component_test",
        ":example-axum-component-test",
        ":example-axum-junit-component-test",
        ":example-spring-boot-component-test",
        ":example-aspnet-xunit-component-test",
    ],
    visibility = ["//visibility:public"],
)
