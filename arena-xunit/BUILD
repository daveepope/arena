load("@arena_version//:defs.bzl", "ARENA_VERSION")
load("@rules_dotnet//dotnet:defs.bzl", "dotnet_library", "dotnet_test", "dotnet_nuget_package")

_ARENA_XUNIT_SRCS = glob(["src/**/*.cs"])

dotnet_library(
    name = "arena_xunit_lib",
    srcs = _ARENA_XUNIT_SRCS,
    netstandard_tfm = "netstandard2.0",
    visibility = ["//visibility:public"],
    deps = [
        "@nuget//:xunit.core",
        "@nuget//:Newtonsoft.Json",
        "@nuget//:Microsoft.Extensions.Logging.Abstractions",
    ],
)

dotnet_test(
    name = "arena_xunit_unit_test",
    size = "small",
    srcs = glob(["tests/Unit/**/*.cs"]),
    deps = [
        ":arena_xunit_lib",
        "@nuget//:xunit.assert",
        "@nuget//:xunit.runner.visualstudio",
    ],
)

dotnet_test(
    name = "arena_xunit_lifecycle_component_test",
    size = "small",
    tags = ["component_test"],
    srcs = glob(["tests/Component/**/*.cs"]),
    data = ["//arena-ffi:arena_ffi_shared"],
    deps = [
        ":arena_xunit_lib",
        "@nuget//:xunit.assert",
        "//examples:ephemeral_test_runtime_dotnet",
    ],
)

dotnet_nuget_package(
    name = "arena_xunit_nuget",
    package = ":arena_xunit_lib",
    id = "Arena.Xunit",
    version = ARENA_VERSION,
    tags = ["manual"],
    summary = "xUnit API and fixtures for Arena (via the C FFI)",
    authors = ["Arena contributors"],
    license_url = "https://github.com/daveepope/arena/blob/main/LICENSE",
    project_url = "https://github.com/daveepope/arena",
    package_refs = [
        "@nuget//:xunit.core",
    ],
    deps = [
        "//arena-ffi:arena_ffi_shared",
    ],
)
