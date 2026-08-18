load("@rules_dotnet//dotnet:defs.bzl", "csharp_library")
load("@rules_python//python:defs.bzl", "py_library", "py_test")
load(":defs.bzl", "csharp_nuget_package")
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

csharp_library(
    name = "nuget_native_libs_test_fixture_lib",
    srcs = ["testdata/FixtureLib.cs"],
    out = "FixtureLib",
    target_frameworks = ["netstandard2.0"],
    compiler_options = ["/langversion:latest"],
)

csharp_nuget_package(
    name = "nuget_native_libs_test_fixture",
    package_id = "Fixture.Package",
    version = "0.0.0",
    nuspec_template = "testdata/fixture_nuspec_template.xml",
    dll = ":nuget_native_libs_test_fixture_lib",
    target_framework = "netstandard2.0",
    native_libs_by_rid = {
        "osx-arm64": ["testdata/fake_native_osx_arm64.txt"],
        "linux-x64": ["testdata/fake_native_linux_x64.txt"],
    },
)

py_test(
    name = "nuget_native_libs_layout_test",
    size = "small",
    srcs = ["nuget_native_libs_layout_test.py"],
    main = "nuget_native_libs_layout_test.py",
    args = ["$(location :nuget_native_libs_test_fixture)"],
    data = [":nuget_native_libs_test_fixture"],
)
