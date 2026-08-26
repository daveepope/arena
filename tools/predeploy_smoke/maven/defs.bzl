load("@rules_dotnet//dotnet/private:common.bzl", "to_rlocation_path")
load("@rules_java//java/common:java_common.bzl", "java_common")

def _java_executable_rlocation_path(ctx, java_executable_runfiles_path):
    if java_executable_runfiles_path.startswith("../"):
        return java_executable_runfiles_path[3:]
    else:
        return ctx.workspace_name + "/" + java_executable_runfiles_path

def _maven_smoke_check_impl(ctx):
    jdk = ctx.attr._jdk[java_common.JavaRuntimeInfo]

    launcher = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.expand_template(
        template = ctx.file._launcher_template,
        output = launcher,
        substitutions = {
            "TEMPLATED_mvn": to_rlocation_path(ctx, ctx.file._mvn),
            "TEMPLATED_java_executable": _java_executable_rlocation_path(ctx, jdk.java_executable_runfiles_path),
            "TEMPLATED_jar": to_rlocation_path(ctx, ctx.file.jar),
            "TEMPLATED_pom": to_rlocation_path(ctx, ctx.file.pom),
            "TEMPLATED_native_jar": to_rlocation_path(ctx, ctx.file.native_classifier_jar),
            "TEMPLATED_native_classifier": ctx.attr.native_classifier,
            "TEMPLATED_group_id": ctx.attr.group_id,
            "TEMPLATED_artifact_id": ctx.attr.artifact_id,
        },
        is_executable = True,
    )

    runfiles = ctx.runfiles(files = [
        ctx.file.jar,
        ctx.file.pom,
        ctx.file.native_classifier_jar,
        ctx.file._mvn,
    ] + ctx.files._maven_dist)
    runfiles = runfiles.merge(ctx.runfiles(transitive_files = jdk.files))
    runfiles = runfiles.merge(ctx.attr._bash_runfiles[DefaultInfo].default_runfiles)

    return [DefaultInfo(executable = launcher, runfiles = runfiles)]

maven_smoke_check = rule(
    implementation = _maven_smoke_check_impl,
    executable = True,
    attrs = {
        "artifact_id": attr.string(mandatory = True),
        "group_id": attr.string(mandatory = True),
        "jar": attr.label(mandatory = True, allow_single_file = [".jar"]),
        "native_classifier": attr.string(mandatory = True),
        "native_classifier_jar": attr.label(mandatory = True, allow_single_file = [".jar"]),
        "pom": attr.label(mandatory = True, allow_single_file = [".xml"]),
        "_bash_runfiles": attr.label(default = Label("@bazel_tools//tools/bash/runfiles")),
        "_jdk": attr.label(default = Label("@bazel_tools//tools/jdk:current_java_runtime")),
        "_launcher_template": attr.label(
            default = Label("//tools/predeploy_smoke/maven:smoke_test_maven.sh.tpl"),
            allow_single_file = True,
        ),
        "_maven_dist": attr.label(default = Label("@apache_maven//:dist")),
        "_mvn": attr.label(default = Label("@apache_maven//:bin/mvn"), allow_single_file = True),
    },
)
