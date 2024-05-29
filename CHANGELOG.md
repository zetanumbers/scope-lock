# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
