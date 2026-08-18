load("@rules_dotnet//dotnet/private:common.bzl", "to_rlocation_path")

def _dotnet_nuget_smoke_check_impl(ctx):
    toolchain = ctx.toolchains["@rules_dotnet//dotnet:toolchain_type"]
    runtime = toolchain.runtime

    launcher = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.expand_template(
        template = ctx.file._launcher_template,
        output = launcher,
        substitutions = {
            "TEMPLATED_dotnet": to_rlocation_path(ctx, runtime.files_to_run.executable),
            "TEMPLATED_nupkg": to_rlocation_path(ctx, ctx.file.package),
            "TEMPLATED_nuget_id": ctx.attr.package_id,
        },
        is_executable = True,
    )

    runfiles = ctx.runfiles(files = [ctx.file.package] + toolchain.dotnetinfo.runtime_files)
    runfiles = runfiles.merge(ctx.attr._bash_runfiles[DefaultInfo].default_runfiles)

    return [DefaultInfo(executable = launcher, runfiles = runfiles)]

dotnet_nuget_smoke_check = rule(
    implementation = _dotnet_nuget_smoke_check_impl,
    executable = True,
    attrs = {
        "package": attr.label(
            mandatory = True,
            allow_single_file = [".nupkg"],
        ),
        "package_id": attr.string(
            mandatory = True,
        ),
        "_launcher_template": attr.label(
            default = Label("//tools/predeploy_smoke/dotnet:smoke_test_nuget.sh.tpl"),
            allow_single_file = True,
        ),
        "_bash_runfiles": attr.label(default = Label("@bazel_tools//tools/bash/runfiles")),
    },
    toolchains = ["@rules_dotnet//dotnet:toolchain_type"],
)
