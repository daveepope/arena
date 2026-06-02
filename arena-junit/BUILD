load("@bazel_skylib//rules:copy_file.bzl", "copy_file")
load("@rules_java//java:defs.bzl", "java_library", "java_test")

copy_file(
    name = "arena_version_resource",
    src = "//:VERSION",
    out = "src/main/resources/arena-version.txt",
)

java_library(
    name = "arena_junit_lib",
    srcs = glob(["src/main/java/**/*.java"]),
    resources = [":arena_version_resource"],
    javacopts = [
        "--release",
        "21",
    ],
    visibility = ["//visibility:public"],
    deps = [
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
)

java_test(
    name = "arena_junit_readings_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/readings/test/ReadingsHttpPlaybookRegistrationTest.java"],
    javacopts = [
        "--release",
        "21",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.readings.test.ReadingsHttpPlaybookRegistrationTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
    ],
)

java_test(
    name = "arena_junit_readings_test",
    size = "small",
    srcs = glob(
        ["src/test/java/arena/junit/readings/**/*.java"],
        exclude = ["src/test/java/arena/junit/readings/test/ReadingsHttpPlaybookRegistrationTest.java"],
    ),
    javacopts = [
        "--release",
        "21",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-package=arena.junit.readings.test",
    ],
    tags = ["requires_docker"],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "//examples:readings_ephemeral_test_runtime_java",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_apache_kafka_kafka_clients",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
    data = [
        "//arena-ffi:arena_ffi_shared",
        "//examples:readings_arena_config",
        "//examples:example-readings-axum-web-app",
        "//examples:instrument_reading_db_schema",
        "//examples:validation_db_schema",
    ],
    jvm_flags = ["-Djna.nosys=true"],
)

java_test(
    name = "arena_junit_playbook_test",
    size = "small",
    srcs = [
        "src/test/java/arena/junit/ActiveHttpPlaybookVerifyTest.java",
        "src/test/java/arena/junit/ClosedArenaExtensionLifecycleTest.java",
        "src/test/java/arena/junit/PlaybookInvocationExtensionTest.java",
    ],
    javacopts = [
        "--release",
        "21",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ClosedArenaExtensionLifecycleTest",
        "--select-class=arena.junit.PlaybookInvocationExtensionTest",
        "--select-class=arena.junit.ActiveHttpPlaybookVerifyTest",
    ],
    tags = ["requires_docker"],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "//examples:readings_ephemeral_test_runtime_java",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
    ],
    data = [
        "//arena-ffi:arena_ffi_shared",
    ],
    jvm_flags = ["-Djna.nosys=true"],
)

java_test(
    name = "arena_junit_dispatcher_host_logging_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ffi/DispatcherHostLoggingLifecycleTest.java"],
    javacopts = [
        "--release",
        "21",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ffi.DispatcherHostLoggingLifecycleTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_params",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_logging",
    ],
    deps = [
        ":arena_junit_lib",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:ch_qos_logback_logback_classic",
        "@arena_java_maven//:ch_qos_logback_logback_core",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_params",
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
    data = ["//arena-ffi:arena_ffi_shared"],
    jvm_flags = ["-Djna.nosys=true"],
)
