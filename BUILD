load("//tools:version_sync.bzl", "sync_arena_version")

exports_files(
    ["VERSION"],
    visibility = ["//visibility:public"],
)

sync_arena_version(
    name = "version_sync",
    stamp = "version_sync.stamp",
)
