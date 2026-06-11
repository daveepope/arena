load("@rules_python//python:defs.bzl", "py_binary", "py_library", "py_test")

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
    name = "bump_release_version",
    srcs = ["bump_release_version.py"],
    main = "bump_release_version.py",
    deps = [":arena_version_lib"],
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
