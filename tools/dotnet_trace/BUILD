load("@rules_dotnet//dotnet:defs.bzl", "dotnet_tool")

package(default_visibility = ["//visibility:public"])

dotnet_tool(
    name = "dotnet_trace_tool",
    target_frameworks = ["net6.0"],
    entrypoint = {"net6.0": "dotnet-trace.dll"},
    runner = {"net6.0": "dotnet"},
    deps = "@dotnet_trace//:tools",
)
