load("@rules_python//python:defs.bzl", "py_binary", "py_test")

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
