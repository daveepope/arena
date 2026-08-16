"""Unit tests for `is_unscoped_instrument_pattern`."""

load("@bazel_skylib//lib:partial.bzl", "partial")
load("@bazel_skylib//lib:unittest.bzl", "asserts", "unittest")
load(":instrument_files.bzl", "is_unscoped_instrument_pattern")

def _is_unscoped_instrument_pattern_starDotDll_returnsTrue(ctx):
    env = unittest.begin(ctx)
    asserts.true(env, is_unscoped_instrument_pattern("*.dll"))
    return unittest.end(env)

def _is_unscoped_instrument_pattern_recursiveStarDotDll_returnsTrue(ctx):
    env = unittest.begin(ctx)
    asserts.true(env, is_unscoped_instrument_pattern("**/*.dll"))
    return unittest.end(env)

def _is_unscoped_instrument_pattern_uppercaseExtension_returnsTrue(ctx):
    env = unittest.begin(ctx)
    asserts.true(env, is_unscoped_instrument_pattern("**/*.DLL"))
    return unittest.end(env)

def _is_unscoped_instrument_pattern_directoryScopedWildcard_returnsTrue(ctx):
    env = unittest.begin(ctx)
    asserts.true(env, is_unscoped_instrument_pattern("some/dir/*.dll"))
    return unittest.end(env)

def _is_unscoped_instrument_pattern_namedAssembly_returnsFalse(ctx):
    env = unittest.begin(ctx)
    asserts.false(env, is_unscoped_instrument_pattern("**/arena_xunit_lib.dll"))
    return unittest.end(env)

def _is_unscoped_instrument_pattern_partialNameWildcard_returnsFalse(ctx):
    env = unittest.begin(ctx)
    asserts.false(env, is_unscoped_instrument_pattern("**/arena_*.dll"))
    return unittest.end(env)

_is_unscoped_instrument_pattern_starDotDll_returnsTrue_test = unittest.make(
    _is_unscoped_instrument_pattern_starDotDll_returnsTrue,
)
_is_unscoped_instrument_pattern_recursiveStarDotDll_returnsTrue_test = unittest.make(
    _is_unscoped_instrument_pattern_recursiveStarDotDll_returnsTrue,
)
_is_unscoped_instrument_pattern_uppercaseExtension_returnsTrue_test = unittest.make(
    _is_unscoped_instrument_pattern_uppercaseExtension_returnsTrue,
)
_is_unscoped_instrument_pattern_directoryScopedWildcard_returnsTrue_test = unittest.make(
    _is_unscoped_instrument_pattern_directoryScopedWildcard_returnsTrue,
)
_is_unscoped_instrument_pattern_namedAssembly_returnsFalse_test = unittest.make(
    _is_unscoped_instrument_pattern_namedAssembly_returnsFalse,
)
_is_unscoped_instrument_pattern_partialNameWildcard_returnsFalse_test = unittest.make(
    _is_unscoped_instrument_pattern_partialNameWildcard_returnsFalse,
)

def instrument_files_test_suite(name):
    unittest.suite(
        name,
        partial.make(_is_unscoped_instrument_pattern_starDotDll_returnsTrue_test, size = "small"),
        partial.make(_is_unscoped_instrument_pattern_recursiveStarDotDll_returnsTrue_test, size = "small"),
        partial.make(_is_unscoped_instrument_pattern_uppercaseExtension_returnsTrue_test, size = "small"),
        partial.make(_is_unscoped_instrument_pattern_directoryScopedWildcard_returnsTrue_test, size = "small"),
        partial.make(_is_unscoped_instrument_pattern_namedAssembly_returnsFalse_test, size = "small"),
        partial.make(_is_unscoped_instrument_pattern_partialNameWildcard_returnsFalse_test, size = "small"),
    )
