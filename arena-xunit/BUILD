load("@arena_version//:defs.bzl", "ARENA_VERSION")
load("@rules_dotnet//dotnet:defs.bzl", "csharp_library", "csharp_test", "import_library")
load("//tools/dotnet_coverage:defs.bzl", "dotnet_coverage_test")
load("//tools/dotnet_nuget:defs.bzl", "csharp_nuget_package", "dotnet_nuget_push")
load(
    "//tools/dotnet_nuget:pinned_dependency_versions.bzl",
    "MS_LOGGING_ABSTRACTIONS_VERSION",
    "NEWTONSOFT_JSON_VERSION",
)

_ARENA_XUNIT_SRCS = glob(["src/**/*.cs"])

_ARENA_XUNIT_NUGET_ID = "ArenaDotnet.Xunit"

_ARENA_XUNIT_PREBUILT_RELEASE_NATIVE_LIBS = glob(
    ["prebuilt/**"],
    allow_empty = True,
)

# Sourced from pinned_dependency_versions.bzl, the same constants MODULE.bazel's
# nuget_archive pins use, so this can't drift from what arena_xunit_lib is
# actually built against.
_ARENA_XUNIT_NUGET_DEPENDENCIES = "\n".join([
    '<dependency id="Newtonsoft.Json" version="{}" />'.format(NEWTONSOFT_JSON_VERSION),
    '<dependency id="Microsoft.Extensions.Logging.Abstractions" version="{}" />'.format(MS_LOGGING_ABSTRACTIONS_VERSION),
])

# Import nuget packages from MODULE.bazel nuget_archive repos
import_library(
    name = "xunit_extensibility_lib",
    library_name = "xunit.extensibility.execution",
    libs = ["@xunit_extensibility//:libs"],
    refs = ["@xunit_extensibility//:refs"],
    tags = ["manual"],
)

import_library(
    name = "xunit_assert_lib",
    library_name = "xunit.assert",
    libs = ["@xunit_assert//:libs"],
    refs = ["@xunit_assert//:refs"],
    tags = ["manual"],
)

import_library(
    name = "newtonsoft_json_lib",
    library_name = "Newtonsoft.Json",
    libs = ["@newtonsoft_json//:libs"],
    refs = ["@newtonsoft_json//:refs"],
    tags = ["manual"],
)

import_library(
    name = "ms_logging_abstractions_lib",
    library_name = "Microsoft.Extensions.Logging.Abstractions",
    libs = ["@ms_logging_abstractions//:libs"],
    refs = ["@ms_logging_abstractions//:refs"],
    tags = ["manual"],
)

import_library(
    name = "xunit_abstractions_lib",
    library_name = "xunit.abstractions",
    libs = ["@xunit_abstractions//:libs"],
    refs = ["@xunit_abstractions//:refs"],
    tags = ["manual"],
)

filegroup(
    name = "xunit_core_netstandard",
    srcs = ["@xunit_extensibility_core//:lib/netstandard1.1/xunit.core.dll"],
    visibility = ["//visibility:public"],
)

import_library(
    name = "xunit_extensibility_core_lib",
    library_name = "xunit.core",
    libs = [":xunit_core_netstandard"],
    refs = [":xunit_core_netstandard"],
    tags = ["manual"],
)

filegroup(
    name = "xunit_runner_utility_netstandard",
    srcs = ["@xunit_runner_utility//:lib/netstandard1.5/xunit.runner.utility.netstandard15.dll"],
)

import_library(
    name = "xunit_runner_utility_lib",
    library_name = "xunit.runner.utility.netstandard15",
    libs = [":xunit_runner_utility_netstandard"],
    refs = [":xunit_runner_utility_netstandard"],
    tags = ["manual"],
)

filegroup(
    name = "xunit_runner_reporters_netstandard",
    srcs = ["@xunit_runner_reporters//:lib/netstandard1.1/xunit.runner.reporters.netstandard11.dll"],
)

import_library(
    name = "xunit_runner_reporters_lib",
    library_name = "xunit.runner.reporters.netstandard11",
    libs = [":xunit_runner_reporters_netstandard"],
    refs = [":xunit_runner_reporters_netstandard"],
    tags = ["manual"],
)

csharp_library(
    name = "arena_xunit_lib",
    srcs = _ARENA_XUNIT_SRCS,
    out = "ArenaDotnet.Xunit",
    target_frameworks = ["netstandard2.0"],
    visibility = ["//visibility:public"],
    compiler_options = ["/langversion:latest", "/debug:pdbonly"],
    deps = [
        ":xunit_extensibility_lib",
        ":xunit_extensibility_core_lib",
        ":xunit_abstractions_lib",
        ":newtonsoft_json_lib",
        ":ms_logging_abstractions_lib",
    ],
    data = ["//arena-ffi:arena_ffi_shared"],
)

csharp_nuget_package(
    name = "arena_xunit_nuget",
    package_id = _ARENA_XUNIT_NUGET_ID,
    version = ARENA_VERSION,
    nuspec_template = "nuspec_template.xml",
    dll = ":arena_xunit_lib",
    target_framework = "netstandard2.0",
    dependencies = _ARENA_XUNIT_NUGET_DEPENDENCIES,
    native_libs = _ARENA_XUNIT_PREBUILT_RELEASE_NATIVE_LIBS,
    readme = "README.md",
    tags = ["manual"],
    visibility = ["//visibility:public"],
)

csharp_nuget_package(
    name = "arena_xunit_nuget_snapshot",
    package_id = _ARENA_XUNIT_NUGET_ID,
    version = ARENA_VERSION,
    nuspec_template = "nuspec_template.xml",
    dll = ":arena_xunit_lib",
    target_framework = "netstandard2.0",
    dependencies = _ARENA_XUNIT_NUGET_DEPENDENCIES,
    native_libs = _ARENA_XUNIT_PREBUILT_RELEASE_NATIVE_LIBS,
    allow_prerelease = True,
    readme = "README.md",
    tags = ["manual"],
    visibility = ["//visibility:public"],
)

dotnet_nuget_push(
    name = "arena_xunit_nuget_push",
    package = ":arena_xunit_nuget",
    tags = ["manual"],
)

dotnet_nuget_push(
    name = "arena_xunit_nuget_snapshot_push",
    package = ":arena_xunit_nuget_snapshot",
    tags = ["manual"],
)

dotnet_coverage_test(
    name = "arena_xunit_unit_test",
    size = "small",
    target_frameworks = ["net8.0"],
    instrument_files = ["**/ArenaDotnet.Xunit.dll"],
    srcs = glob(["tests/Unit/**/*.cs"]),
    data = ["//arena-ffi:arena_ffi_shared", ":xunit_core_netstandard"],
    deps = [
        ":arena_xunit_lib",
        ":xunit_assert_lib",
        ":xunit_abstractions_lib",
        ":xunit_extensibility_lib",
        ":xunit_extensibility_core_lib",
        ":xunit_runner_utility_lib",
        ":xunit_runner_reporters_lib",
        ":newtonsoft_json_lib",
        ":ms_logging_abstractions_lib",
        "//tools/dotnet_xunit_runner:xunit_console_runner",
    ],
)

csharp_test(
    name = "arena_xunit_lifecycle_component_test",
    size = "small",
    tags = ["component_test"],
    target_frameworks = ["net8.0"],
    srcs = glob(["tests/Component/**/*.cs"]),
    data = ["//arena-ffi:arena_ffi_shared", ":xunit_core_netstandard"],
    deps = [
        ":arena_xunit_lib",
        ":xunit_assert_lib",
        ":xunit_abstractions_lib",
        ":xunit_extensibility_lib",
        ":xunit_extensibility_core_lib",
        ":xunit_runner_utility_lib",
        ":xunit_runner_reporters_lib",
        ":newtonsoft_json_lib",
        ":ms_logging_abstractions_lib",
        "//tools/dotnet_xunit_runner:xunit_console_runner",
    ],
)
