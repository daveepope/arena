def _sync_arena_version_impl(ctx):
    stamp = ctx.outputs.stamp
    sync = ctx.executable._sync
    ctx.actions.run_shell(
        mnemonic = "SyncArenaVersion",
        command = '"{}" && touch "{}"'.format(sync.path, stamp.path),
        tools = [sync],
        outputs = [stamp],
        execution_requirements = {
            "local": "1",
            "no-sandbox": "1",
        },
    )

sync_arena_version = rule(
    implementation = _sync_arena_version_impl,
    attrs = {
        "stamp": attr.output(mandatory = True),
        "_sync": attr.label(
            default = Label("//scripts:sync_version"),
            executable = True,
            cfg = "exec",
        ),
    },
)
