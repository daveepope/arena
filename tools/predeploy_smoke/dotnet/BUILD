load("@rules_dotnet//dotnet:defs.bzl", "csharp_test", "import_library")
load(":defs.bzl", "dotnet_nuget_smoke_check")

exports_files(["smoke_test_nuget.sh.tpl"])

dotnet_nuget_smoke_check(
    name = "nuget_smoke_test",
    package = "//arena-xunit:arena_xunit_nuget",
    package_id = "ArenaDotnet.Xunit",
    tags = ["manual"],
)

dotnet_nuget_smoke_check(
    name = "nuget_snapshot_smoke_test",
    package = "//arena-xunit:arena_xunit_nuget_snapshot",
    package_id = "ArenaDotnet.Xunit",
    tags = ["manual"],
)

genrule(
    name = "bazel_consumer_managed_dll",
    srcs = ["//arena-xunit:arena_xunit_nuget"],
    outs = ["ArenaDotnet.Xunit.dll"],
    cmd = "unzip -p $(location //arena-xunit:arena_xunit_nuget) lib/netstandard2.0/ArenaDotnet.Xunit.dll > $@",
    tags = ["manual"],
)

config_setting(
    name = "_macos_arm64",
    constraint_values = [
        "@platforms//os:macos",
        "@platforms//cpu:arm64",
    ],
)

config_setting(
    name = "_macos_x86_64",
    constraint_values = [
        "@platforms//os:macos",
        "@platforms//cpu:x86_64",
    ],
)

genrule(
    name = "bazel_consumer_native_lib",
    srcs = select({
        ":_macos_arm64": ["//arena-xunit:prebuilt_native_libs_osx_arm64"],
        ":_macos_x86_64": ["//arena-xunit:prebuilt_native_libs_osx_x64"],
        "//conditions:default": ["//arena-xunit:prebuilt_native_libs_linux_x64"],
    }),
    outs = [
        "libarena_ffi_shared.so",
        "libarena_ffi_shared.dylib",
    ],
    cmd = "cp $(SRCS) $(RULEDIR)/libarena_ffi_shared.so && " +
          "cp $(SRCS) $(RULEDIR)/libarena_ffi_shared.dylib",
    tags = ["manual"],
)

import_library(
    name = "bazel_consumer_arena_xunit_import",
    library_name = "ArenaDotnet.Xunit",
    libs = [":bazel_consumer_managed_dll"],
    refs = [":bazel_consumer_managed_dll"],
    native = [":bazel_consumer_native_lib"],
    tags = ["manual"],
    deps = [
        "//arena-xunit:xunit_extensibility_lib",
        "//arena-xunit:xunit_extensibility_core_lib",
        "//arena-xunit:xunit_abstractions_lib",
        "//arena-xunit:newtonsoft_json_lib",
        "//arena-xunit:ms_logging_abstractions_lib",
    ],
)

csharp_test(
    name = "bazel_consumer_smoke_test",
    size = "small",
    tags = [
        "component_test",
        "manual",
    ],
    target_frameworks = ["net8.0"],
    nullable = "enable",
    srcs = [
        "BazelConsumerSmokeTest.cs",
        "Program.cs",
    ],
    data = ["//arena-xunit:xunit_core_netstandard"],
    deps = [
        ":bazel_consumer_arena_xunit_import",
        "//arena-xunit:xunit_assert_lib",
        "//arena-xunit:xunit_abstractions_lib",
        "//arena-xunit:xunit_extensibility_lib",
        "//arena-xunit:xunit_extensibility_core_lib",
        "//arena-xunit:xunit_runner_utility_lib",
        "//arena-xunit:xunit_runner_reporters_lib",
        "//arena-xunit:newtonsoft_json_lib",
        "//arena-xunit:ms_logging_abstractions_lib",
        "//tools/dotnet_xunit_runner:xunit_console_runner",
    ],
)
