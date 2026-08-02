load("@rules_dotnet//dotnet:defs.bzl", "dotnet_tool")
load("@rules_python//python:defs.bzl", "py_binary", "py_library", "py_test")

package(default_visibility = ["//visibility:public"])

dotnet_tool(
    name = "dotnet_coverage_tool",
    target_frameworks = ["net8.0"],
    entrypoint = {"net8.0": "dotnet-coverage.dll"},
    runner = {"net8.0": "dotnet"},
    deps = "@dotnet_coverage//:tools",
)

py_library(
    name = "cobertura_to_lcov_lib",
    srcs = ["cobertura_to_lcov.py"],
    imports = ["."],
)

py_binary(
    name = "cobertura_to_lcov",
    srcs = ["cobertura_to_lcov.py"],
    main = "cobertura_to_lcov.py",
    deps = [":cobertura_to_lcov_lib"],
)

# this is the rule not actually py_test not to be confused, this is purely for bridging dotnet coverage
py_test(
    name = "cobertura_to_lcov_test",
    size = "small",
    srcs = ["cobertura_to_lcov_test.py"],
    main = "cobertura_to_lcov_test.py",
    deps = [":cobertura_to_lcov_lib"],
)

exports_files(["coverage_wrapper.sh.tpl"])
