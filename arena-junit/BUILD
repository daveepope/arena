load("@arena_version//:defs.bzl", "ARENA_VERSION")
load("@rules_java//java:defs.bzl", "java_library", "java_test")
load("@rules_jvm_external//:defs.bzl", "java_export")

_ARENA_JUNIT_SRCS = glob(["src/main/java/**/*.java"])

_ARENA_JUNIT_JAVACOPTS = [
    "--release",
    "25",
]

_ARENA_JUNIT_DEPS = [
    "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
    "@arena_java_maven//:net_java_dev_jna_jna",
    "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
    "@arena_java_maven//:org_slf4j_slf4j_api",
]

_MAVEN_GROUP_ARTIFACT = "io.github.stationdevx:arena-junit"

java_library(
    name = "arena_junit_lib",
    srcs = _ARENA_JUNIT_SRCS,
    javacopts = _ARENA_JUNIT_JAVACOPTS,
    visibility = ["//visibility:public"],
    deps = _ARENA_JUNIT_DEPS,
)

java_export(
    name = "arena_junit_maven",
    srcs = _ARENA_JUNIT_SRCS,
    javacopts = _ARENA_JUNIT_JAVACOPTS,
    tags = ["manual"],
    maven_coordinates = _MAVEN_GROUP_ARTIFACT + ":" + ARENA_VERSION,
    pom_template = "pom_template.xml",
    visibility = ["//visibility:public"],
    deps = _ARENA_JUNIT_DEPS,
)

java_export(
    name = "arena_junit_maven_snapshot",
    srcs = _ARENA_JUNIT_SRCS,
    javacopts = _ARENA_JUNIT_JAVACOPTS,
    tags = ["manual"],
    maven_coordinates = _MAVEN_GROUP_ARTIFACT + ":" + ARENA_VERSION + "-SNAPSHOT",
    pom_template = "pom_template.xml",
    visibility = ["//visibility:public"],
    deps = _ARENA_JUNIT_DEPS,
)

java_test(
    name = "arena_junit_http_playbook_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/HttpPlaybookBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.HttpPlaybookBuilderSerializationTest",
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
    name = "arena_junit_http_playbook_registration_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/HttpPlaybookRegistrationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.HttpPlaybookRegistrationTest",
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
    name = "arena_junit_playbook_extension_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/playbook/PlaybookInvocationExtensionUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.playbook.PlaybookInvocationExtensionUnitTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
    ],
)

java_test(
    name = "arena_junit_playbook_extension_component_test",
    size = "medium",
    srcs = [
        "src/test/java/arena/junit/PlaybookInvocationExtensionComponentTest.java",
    ],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.PlaybookInvocationExtensionComponentTest",
    ],
    tags = ["component_test"],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "//examples:ephemeral_test_runtime_java",
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
    name = "arena_junit_arena_extension_lifecycle_component_test",
    size = "medium",
    srcs = ["src/test/java/arena/junit/ArenaExtensionLifecycleComponentTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ArenaExtensionLifecycleComponentTest",
    ],
    tags = ["component_test"],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "//examples:ephemeral_test_runtime_java",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
    data = [
        "//arena-ffi:arena_ffi_shared",
    ],
    jvm_flags = ["-Djna.nosys=true"],
)

java_test(
    name = "arena_junit_http_playbook_fluent_api_test",
    size = "medium",
    srcs = ["src/test/java/arena/junit/HttpPlaybookFluentApiTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.HttpPlaybookFluentApiTest",
    ],
    tags = ["component_test"],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "//examples:ephemeral_test_runtime_java",
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
    name = "arena_junit_temporal_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/dep/temporal/TemporalDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.dep.temporal.TemporalDependencyBuilderSerializationTest",
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
    name = "arena_junit_dispatcher_host_logging_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ffi/DispatcherHostLoggingLifecycleTest.java"],
    javacopts = [
        "--release",
        "25",
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
