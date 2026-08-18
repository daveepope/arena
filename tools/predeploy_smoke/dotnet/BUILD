load(":defs.bzl", "dotnet_nuget_smoke_check")

exports_files(["smoke_test_nuget.sh.tpl"])

dotnet_nuget_smoke_check(
    name = "nuget_smoke_test",
    package = "//arena-xunit:arena_xunit_nuget",
    package_id = "ArenaDotnet.Xunit",
    tags = ["manual"],
)

dotnet_nuget_smoke_check(
    name = "nuget_snapshot_smoke_test",
    package = "//arena-xunit:arena_xunit_nuget_snapshot",
    package_id = "ArenaDotnet.Xunit",
    tags = ["manual"],
)
