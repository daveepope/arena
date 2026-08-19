load(":defs.bzl", "maven_smoke_check")

exports_files(["smoke_test_maven.sh.tpl"])

maven_smoke_check(
    name = "maven_smoke_test",
    artifact_id = "arena-junit",
    group_id = "io.github.stationdevx",
    jar = "//arena-junit:arena_junit_maven-lib",
    native_classifier = "linux-x86_64",
    native_classifier_jar = "//arena-junit:arena_junit_native_linux_x86_64",
    pom = "//arena-junit:arena_junit_maven-pom",
    tags = ["manual"],
)

maven_smoke_check(
    name = "maven_snapshot_smoke_test",
    artifact_id = "arena-junit",
    group_id = "io.github.stationdevx",
    jar = "//arena-junit:arena_junit_maven_snapshot-lib",
    native_classifier = "linux-x86_64",
    native_classifier_jar = "//arena-junit:arena_junit_native_linux_x86_64",
    pom = "//arena-junit:arena_junit_maven_snapshot-pom",
    tags = ["manual"],
)

maven_smoke_check(
    name = "maven_snapshot_smoke_test_osx_aarch64",
    artifact_id = "arena-junit",
    group_id = "io.github.stationdevx",
    jar = "//arena-junit:arena_junit_maven_snapshot-lib",
    native_classifier = "osx-aarch_64",
    native_classifier_jar = "//arena-junit:arena_junit_native_osx_aarch64",
    pom = "//arena-junit:arena_junit_maven_snapshot-pom",
    tags = ["manual"],
)

maven_smoke_check(
    name = "maven_snapshot_smoke_test_osx_x86_64",
    artifact_id = "arena-junit",
    group_id = "io.github.stationdevx",
    jar = "//arena-junit:arena_junit_maven_snapshot-lib",
    native_classifier = "osx-x86_64",
    native_classifier_jar = "//arena-junit:arena_junit_native_osx_x86_64",
    pom = "//arena-junit:arena_junit_maven_snapshot-pom",
    tags = ["manual"],
)
