load("@rules_python//python:defs.bzl", "py_library", "py_test")
load(":nuget_version_test.bzl", "nuspec_file_test_suite")

package(default_visibility = ["//visibility:public"])

exports_files([
    "content_types.xml",
    "rels_template.xml",
    "core_properties_template.psmdcp",
    "nuget_push.sh.tpl",
    "pinned_dependency_versions.bzl",
])

nuspec_file_test_suite(name = "nuspec_file_test")

py_library(
    name = "check_pinned_versions_lib",
    srcs = ["check_pinned_versions.py"],
    imports = ["."],
)

py_test(
    name = "check_pinned_versions_test",
    size = "small",
    srcs = ["check_pinned_versions_test.py"],
    main = "check_pinned_versions_test.py",
    deps = [":check_pinned_versions_lib"],
)
