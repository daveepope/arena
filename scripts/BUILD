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
    name = "sync_version",
    srcs = ["sync_version.py"],
    main = "sync_version.py",
    deps = [":arena_version_lib"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

py_binary(
    name = "bump_patch_version",
    srcs = ["bump_patch_version.py"],
    main = "bump_patch_version.py",
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
    name = "bump_patch_version_test",
    size = "small",
    srcs = [
        "bump_patch_version.py",
        "bump_patch_version_test.py",
    ],
    main = "bump_patch_version_test.py",
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
