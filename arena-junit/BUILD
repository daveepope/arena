load("@arena_version//:defs.bzl", "ARENA_VERSION")
load("@bazel_skylib//rules:copy_file.bzl", "copy_file")
load("@rules_java//java:defs.bzl", "java_library", "java_test")
load("@rules_jvm_external//:defs.bzl", "java_export")
load(":native_lib.bzl", "arena_junit_classifier_native_lib")

_ARENA_JUNIT_SRCS = glob(["src/main/java/**/*.java"])

copy_file(
    name = "arena_ffi_shared_for_maven_linux_x86_64",
    src = "//arena-ffi:arena_ffi_shared",
    out = "src/main/resources/linux-x86-64/libarena_ffi_shared.so",
)

_ARENA_JUNIT_CLASSIFIER_ARTIFACTS = {}

arena_junit_classifier_native_lib(
    name = "arena_junit_native_linux_x86_64",
    classifier = "linux-x86_64",
    resources = [":arena_ffi_shared_for_maven_linux_x86_64"],
    classifier_artifacts = _ARENA_JUNIT_CLASSIFIER_ARTIFACTS,
)

arena_junit_classifier_native_lib(
    name = "arena_junit_native_osx_aarch64",
    classifier = "osx-aarch_64",
    resources = glob(
        ["prebuilt/src/main/resources/darwin-aarch64/**"],
        allow_empty = True,
    ),
    classifier_artifacts = _ARENA_JUNIT_CLASSIFIER_ARTIFACTS,
)

arena_junit_classifier_native_lib(
    name = "arena_junit_native_osx_x86_64",
    classifier = "osx-x86_64",
    resources = glob(
        ["prebuilt/src/main/resources/darwin-x86-64/**"],
        allow_empty = True,
    ),
    classifier_artifacts = _ARENA_JUNIT_CLASSIFIER_ARTIFACTS,
)

arena_junit_classifier_native_lib(
    name = "arena_junit_native_windows_x86_64",
    classifier = "windows-x86_64",
    resources = glob(
        ["prebuilt/src/main/resources/windows-x86-64/**"],
        allow_empty = True,
    ),
    classifier_artifacts = _ARENA_JUNIT_CLASSIFIER_ARTIFACTS,
)

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
    classifier_artifacts = _ARENA_JUNIT_CLASSIFIER_ARTIFACTS,
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
    classifier_artifacts = _ARENA_JUNIT_CLASSIFIER_ARTIFACTS,
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
    name = "arena_junit_arena_extension_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ArenaExtensionUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ArenaExtensionUnitTest",
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
        "@arena_java_maven//:org_junit_platform_junit_platform_suite_api",
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
)

java_test(
    name = "arena_junit_closed_arena_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ClosedArenaUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ClosedArenaUnitTest",
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
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
)

java_test(
    name = "arena_junit_closed_arena_non_ascii_component_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ClosedArenaNonAsciiComponentTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ClosedArenaNonAsciiComponentTest",
    ],
    tags = ["component_test"],
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
    data = [
        "//arena-ffi:arena_ffi_shared",
    ],
    jvm_flags = ["-Djna.nosys=true"],
)

java_test(
    name = "arena_junit_readiness_checks_ffi_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/readiness/ReadinessChecksFfiSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.readiness.ReadinessChecksFfiSerializationTest",
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
    name = "arena_junit_match_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/match/MatchUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.match.MatchUnitTest",
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
    name = "arena_junit_match_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/match/MatchBuilderUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.match.MatchBuilderUnitTest",
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
    name = "arena_junit_registered_playbook_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/match/RegisteredPlaybookUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.match.RegisteredPlaybookUnitTest",
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
    name = "arena_junit_localstack_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/dep/LocalstackDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.dep.LocalstackDependencyBuilderSerializationTest",
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
    name = "arena_junit_mssql_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/dep/MssqlDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.dep.MssqlDependencyBuilderSerializationTest",
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
    name = "arena_junit_postgres_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/dep/PostgresDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.dep.PostgresDependencyBuilderSerializationTest",
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
    name = "arena_junit_oracle_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/dep/oracledb/OracleDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.dep.oracledb.OracleDependencyBuilderSerializationTest",
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
    name = "arena_junit_kafka_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/dep/KafkaDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.dep.KafkaDependencyBuilderSerializationTest",
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
    name = "arena_junit_http_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/dep/HttpDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.dep.HttpDependencyBuilderSerializationTest",
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
    name = "arena_junit_oauth_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/oauth/OauthDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.oauth.OauthDependencyBuilderSerializationTest",
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
    name = "arena_junit_oauth_issuer_hosts_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/oauth/OauthIssuerHostsUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.oauth.OauthIssuerHostsUnitTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
    ],
)

java_test(
    name = "arena_junit_containerized_component_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/exec/ContainerizedComponentBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.exec.ContainerizedComponentBuilderSerializationTest",
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
    name = "arena_junit_executable_component_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/exec/ExecutableComponentBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.exec.ExecutableComponentBuilderSerializationTest",
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
    name = "arena_junit_active_playbook_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/playbook/ActivePlaybookUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.playbook.ActivePlaybookUnitTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
    ],
)

java_test(
    name = "arena_junit_managed_postgres_playbook_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/playbook/ManagedPostgresPlaybookUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.playbook.ManagedPostgresPlaybookUnitTest",
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
    name = "arena_junit_managed_oracle_playbook_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/playbook/oracledb/ManagedOraclePlaybookUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.playbook.oracledb.ManagedOraclePlaybookUnitTest",
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
    name = "arena_junit_managed_mssql_playbook_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/playbook/ManagedMssqlPlaybookUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.playbook.ManagedMssqlPlaybookUnitTest",
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
    name = "arena_junit_managed_localstack_playbook_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/playbook/ManagedLocalstackPlaybookUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.playbook.ManagedLocalstackPlaybookUnitTest",
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
    name = "arena_junit_managed_http_playbook_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/playbook/ManagedHttpPlaybookUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.playbook.ManagedHttpPlaybookUnitTest",
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
    name = "arena_junit_open_arena_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/OpenArenaUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.OpenArenaUnitTest",
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
    name = "arena_junit_arena_status_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ffi/ArenaStatusUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ffi.ArenaStatusUnitTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_params",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_params",
    ],
)

java_test(
    name = "arena_junit_arena_binding_error_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ffi/ArenaBindingErrorUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ffi.ArenaBindingErrorUnitTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
    ],
)

java_test(
    name = "arena_junit_arena_identifiers_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/support/ArenaIdentifiersUnitTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.support.ArenaIdentifiersUnitTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
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
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_logging",
    ],
    deps = [
        ":arena_junit_lib",
        "//examples:ephemeral_test_runtime_java",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:ch_qos_logback_logback_classic",
        "@arena_java_maven//:ch_qos_logback_logback_core",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_junit_platform_junit_platform_suite_api",
        "@arena_java_maven//:org_slf4j_slf4j_api",
    ],
    data = [
        "//arena-ffi:arena_ffi_shared",
    ],
    jvm_flags = ["-Djna.nosys=true"],
)

java_test(
    name = "arena_junit_children_lifecycle_component_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ChildrenLifecycleComponentTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.ChildrenLifecycleComponentTest",
    ],
    tags = ["component_test"],
    target_compatible_with = select({
        "@platforms//os:windows": ["@platforms//:incompatible"],
        "//conditions:default": [],
    }),
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
        "@arena_java_maven//:org_springframework_boot_spring_boot_starter_logging",
    ],
    deps = [
        ":arena_junit_lib",
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
    name = "arena_junit_suite_sharing_component_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/ArenaSuiteSharingComponentTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-platform-suite",
        "--select-class=arena.junit.ArenaSuiteSharingComponentTest$$SharedSuite",
    ],
    tags = ["component_test"],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
        "@arena_java_maven//:org_junit_platform_junit_platform_suite_engine",
    ],
    deps = [
        ":arena_junit_lib",
        "//examples:ephemeral_test_runtime_java",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:net_java_dev_jna_jna",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_junit_platform_junit_platform_suite_api",
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
    name = "arena_junit_smtp_dependency_builder_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/dep/smtp/SmtpDependencyBuilderSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.dep.smtp.SmtpDependencyBuilderSerializationTest",
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
    name = "arena_junit_children_ffi_unit_test",
    size = "small",
    srcs = ["src/test/java/arena/junit/support/ChildrenFfiSerializationTest.java"],
    javacopts = [
        "--release",
        "25",
    ],
    use_testrunner = False,
    main_class = "org.junit.platform.console.ConsoleLauncher",
    args = [
        "--include-engine=junit-jupiter",
        "--select-class=arena.junit.support.ChildrenFfiSerializationTest",
    ],
    runtime_deps = [
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_engine",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_params",
        "@arena_java_maven//:org_junit_platform_junit_platform_console",
        "@arena_java_maven//:org_junit_platform_junit_platform_launcher",
    ],
    deps = [
        ":arena_junit_lib",
        "@arena_java_maven//:com_fasterxml_jackson_core_jackson_databind",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_api",
        "@arena_java_maven//:org_junit_jupiter_junit_jupiter_params",
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
