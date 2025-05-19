# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/zetanumbers/scope-lock/compare/v0.3.0...v0.3.1) - 2025-05-19

### Fixed

- reimplement extended reference tracking using std

### Other

- allow documentation hidding for a minor update
- hide leftover legacy code
- Revert "docs: deprecate and hide docs on leftover legacy code"
- restore Unpin impls in leftover legacy code
- relax MIRI restrictions, add tree borrows and run 8 random seeds
- deprecate and hide docs on leftover legacy code

## [0.3.0](https://github.com/zetanumbers/scope-lock/compare/v0.2.5...v0.3.0) - 2025-05-15

### Added

- [**breaking**] carry the return value of the `lock_scope` closure

### Other

- Separate MIRI setup stage
- temporary disable MIRI preemptive scheduling
- remove rust-toolchain.toml
- [**breaking**] remove deprecated methods
- disable tree borrows as parking_lot does not support strict provenance
- fix miri CI
- add flake dev shell and update rust version

## [0.2.5](https://github.com/zetanumbers/scope-lock/compare/v0.2.4...v0.2.5) - 2024-06-02

### Added
- add new Extender methods which don't use dynamic dispatch
- add unchecked versions of extend functions
- pointer_like traits added

### Fixed
- drop reference guard after extended value is dropped

### Other
- remove old empty files
- disable stacked borrows on tree borrows check
- add test for a specific data race error on miri
- remove unused pub(crate)
- update parking_lot in Cargo.lock
- swap order of tree borrows and stack borrows tests
- set minimal supported rust version to 1.66
- revert usage of ptr::cast_const and cast_mut
- modularize code
- run miri tests with tree borrows model too
- move msrv and separate miri test into separate push workflow
- run minimal version check on any push

## [0.2.4](https://github.com/zetanumbers/scope-lock/compare/v0.2.3...v0.2.4) - 2024-05-29

### Fixed
- `Extender::extend_fn_once` use after free

## [0.2.3](https://github.com/zetanumbers/scope-lock/compare/v0.2.2...v0.2.3) - 2024-05-28

### Added
- add futures support ([#2](https://github.com/zetanumbers/scope-lock/pull/2))

### Fixed
- wait for extended objects before invalidating reference to extender

## [0.2.2](https://github.com/zetanumbers/scope-lock/compare/v0.2.1...v0.2.2) - 2024-05-28

### Fixed
- fix double drop on RefOnce::into_inner

### Other
- check minimal supported rust version
- miri test on release-plz workflow
- add release-plz gh action
- Separate doc examples into examples/ folder
- Remove unused mut in the doc test
