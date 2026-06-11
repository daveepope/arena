def _version_repo_impl(repository_ctx):
    version = repository_ctx.read(repository_ctx.attr.version_file).strip()
    if not version:
        fail("VERSION is empty")
    repository_ctx.file("BUILD.bazel", "")
    repository_ctx.file(
        "defs.bzl",
        "ARENA_VERSION = \"{}\"\n".format(version),
    )

version_repo = repository_rule(
    implementation = _version_repo_impl,
    attrs = {
        "version_file": attr.label(
            mandatory = True,
            allow_single_file = ["VERSION"],
        ),
    },
)
