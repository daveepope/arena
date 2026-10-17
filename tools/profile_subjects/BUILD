load("@rules_dotnet//dotnet:defs.bzl", "csharp_binary")
load("@rules_rust//rust:defs.bzl", "rust_binary")

package(default_visibility = ["//visibility:public"])

exports_files([
    "busy_loop.py",
    "BusyLoop.java",
])

rust_binary(
    name = "busy_loop_native",
    srcs = ["busy_loop.rs"],
    edition = "2021",
)

csharp_binary(
    name = "busy_loop_dotnet",
    target_frameworks = ["net8.0"],
    srcs = ["Program.cs"],
)
