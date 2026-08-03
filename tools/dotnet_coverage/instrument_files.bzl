"""Validation and `dotnet-coverage collect -if` argument building for `instrument_files` globs.

Kept apart from `defs.bzl` so `is_unscoped_instrument_pattern` can be loaded
directly by `instrument_files_test.bzl`.
"""

def is_unscoped_instrument_pattern(pattern):
    """Returns True if `pattern`'s basename is a bare `*.dll`-style glob.

    A bare basename wildcard also matches every other assembly dotnet-coverage
    finds alongside the test binary in its runfiles directory, including the
    vendored .NET runtime, which is unstable to instrument and can exhaust
    host memory.
    """
    basename = pattern.rsplit("/", 1)[-1]
    return basename.lower() in ("*.dll", "*", "**")

def _shell_single_quote(s):
    return "'" + s.replace("'", "'\\''") + "'"

def include_files_args(patterns):
    for pattern in patterns:
        if is_unscoped_instrument_pattern(pattern):
            fail(("instrument_files pattern %r is an unscoped wildcard: it also " +
                  "matches the vendored .NET runtime shipped in the test's " +
                  "runfiles, which is unstable to instrument and can exhaust " +
                  "host memory. Scope it to this target's own assembly instead.") % pattern)
    return " ".join(["-if %s" % _shell_single_quote(pattern) for pattern in patterns])
