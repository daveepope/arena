"""Single source of truth for NuGet dependency versions shared between
`MODULE.bazel`'s `nuget_archive` pins and the `.nuspec` `<dependencies>`
metadata for packages built by `csharp_nuget_package` - so a version bump in
one place can't silently drift from the other.
"""

NEWTONSOFT_JSON_VERSION = "13.0.4"
MS_LOGGING_ABSTRACTIONS_VERSION = "8.0.2"
