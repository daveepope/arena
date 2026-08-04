"""NuGet package version-string computation, kept apart from `defs.bzl` so
`nuget_package_version` can be loaded directly by `nuget_version_test.bzl`.
"""

_VALID_PRERELEASE_CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-"

def is_valid_prerelease_label(label):
    """Returns True if `label` is alphanumeric-and-hyphens only.

    NuGet SemVer2 prerelease labels (the part after the `-` in `1.2.3-pr123`)
    must not contain dots or other characters that would split into
    additional SemVer2 identifiers.
    """
    if not label:
        return False
    for c in label.elems():
        if c not in _VALID_PRERELEASE_CHARS:
            return False
    return True

def nuget_package_version(version, prerelease):
    """Returns `version`, or `version-prerelease` if `prerelease` is set.

    Fails if `prerelease` is non-empty but not a valid SemVer2 prerelease
    label, since an invalid label would otherwise silently produce a
    `.nupkg` that NuGet.org rejects at push time instead of at build time.
    """
    if not prerelease:
        return version
    if not is_valid_prerelease_label(prerelease):
        fail(("nuget prerelease label %r is invalid: NuGet SemVer2 prerelease " +
              "labels must be alphanumeric characters and hyphens only.") % prerelease)
    return version + "-" + prerelease
