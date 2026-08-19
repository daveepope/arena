# Changelog

All notable changes to Arena will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [5.4.1]

### Fixed

- `arena-xunit` NuGet package placed native libraries under `lib/<tfm>/` instead of `runtimes/<rid>/native/`, so they were never deployed to consumers; `ArenaPaths` also gained an `AssemblyDependencyResolver` fallback for the common case where no `RuntimeIdentifier` is set

### Added

- Predeploy smoke test gates (`tools/predeploy_smoke/`) for arena-xunit, arena-junit, and arena-pytest: each publish job now installs the real built artifact via its real package manager and opens/closes an arena before publishing, with cross-platform execution coverage on macOS and a fast hermetic packaging-layout test covering all RIDs/classifiers including Windows

## [5.4.0]

### Added

- `ContainerizedComponentBuilder.fromImage()` to run an already-published registry image instead of building from a Containerfile, `withPlatform()` to override the container platform, and validation that rejects `withImageTag()`/`withBuildContext()` combined with `fromImage()` (#198)

## [5.3.1]

### Added

- Developer Certificate of Origin (DCO) sign-off requirement and enforcement workflow

### Security

- Bumped `jackson-databind` to 2.22.2 in `arena-junit` (transitive resolution was picking 2.21.4, vulnerable to GHSA-5gvw-p9qm-jgwh, GHSA-5jmj-h7xm-6q6v, GHSA-mhm7-754m-9p8w)

## [5.3.0]

### Added

- Bind-mount (volume) support for `ContainerizedComponentBuilder` across Rust, `arena-pytest`, `arena-junit`, and `arena-xunit` (#185)

## [5.2.0]

### Added

- Windows (`x86_64-pc-windows-msvc`) support for `arena-ffi`, and native binary packaging for `arena-pytest` (PyPI), `arena-junit` (Maven), and `arena-xunit` (NuGet)

## [5.1.3]

### Changed

- Replaced `rdkafka` with `rskafka` in `arena-kafka` (pure Rust, no C toolchain required)

## [5.1.2]

### Added

- Additional unit test coverage across `arena-junit`

## [5.1.1]

### Added

- Additional unit test coverage across `arena`, `arena-http`, `arena-kafka`, `arena-postgres`, `arena-localstack`, `arena-oauth`, and `arena-mssql`

## [5.1.0]

### Added

- Dependency and component trees: `with_child_dependencies`/`with_child_components` (Rust: `add_child`) across Rust, Java, Python, and .NET clients
- ClusterFuzzLite continuous fuzzing (`fuzz/`, `.clusterfuzzlite/`)

### Fixed

- xunit ASP.NET example app: health check now waits for the Temporal worker to be actively polling before reporting ready, fixing flaky startup/logging in the xunit example tests
- Managed playbooks and `arena_soft_reset`/`arena_hard_reset` now resolve dependencies nested as children, not just top-level dependencies
- arena-ffi entry points no longer crash the host process on malformed JSON input; panics are now returned as errors
- arena-pytest: `ManagedPostgresPlaybook` is now exported from the package root
- Documented accepted `cargo audit` advisories for `rsa`/`rustls-webpki` transitive deps (no upstream fix available; low exploitability for Arena's ephemeral local connections)

## [5.0.0]

### Breaking

- Scoped `Managed*Playbook` activation now runs after the test body by default instead of before (Python, Java, .NET); custom Python `Playbook` subclasses must now extend `ManagedPlaybook` or `UnmanagedPlaybook`

### Added

- Unmanaged playbook support in arena-pytest, arena-junit, arena-xunit
- arena-junit: share one arena across multiple test classes via `@Suite`/`@SelectClasses` and `@Arena(value = ...)`
- `--bump major/minor/patch` flag for the version bump script

### Changed

- arena-xunit `ManagedPlaybook.Run()` is now overridable
- arena-pytest and arena-xunit published packages use the root `README.md` instead of per-package copies

### Fixed

- arena-junit `ActivePlaybook` no longer rejects a zero/null handle
- Bump Spring Boot 3.5.3 → 3.5.16, `jackson-databind` → 2.18.9, `logback-classic` → 1.5.38 (CVE fixes)

### Removed

- arena-pytest and arena-xunit per-package `README.md`/`DESCRIPTION.md` files

## [4.0.1]

### Fixed

- Bump `cryptography` 48.0.1 → 50.0.0 in examples (CVE fix)

## [4.0.0]

Not published to PyPI, Maven Central, or NuGet due to a GitHub Actions outage; superseded immediately by 4.0.1.

### Changed

- Reduced release build size; split the release build matrix per platform for arena-junit
- Release builds now run on every merge to master

### Fixed

- Rust builds on Intel Mac (missing `x86_64-apple-darwin` in `crate.from_cargo` `supported_platform_triples`)
- Maven Central publish pipeline

## [3.7.0]

### Added

- Scoped `@ArenaLogger`-style logging annotation for arena-junit and arena-xunit's `ArenaCollectionFixture`
- FFI test coverage for postgres and Temporal dependency wiring

### Fixed

- Dependency log forwarding gaps

## [3.6.0]

### Added

- `ManagedPostgresPlaybook` across arena-pytest, arena-junit, and arena-xunit, with FFI bindings and a Postgres playbook manifest

## [3.5.9]

### Fixed

- Release publish pipeline (CI workflow, `.bazelrc`, lockfile sync)

## [3.5.7]

### Fixed

- Native library resolution/packaging for arena-junit and arena-xunit (`ArenaBindings`, `ArenaNativeLib`, `ArenaPaths`)

## [3.5.6]

### Changed

- arena-junit, arena-pytest, and arena-xunit package description metadata (POM, `DESCRIPTION.md`, nuspec)

## [3.5.5]

### Fixed

- Renamed the arena-xunit NuGet assembly/package for publishing

## [3.5.4]

### Changed

- Dependency lockfile repin; CI workflow tweak

## [3.5.3]

### Added

- Publish arena-xunit to NuGet, including the `csharp_nuget_package` Bazel packaging toolchain (nuspec template, push script, pinned-version checks)

## [3.5.2]

### Changed

- Default container image CVE scan is now a label-gated GitHub Actions option instead of always running

## [3.5.1]

### Added

- TCP readiness check alongside the existing HTTP readiness check

## [3.5.0]

### Added

- STARTTLS support for arena-smtp (`with_starttls()` / `.withStarttls()`), backed by an ephemeral self-signed certificate shared via arena-container

## [3.4.0]

### Added

- arena-smtp mail-capture dependency crate, wired into arena-pytest (`SmtpDependencyBuilder`) and arena-junit (`SmtpDependency`)

## [3.3.0]

### Added

- Temporal dependency support in arena-pytest and arena-junit, with FFI bindings

## [3.2.1]

### Fixed

- Bump jackson-databind, postgresql, and mssql-jdbc to patched versions (CVE fixes)

## [3.2.0]

### Added

- `arena-temporal` dependency crate with a gRPC-based readiness check and pinned image tag

## [3.1.1]

### Changed

- Enabled BuildBuddy remote build caching for CI

## [3.1.0]

### Added

- Best-effort container image CVE search (`scripts/check_container_cves.py`) and bumped default image versions

## [3.0.1]

### Fixed

- Bump `serde_with` 3.18.0 → 3.21.0

## [3.0.0]

### Breaking

- arena-junit now targets Java 25

### Removed

- The unrelated `arbiter` crate and its tests

## [2.0.0]

### Breaking

- arena-junit examples move from a separate `ArenaFixture` class to declaring `@ArenaDependency`/`@ArenaComponent` directly on the test class (the `@Arena` annotation pattern)

## [1.2.4]

### Fixed

- Maven Central publish CI workflow

## [1.2.3]

### Fixed

- Maven Central publish script

## [1.2.2]

### Added

- Publish arena-junit to Maven Central, including the publish script and POM template

## [1.2.1]

### Added

- Dependency review workflow

### Changed

- Pinned default container image tags/versions with a security scan check

## [1.2.0]

### Added

- Container image default/version management scripts (`container_defaults.py`, image matrix, default-images codegen)

## [1.1.1]

### Fixed

- Bump `cmov` 0.5.3 → 0.5.4

## [1.1.0]

### Changed

- Simplified CI/CD: publish now triggers on version update instead of PR labels; removed the PR-label version-bump feature

## [1.0.0]

### Added

- First stable release
- `tools/version_sync.bzl` and related scripts to keep the workspace version in sync

### Changed

- CI now enforces a 3-day dependency release-age gate and locks down `MODULE.bazel.lock`

## [0.4.0-b1] (pre-release)

### Added

- Initial PyPI pre-release
