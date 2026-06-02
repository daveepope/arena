load("@rules_python//python:defs.bzl", "py_binary", "py_test")

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
    srcs = ["version_sync_test.py"],
    main = "version_sync_test.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)
