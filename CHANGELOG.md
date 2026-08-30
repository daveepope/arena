# Changelog

All notable changes to Arena will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [6.2.0]

### Added

- `arena-oauth`: multi-issuer support on `OauthDependency` via `with_issuer`/`with_provider` (#218)
- `arena-oauth`: `Provider` presets for Cognito, Okta, and Entra ID (#218)
- `arena-oauth`: `sign_claims`/`signing_key_pem`/`issuer_at`/`issuer_count` on `OauthDependency` (#218)
- `arena-ffi`: `arena_oauth_sign_claims` (#218)
- `arena-pytest`, `arena-junit`, `arena-xunit`: `with_issuer_cognito`/`with_issuer_okta`/`with_issuer_entra_id`/`with_issuer`/`sign_claims` bindings (#218)
- Example apps (Rust, Spring Boot, ASP.NET): Cognito-shaped OAuth provider in test fixtures (#218)
- `arena-junit`: `OauthSigner`/`@ArenaOauthSigner` for injecting a per-test OAuth signer (#218)
- `arena-xunit`: `ArenaCollectionFixture.Signer`/`GetDependency<T>()` for injecting a per-fixture OAuth signer (#218)
- `arena-pytest`: `oauth_signer_fixture` for wiring an `OauthSigner` pytest fixture (#218)
- `arena-host`: new crate providing `find_available_port`/`PortSearchStrategy` for zero-dependency, process-safe free TCP port discovery (#202)
- `arena-ffi`: `arena_find_available_port` (#202)
- `arena-pytest`, `arena-junit`, `arena-xunit`: `find_available_port`/`ArenaHost` bindings, `PortSearchStrategy`, `ArenaPortNotFoundError`/`ArenaPortNotFoundException` (#202)
- `--config=stream` for streamed test output with a detailed summary
- mold linker for Rust targets on Linux, installed by CI and required for local builds

### Changed

- `bazel test` now defaults to `--test_output=errors`; use `--config=stream` for streamed output

### Fixed

- `arena-junit`: `ArenaExtension` now makes the arena queryable during `@ArenaAfterOpen`, not just after
- `arena-oracledb`: SQL readiness fails immediately when the container has stopped or been removed, instead of retrying it for the full timeout
- FastAPI example component tests: dependency containers get run-unique names, so parallel targets no longer force-remove each other's containers
- Example test runtimes (pytest, junit, xunit): ephemeral port ranges partitioned per test target to stop parallel targets drawing colliding host ports (#220)

## [6.1.0]

### Added

- `audit_and_vet_rust` now fails if the count of cargo-vet audited or exempted packages changes, to catch silent supply-chain trust regressions (or acknowledge improvements)
- arena-junit: Oracle dependency and playbook support (`OracleDependencyBuilder`, `ManagedOraclePlaybook`)
- `arena-xunit`: Oracle dependency and playbook support (`OracleDependencyBuilder`, `ManagedOraclePlaybook`)
- `arena-pytest`: Oracle dependency and playbook support (`OracleDependencyBuilder`, `ManagedOraclePlaybook`)
- FastAPI example app: Oracle-backed weather report endpoints (`POST/GET /weather`), wired into both pytest component test suites
- Spring Boot and ASP.NET example apps: Oracle-backed weather report endpoints, wired into both junit and xunit component/chained-component test suites
- Oracle dependency builders (`arena-oracledb`, `arena-junit`, `arena-xunit`, `arena-pytest`) require an explicit `.full_build()` opt-in for a non-default database name, and scale the SQL readiness timeout accordingly, since a custom name forces a slow from-scratch pluggable database build
- CI: betterleaks secret scan on every PR, push to master, and daily cron

### Fixed

- `fromImage` now resolves registry credentials (`credHelpers`, `credsStore`, `auths`, honoring `DOCKER_CONFIG`) instead of pulling anonymously (#210)
- `fromImage` skips pulling when a locally cached image already matches the requested platform (#210)
- Image pull/build failures now return a typed error instead of panicking (#210)
- Registry credential resolution no longer blocks the async runtime and no longer silently swallows credential-helper failures (#210)
- Example Postgres/MSSQL test credentials are now randomly generated instead of hardcoded, matching the existing Oracle credential pattern

### Changed

- PR pipeline no longer auto-publishes preview builds (TestPyPI, Maven snapshot, NuGet snapshot) on every push; apply the `pre-release` label to opt in

## [6.0.2]

### Security

- Bumped `h2` to 0.4.16, fixing RUSTSEC-2026-0258
- Bumped `jackson-databind`, `grpc-core`/`grpc-netty-shaded`, `netty-codec`/`-http`/`-http2`, and `log4j-api` in the example Spring Boot app (`pom.xml` and `MODULE.bazel`) to patched versions
- Pinned `arena_java_maven` to a checksum-verified lock file (`arena_java_maven_install.json`) instead of unverified live resolution
- Enabled hash verification (`generate_hashes = True`) for `arena-pytest` and `examples` pip lockfiles
- Added floor constraints (`idna>=3.15`, `pygments>=2.20.0`) to `arena-pytest`/`examples` `requirements.txt` so OSV-Scanner's independent resolution of the unpinned manifest also picks up patched versions
- Documented OSV-Scanner exceptions (`osv-scanner.toml`) for `rsa`, `rustls-pemfile`, and `rustls-webpki` advisories with no reachable fix through the `tiberius` dependency chain
- Extended OSV-Scanner CI coverage across all 4 dependency ecosystems (Cargo, pip, Maven, NuGet) by adding generated CycloneDX SBOMs for `arena-ffi` (Rust), `arena-junit` (Maven), and `arena-xunit` (NuGet); pip was already natively scanned via `requirements_lock.txt`. Closes a gap where the actual published Maven/NuGet dependencies (as opposed to the example apps) had no CVE scanning at all
- Bumped `kafka-clients` to 3.9.2, migrated `org.lz4:lz4-java` to its relocated `at.yawk.lz4:lz4-java` coordinates at 1.11.1, and bumped `micrometer-core` to 1.15.12, fixing CVEs surfaced by the new Maven SBOM scan
- Closed a fail-open gap in `check_dependency_release_age.py` where Maven/BCR/NuGet dependencies silently passed the 3-day release-age check if their publish time couldn't be determined; also replaced the unreliable Maven Central search-index and empty BCR-timestamp lookups with direct repository/commit-history queries
- Bumped the pinned `google/osv-scanner-action` from a May 2026 commit to v2.5.1, picking up native `.csproj` (NuGet) scanning support that the old pin lacked

### Added

- `bazel run //scripts:repin` to force-repin Rust, Python, and Maven lockfiles independent of a version bump
- `scripts/generate_rust_sbom.py`, `generate_maven_sbom.py`, `generate_nuget_sbom.py` to produce CycloneDX SBOMs for OSV-Scanner from `Cargo.Bazel.lock`, `arena_java_maven_install.json`, and `MODULE.bazel`
- `cargo audit bin` and `cargo vet` against a `cargo-auditable`-built `arena_ffi_shared` binary, run as part of `bazel run //scripts:repin` and as a standalone, Rust-path-filtered CI job (`supply-chain/` cargo-vet config, bootstrapped with exemptions for all currently-used crates)
- Added `chrono` as a direct workspace dependency (with the `clock` feature) so `arena-kafka`/`examples` no longer rely on `rskafka`'s re-exported `chrono` having that feature enabled only by incidental cross-crate feature unification; this is what made a standalone `cargo build -p arena-ffi` possible in the first place

### Fixed

- Example Axum web app now retries the Postgres connect and OAuth JWKS fetch on startup instead of panicking on the first attempt, fixing flaky example component tests under CI load

### Changed

- Bumped `rustls` to 0.23.43

## [6.0.1]

### Fixed

- `arena-xunit` native library resolution now prefers a co-located `libarena_ffi_shared` over `.deps.json`/`AssemblyDependencyResolver`, fixing wrong-RID resolution on Apple Silicon

### Added

- NuGet predeploy smoke test now runs via a real xUnit test project (`dotnet test`) against the real global NuGet cache, not an isolated one
- New Bazel-consumer predeploy smoke test (`//tools/predeploy_smoke/dotnet:bazel_consumer_smoke_test`) covers the `rules_dotnet csharp_import` + `bazel test` consumption path

## [6.0.0]

### Added

- `arena-xunit` API parity with `arena-pytest`/`arena-junit`: `WithImageName`/`WithImage`/`WithContainerName` on Kafka, Mssql, Postgres, and Smtp dependencies; `WithLambda` on Localstack; `WithHttp()`/`transport` and default-issuer fallback on Oauth; `WithBuildToolCustom` on `ExecutableComponentBuilder`

### Fixed

- `OauthDependencyBuilder` in `arena-xunit` defaulted to port 9443 instead of 9444, matching `arena-pytest`/`arena-junit`
- `KafkaDependencyBuilder.Build()` in `arena-xunit` aliased its internal topic list instead of copying it, so mutating the builder after `Build()` mutated the already-built dependency
- `LocalstackDependencyBuilder.WithLambda` source directory now expands a leading `~` before resolving to an absolute path

### Removed

- `LocalstackDependencyBuilder.WithImage(string)` in `arena-xunit`: sent a wire key the Rust FFI never read, making it a silent no-op; replaced with `WithImageName`/`WithImageTag`

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
