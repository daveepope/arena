"""Unit tests for `nuget_package_version`/`is_valid_prerelease_label`."""

load("@bazel_skylib//lib:unittest.bzl", "asserts", "unittest")
load(":nuget_version.bzl", "is_valid_prerelease_label", "nuget_package_version")

def _nuget_package_version_noPrerelease_returnsVersion(ctx):
    env = unittest.begin(ctx)
    asserts.equals(env, "3.5.2", nuget_package_version("3.5.2", ""))
    return unittest.end(env)

def _nuget_package_version_withPrerelease_appendsSuffix(ctx):
    env = unittest.begin(ctx)
    asserts.equals(env, "3.5.2-pr123", nuget_package_version("3.5.2", "pr123"))
    return unittest.end(env)

def _is_valid_prerelease_label_alphanumericHyphen_returnsTrue(ctx):
    env = unittest.begin(ctx)
    asserts.true(env, is_valid_prerelease_label("pr123"))
    asserts.true(env, is_valid_prerelease_label("pr-123-beta"))
    return unittest.end(env)

def _is_valid_prerelease_label_withDot_returnsFalse(ctx):
    env = unittest.begin(ctx)
    asserts.false(env, is_valid_prerelease_label("pr.123"))
    return unittest.end(env)

def _is_valid_prerelease_label_withUnderscore_returnsFalse(ctx):
    env = unittest.begin(ctx)
    asserts.false(env, is_valid_prerelease_label("pr_123"))
    return unittest.end(env)

def _is_valid_prerelease_label_empty_returnsFalse(ctx):
    env = unittest.begin(ctx)
    asserts.false(env, is_valid_prerelease_label(""))
    return unittest.end(env)

_nuget_package_version_noPrerelease_returnsVersion_test = unittest.make(
    _nuget_package_version_noPrerelease_returnsVersion,
)
_nuget_package_version_withPrerelease_appendsSuffix_test = unittest.make(
    _nuget_package_version_withPrerelease_appendsSuffix,
)
_is_valid_prerelease_label_alphanumericHyphen_returnsTrue_test = unittest.make(
    _is_valid_prerelease_label_alphanumericHyphen_returnsTrue,
)
_is_valid_prerelease_label_withDot_returnsFalse_test = unittest.make(
    _is_valid_prerelease_label_withDot_returnsFalse,
)
_is_valid_prerelease_label_withUnderscore_returnsFalse_test = unittest.make(
    _is_valid_prerelease_label_withUnderscore_returnsFalse,
)
_is_valid_prerelease_label_empty_returnsFalse_test = unittest.make(
    _is_valid_prerelease_label_empty_returnsFalse,
)

def nuspec_file_test_suite(name):
    unittest.suite(
        name,
        _nuget_package_version_noPrerelease_returnsVersion_test,
        _nuget_package_version_withPrerelease_appendsSuffix_test,
        _is_valid_prerelease_label_alphanumericHyphen_returnsTrue_test,
        _is_valid_prerelease_label_withDot_returnsFalse_test,
        _is_valid_prerelease_label_withUnderscore_returnsFalse_test,
        _is_valid_prerelease_label_empty_returnsFalse_test,
    )
