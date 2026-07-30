# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [v0.1.1] - 2026-07-30

### Changed

- Strengthened audited FFI boundaries with narrower unsafe scopes and concrete
  safety documentation for callbacks, pointer projections, tagged unions, and
  input event writes.
- Added workspace Clippy enforcement for reasoned allow attributes, documented
  unsafe blocks, and one unsafe operation per block.
- Updated the proc-macro implementation to `syn` 3.0.3 and synchronized all four
  public crates at version 0.1.1.

## [v0.1.0] - 2026-07-26

### Added

- Added complete raw and typed coverage of the public SCS SDK 1.14 Telemetry
  interface and the independently versioned Input API 1.00 interface.
- Added safe Telemetry and Input plugin runtimes with explicit registration,
  rollback or conservative retirement, panic containment, callback-context
  ownership, shutdown, and stale-generation isolation.
- Added `export_plugin!` and `export_input_plugin!` proc macros, including
  independent consumer fixtures and a combined four-export fixture.
- Added safe Telemetry, Generic Input, Semantical Input, and Telemetry fallback
  examples for real ETS2 boundary validation.
- Added typed schema history, compatibility metadata, distinct index and version
  domains, lossless game identity, and complete descriptor catalogs.
- Added x86-64 Windows, Linux with a glibc 2.17 floor, and macOS artifact builds
  with native format, architecture, signature where applicable, and exact export
  verification.
- Added workspace tests, strict Clippy and rustdoc gates, Miri strict-provenance
  validation, package audits, dual MIT or Apache-2.0 licensing, preserved SCS SDK
  notices, and automated crates.io and GitHub Release publication.

[Unreleased]: https://github.com/AptS-1547/scs-sdk-crates/compare/v0.1.1...HEAD
[v0.1.1]: https://github.com/AptS-1547/scs-sdk-crates/compare/v0.1.0...v0.1.1
[v0.1.0]: https://github.com/AptS-1547/scs-sdk-crates/releases/tag/v0.1.0
