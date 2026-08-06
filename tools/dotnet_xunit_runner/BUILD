load("@rules_dotnet//dotnet:defs.bzl", "csharp_library", "import_library")

package(default_visibility = ["//visibility:public"])

import_library(
    name = "xunit_abstractions_lib",
    library_name = "xunit.abstractions",
    libs = ["@xunit_abstractions//:libs"],
    refs = ["@xunit_abstractions//:refs"],
    tags = ["manual"],
)

import_library(
    name = "xunit_extensibility_lib",
    library_name = "xunit.extensibility.execution",
    libs = ["@xunit_extensibility//:libs"],
    refs = ["@xunit_extensibility//:refs"],
    tags = ["manual"],
)

filegroup(
    name = "xunit_core_netstandard",
    srcs = ["@xunit_extensibility_core//:lib/netstandard1.1/xunit.core.dll"],
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

csharp_library(
    name = "xunit_console_runner",
    target_frameworks = ["net8.0"],
    srcs = ["XunitConsoleRunner.cs"],
    deps = [
        ":xunit_abstractions_lib",
        ":xunit_extensibility_core_lib",
        ":xunit_extensibility_lib",
        ":xunit_runner_utility_lib",
    ],
)
