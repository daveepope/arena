"""`csharp_nuget_package`: builds a `.nupkg` from a `csharp_library`.

`rules_dotnet` has no NuGet packaging rule (only consumption-side
`nuget_archive`/`import_library`), so this builds the `.nupkg` directly as a
zip via `rules_pkg`, expanding a `.nuspec` template rather than shelling out
to `dotnet pack` (which needs a non-hermetic MSBuild/NuGet restore).

A `.nupkg` is an Open Packaging Conventions (OPC) archive, not a plain zip:
besides the `.nuspec` and `lib/<tfm>/*.dll`, it needs `[Content_Types].xml`,
`_rels/.rels`, and a `package/services/metadata/core-properties/*.psmdcp`
file, or OPC-based readers (including NuGet.org's own ingestion, which opens
packages via `System.IO.Packaging`) may reject it.

`dotnet_nuget_push` runs `dotnet nuget push` through the hermetic `dotnet`
runtime `rules_dotnet`'s own toolchain already pins (`MODULE.bazel`'s
`dotnet.toolchain(dotnet_version = ...)`), the same mechanism
`tools/dotnet_coverage`'s `dotnet_tool` target uses - so CI doesn't need a
separate `actions/setup-dotnet` install that could drift from the pinned
version. `to_rlocation_path`/toolchain resolution are internal (non-public)
rules_dotnet APIs, per the same tradeoff `tools/dotnet_coverage/defs.bzl`
documents.
"""

load("@rules_dotnet//dotnet/private:common.bzl", "to_rlocation_path")
load("@rules_pkg//pkg:mappings.bzl", "pkg_files", "strip_prefix")
load("@rules_pkg//pkg:zip.bzl", "pkg_zip")
load(":nuget_version.bzl", "nuget_package_version")

def _expand_package_xml_impl(ctx):
    prerelease = ctx.var.get("arena_xunit_prerelease", "") if ctx.attr.allow_prerelease else ""
    version = nuget_package_version(ctx.attr.version, prerelease)
    out = ctx.actions.declare_file(ctx.attr.output_name)
    ctx.actions.expand_template(
        template = ctx.file.template,
        output = out,
        substitutions = {
            "{dependencies}": ctx.attr.dependencies,
            "{id}": ctx.attr.package_id,
            "{version}": version,
        },
    )
    return [DefaultInfo(files = depset([out]))]

_expand_package_xml = rule(
    implementation = _expand_package_xml_impl,
    attrs = {
        "allow_prerelease": attr.bool(
            default = False,
            doc = "If false, ignore the `arena_xunit_prerelease` --define entirely, " +
                  "so a release target can't accidentally pick up a stray prerelease " +
                  "suffix from the build environment.",
        ),
        "dependencies": attr.string(default = ""),
        "output_name": attr.string(mandatory = True),
        "package_id": attr.string(mandatory = True),
        "template": attr.label(allow_single_file = True, mandatory = True),
        "version": attr.string(mandatory = True),
    },
)

def csharp_nuget_package(
        name,
        package_id,
        version,
        nuspec_template,
        dll,
        target_framework,
        dependencies = "",
        allow_prerelease = False,
        readme = None,
        tags = None,
        visibility = None):
    """Builds `name.nupkg` from `dll` (a `csharp_library`) and `nuspec_template`.

    If `allow_prerelease` is true, the prerelease suffix (if any) is read from
    the `arena_xunit_prerelease` `--define`, e.g.
    `--define=arena_xunit_prerelease=pr123`, at build time - it's not a macro
    argument since macros run before flags are configured. If false (the
    default, intended for release targets), that `--define` is ignored
    entirely, so a stray/leaked flag can't silently turn a release build into
    a prerelease one.

    Args:
        name: target name; the output is `name.nupkg`.
        package_id: the NuGet package ID, e.g. "Arena.Xunit".
        version: the release version, e.g. "3.5.2" (from `ARENA_VERSION`).
        nuspec_template: a `.nuspec` template with `{id}`/`{version}`/
            `{dependencies}` placeholders.
        dll: the `csharp_library` target whose output assembly is packaged.
        target_framework: the NuGet `lib/<target_framework>/` folder name,
            e.g. "netstandard2.0".
        dependencies: pre-formatted `<dependency .../>` XML to substitute
            into the template's `{dependencies}` placeholder.
        allow_prerelease: whether this target may receive the
            `arena_xunit_prerelease` `--define`; set true only for
            snapshot/prerelease targets.
        readme: optional file to package at the `.nupkg` root; if set, the
            `nuspec_template` must reference it by the same basename via a
            `<readme>` element, since NuGet validates that the referenced
            file actually exists in the package.
        tags: passed through to all generated targets.
        visibility: applied to the `name.nupkg` output target.
    """
    nuspec_name = name + "_nuspec"
    _expand_package_xml(
        name = nuspec_name,
        output_name = name + "/" + package_id + ".nuspec",
        template = nuspec_template,
        package_id = package_id,
        version = version,
        dependencies = dependencies,
        allow_prerelease = allow_prerelease,
        tags = tags,
    )

    rels_name = name + "_rels"
    _expand_package_xml(
        name = rels_name,
        output_name = name + "/rels.xml",
        template = "//tools/dotnet_nuget:rels_template.xml",
        package_id = package_id,
        version = version,
        allow_prerelease = allow_prerelease,
        tags = tags,
    )

    core_properties_name = name + "_core_properties"
    _expand_package_xml(
        name = core_properties_name,
        output_name = name + "/core_properties.psmdcp",
        template = "//tools/dotnet_nuget:core_properties_template.psmdcp",
        allow_prerelease = allow_prerelease,
        package_id = package_id,
        version = version,
        tags = tags,
    )

    opc_files_name = name + "_opc_files"
    pkg_files(
        name = opc_files_name,
        srcs = [
            "//tools/dotnet_nuget:content_types.xml",
            ":" + rels_name,
            ":" + core_properties_name,
        ],
        renames = {
            "//tools/dotnet_nuget:content_types.xml": "[Content_Types].xml",
            ":" + rels_name: "_rels/.rels",
            ":" + core_properties_name: "package/services/metadata/core-properties/primary.psmdcp",
        },
        tags = tags,
    )

    nuspec_files_name = name + "_nuspec_files"
    pkg_files(
        name = nuspec_files_name,
        srcs = [":" + nuspec_name],
        strip_prefix = strip_prefix.files_only(),
        tags = tags,
    )

    lib_files_name = name + "_lib_files"
    pkg_files(
        name = lib_files_name,
        srcs = [dll],
        strip_prefix = strip_prefix.files_only(),
        prefix = "lib/" + target_framework,
        tags = tags,
    )

    zip_srcs = [
        ":" + opc_files_name,
        ":" + nuspec_files_name,
        ":" + lib_files_name,
    ]

    if readme:
        readme_files_name = name + "_readme_files"
        pkg_files(
            name = readme_files_name,
            srcs = [readme],
            strip_prefix = strip_prefix.files_only(),
            tags = tags,
        )
        zip_srcs.append(":" + readme_files_name)

    pkg_zip(
        name = name,
        srcs = zip_srcs,
        out = name + ".nupkg",
        tags = tags,
        visibility = visibility,
    )

def _dotnet_nuget_push_impl(ctx):
    toolchain = ctx.toolchains["@rules_dotnet//dotnet:toolchain_type"]
    runtime = toolchain.runtime

    launcher = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.expand_template(
        template = ctx.file._launcher_template,
        output = launcher,
        substitutions = {
            "TEMPLATED_dotnet": to_rlocation_path(ctx, runtime.files_to_run.executable),
            "TEMPLATED_package": to_rlocation_path(ctx, ctx.file.package),
        },
        is_executable = True,
    )

    runfiles = ctx.runfiles(files = [ctx.file.package] + toolchain.dotnetinfo.runtime_files)
    runfiles = runfiles.merge(ctx.attr._bash_runfiles[DefaultInfo].default_runfiles)

    return [DefaultInfo(executable = launcher, runfiles = runfiles)]

dotnet_nuget_push = rule(
    implementation = _dotnet_nuget_push_impl,
    executable = True,
    doc = """Runs `dotnet nuget push <package>` via the pinned dotnet toolchain.

    Reads `NUGET_API_KEY` (e.g. from a `NuGet/login` OIDC step's output) and
    `NUGET_SOURCE` from the environment at `bazel run` time - not as build
    flags, so the key never appears in a recorded Bazel command line.
    """,
    attrs = {
        "package": attr.label(
            mandatory = True,
            allow_single_file = [".nupkg"],
            doc = "The `csharp_nuget_package` output to push.",
        ),
        "_launcher_template": attr.label(
            default = Label("//tools/dotnet_nuget:nuget_push.sh.tpl"),
            allow_single_file = True,
        ),
        "_bash_runfiles": attr.label(default = Label("@bazel_tools//tools/bash/runfiles")),
    },
    toolchains = ["@rules_dotnet//dotnet:toolchain_type"],
)
