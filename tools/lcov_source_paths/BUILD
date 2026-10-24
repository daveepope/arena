load("@rules_python//python:defs.bzl", "py_binary", "py_library", "py_test")

package(default_visibility = ["//visibility:public"])

py_library(
    name = "prefix_source_paths_lib",
    srcs = ["prefix_source_paths.py"],
    imports = ["."],
)

py_binary(
    name = "prefix_source_paths",
    srcs = ["prefix_source_paths.py"],
    main = "prefix_source_paths.py",
    deps = [":prefix_source_paths_lib"],
)

py_test(
    name = "prefix_source_paths_test",
    size = "small",
    srcs = ["prefix_source_paths_test.py"],
    main = "prefix_source_paths_test.py",
    deps = [":prefix_source_paths_lib"],
)
