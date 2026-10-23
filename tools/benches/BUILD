sh_binary(
    name = "bench_all",
    srcs = ["run_all.sh"],
    data = [
        "hardware_profile.sh",
        "//tools/benches/dotnet:bench_dotnet",
        "//tools/benches/maven:bench_maven",
        "//tools/benches/pypi:bench_pypi",
        "@apache_maven//:dist",
    ],
    tags = ["manual"],
)

sh_test(
    name = "hardware_profile_test",
    size = "small",
    srcs = ["hardware_profile_test.sh"],
    data = ["hardware_profile.sh"],
    deps = ["@bazel_tools//tools/bash/runfiles"],
)
