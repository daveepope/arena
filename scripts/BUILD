load("@rules_python//python:defs.bzl", "py_binary", "py_library", "py_test")

config_setting(
    name = "trivy_linux_x86_64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ],
)

config_setting(
    name = "trivy_macos_x86_64",
    constraint_values = [
        "@platforms//os:macos",
        "@platforms//cpu:x86_64",
    ],
)

config_setting(
    name = "trivy_macos_aarch64",
    constraint_values = [
        "@platforms//os:macos",
        "@platforms//cpu:aarch64",
    ],
)

alias(
    name = "trivy_bin",
    actual = select({
        ":trivy_linux_x86_64": "@trivy_linux_x86_64//:trivy",
        ":trivy_macos_x86_64": "@trivy_macos_x86_64//:trivy",
        ":trivy_macos_aarch64": "@trivy_macos_arm64//:trivy",
    }),
)

py_binary(
    name = "sync_agent_rules",
    srcs = ["sync_agent_rules.py"],
    main = "sync_agent_rules.py",
    visibility = ["//visibility:public"],
)

py_library(
    name = "arena_version_lib",
    srcs = ["arena_version.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "sync_version",
    srcs = ["sync_version.py"],
    main = "sync_version.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "bump_version",
    srcs = ["bump_version.py"],
    main = "bump_version.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "check_version_bump",
    srcs = ["check_version_bump.py"],
    main = "check_version_bump.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "check_release_lockfiles",
    srcs = ["check_release_lockfiles.py"],
    main = "check_release_lockfiles.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_test(
    name = "check_version_bump_test",
    size = "small",
    srcs = [
        "check_release_lockfiles.py",
        "check_version_bump.py",
        "check_version_bump_test.py",
    ],
    main = "check_version_bump_test.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_test(
    name = "bump_version_test",
    size = "small",
    srcs = [
        "bump_version.py",
        "bump_version_test.py",
    ],
    main = "bump_version_test.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_test(
    name = "version_sync_test",
    size = "small",
    srcs = ["version_sync_test.py"],
    main = "version_sync_test.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_test(
    name = "arena_version_test",
    size = "small",
    srcs = ["arena_version_test.py"],
    main = "arena_version_test.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "check_dependency_release_age",
    srcs = ["check_dependency_release_age.py"],
    main = "check_dependency_release_age.py",
    visibility = ["//visibility:public"],
)

py_binary(
    name = "publish_maven_central",
    srcs = ["publish_maven_central.py"],
    main = "publish_maven_central.py",
    visibility = ["//visibility:public"],
)

py_library(
    name = "container_defaults_lib",
    srcs = ["container_defaults.py"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "write_default_images_rs",
    srcs = ["write_default_images_rs.py"],
    main = "write_default_images_rs.py",
    deps = [":container_defaults_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "container_image_matrix",
    srcs = ["container_image_matrix.py"],
    main = "container_image_matrix.py",
    deps = [":container_defaults_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_library(
    name = "check_container_cves_lib",
    srcs = ["check_container_cves.py"],
    deps = [
        ":container_defaults_lib",
        "@bazel_tools//tools/python/runfiles",
    ],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "check_container_cves",
    srcs = ["check_container_cves.py"],
    data = [":trivy_bin"],
    main = "check_container_cves.py",
    deps = [":check_container_cves_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_test(
    name = "check_container_cves_parse_test",
    size = "small",
    srcs = ["check_container_cves_test.py"],
    main = "check_container_cves_test.py",
    deps = [":check_container_cves_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_test(
    name = "check_container_defaults_test",
    size = "small",
    srcs = [
        "check_container_defaults_test.py",
        "container_defaults.py",
    ],
    main = "check_container_defaults_test.py",
    deps = [":container_defaults_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_test(
    name = "dependency_release_age_parse_test",
    size = "small",
    srcs = [
        "check_dependency_release_age.py",
        "check_dependency_release_age_test.py",
    ],
    main = "check_dependency_release_age_test.py",
    python_version = "PY3",
    visibility = ["//visibility:public"],
)
