# Changelog

All notable changes to Arena will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.1.0]

### Added

- Unmanaged playbook support in arena-pytest, arena-junit, arena-xunit
- `--bump major/minor/patch` flag for the version bump script

### Changed

- arena-xunit `ManagedPlaybook.Run()` is now overridable

### Fixed

- arena-junit `ActivePlaybook` no longer rejects a zero/null handle
