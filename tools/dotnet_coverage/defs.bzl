"""Coverage-instrumented replacement for `csharp_test`.

rules_dotnet has no coverage instrumentation support for `csharp_test`
(tracked upstream at https://github.com/bazel-contrib/rules_dotnet/issues/359):
its test launcher does a plain `dotnet exec` with no `COVERAGE_DIR` handling,
so `bazel coverage` collects nothing for it.

`dotnet_coverage_test` builds the test as a testonly `csharp_binary` and runs
it through a wrapper that, only when `bazel coverage` sets `COVERAGE_DIR`,
collects real line coverage with `dotnet-coverage` (an attach-based collector,
so it needs neither MSBuild nor the VSTest host that rules_dotnet doesn't use)
and converts the result to the lcov format `bazel coverage` expects.

This relies on `DotnetBinaryInfo`/`DotnetAssemblyRuntimeInfo` and
`to_rlocation_path`, which are internal (non-public) rules_dotnet APIs, to
recover the `.pdb` files rules_dotnet builds but does not add to a test's
runfiles; without them dotnet-coverage can't map instrumented IL back to
source lines. A rules_dotnet upgrade could rename or remove these.
"""

load("@rules_dotnet//dotnet:defs.bzl", "csharp_binary", "csharp_test")
load("@rules_dotnet//dotnet/private:common.bzl", "to_rlocation_path")
load("@rules_dotnet//dotnet/private:providers.bzl", "DotnetBinaryInfo")
load(":instrument_files.bzl", "include_files_args")

_TEST_ONLY_ATTRS = ["size", "timeout", "flaky", "shard_count", "local", "tags", "args"]

def _transitive_pdbs(binary_info):
    pdbs = []
    for runtime_dep in binary_info.transitive_runtime_deps:
        pdbs.extend(runtime_dep.pdbs)
    return pdbs

def _dotnet_coverage_test_impl(ctx):
    binary = ctx.attr.binary
    binary_info = binary[DotnetBinaryInfo]

    wrapper = ctx.actions.declare_file(ctx.label.name + "_coverage_wrapper.sh")
    ctx.actions.expand_template(
        template = ctx.file._wrapper_template,
        output = wrapper,
        substitutions = {
            "TEMPLATED_binary": to_rlocation_path(ctx, binary.files_to_run.executable),
            "TEMPLATED_coverage_tool": to_rlocation_path(ctx, ctx.executable._coverage_tool),
            "TEMPLATED_lcov_converter": to_rlocation_path(ctx, ctx.executable._lcov_converter),
            "TEMPLATED_include_files_args": include_files_args(ctx.attr.instrument_files),
        },
        is_executable = True,
    )

    runfiles = ctx.runfiles(files = _transitive_pdbs(binary_info))
    runfiles = runfiles.merge(binary[DefaultInfo].default_runfiles)
    runfiles = runfiles.merge(ctx.attr._coverage_tool[DefaultInfo].default_runfiles)
    runfiles = runfiles.merge(ctx.attr._lcov_converter[DefaultInfo].default_runfiles)
    runfiles = runfiles.merge(ctx.attr._bash_runfiles[DefaultInfo].default_runfiles)

    return [
        DefaultInfo(executable = wrapper, runfiles = runfiles),
        coverage_common.instrumented_files_info(ctx, dependency_attributes = ["binary"], extensions = ["cs"]),
    ]

_dotnet_coverage_test = rule(
    implementation = _dotnet_coverage_test_impl,
    test = True,
    attrs = {
        "binary": attr.label(
            mandatory = True,
            executable = True,
            cfg = "target",
            providers = [DotnetBinaryInfo],
            doc = "A testonly csharp_binary to run, and to instrument under `bazel coverage`.",
        ),
        "instrument_files": attr.string_list(
            mandatory = True,
            allow_empty = False,
            doc = "`dotnet-coverage collect -if` glob(s) scoping which assemblies to instrument.",
        ),
        "_coverage_tool": attr.label(
            default = Label("//tools/dotnet_coverage:dotnet_coverage_tool"),
            executable = True,
            cfg = "target",
        ),
        "_lcov_converter": attr.label(
            default = Label("//tools/dotnet_coverage:cobertura_to_lcov"),
            executable = True,
            cfg = "target",
        ),
        "_wrapper_template": attr.label(
            default = Label("//tools/dotnet_coverage:coverage_wrapper.sh.tpl"),
            allow_single_file = True,
        ),
        "_bash_runfiles": attr.label(default = Label("@bazel_tools//tools/bash/runfiles")),
    },
)

def dotnet_coverage_test(name, instrument_files, **kwargs):
    """`csharp_test` replacement that produces real data under `bazel coverage`.

    On Windows the coverage wrapper's shell-script launcher has no native
    equivalent, and coverage is already collected on Ubuntu, so this produces
    a plain `csharp_test` there instead, marked `target_compatible_with` the
    complementary platform of the coverage-wrapped test so `bazel test //...`
    picks up exactly one of the two per platform without needing them to
    share a single label (neither `alias` nor `test_suite` can conditionally
    stand in for a test target: `alias` isn't recognized by `bazel test`'s
    target-pattern expansion, and `test_suite.tests` doesn't support `select`).
    """
    test_kwargs = {}
    for attr_name in _TEST_ONLY_ATTRS:
        if attr_name in kwargs:
            test_kwargs[attr_name] = kwargs.pop(attr_name)

    csharp_binary(
        name = name + "_bin",
        testonly = True,
        **kwargs
    )

    _dotnet_coverage_test(
        name = name + "_coverage",
        binary = ":%s_bin" % name,
        instrument_files = instrument_files,
        target_compatible_with = select({
            "@platforms//os:windows": ["@platforms//:incompatible"],
            "//conditions:default": [],
        }),
        **test_kwargs
    )

    plain_kwargs = dict(kwargs)
    plain_kwargs.update(test_kwargs)
    csharp_test(
        name = name + "_plain",
        target_compatible_with = select({
            "@platforms//os:windows": [],
            "//conditions:default": ["@platforms//:incompatible"],
        }),
        **plain_kwargs
    )
