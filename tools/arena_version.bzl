load("//tools:version_repo.bzl", "version_repo")

def _arena_version_impl(_module_ctx):
    version_repo(
        name = "arena_version",
        version_file = Label("//:VERSION"),
    )

arena_version = module_extension(
    implementation = _arena_version_impl,
)
