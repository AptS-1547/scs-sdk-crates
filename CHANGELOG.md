# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added complete raw and typed coverage of the public SCS SDK 1.16 Telemetry
  interface. The vendored source of truth moves from `third-party/scs_sdk_1_14/`
  to `third-party/scs_sdk_1_16/`; the SDK license text is byte-identical, so the
  preserved 2016 notice now covers SDK 1.6 through 1.16.
- Added ETS2 telemetry schemas 1.19, 1.20, and 1.21 and ATS schemas 1.06, 1.07,
  and 1.08; `CURRENT` is now ETS2 1.21 and ATS 1.08.
- Added the `mandatory.break` common channel (`Channel<i32>`). Its availability
  records the introduction schemas, ETS2 1.19 and ATS 1.06; SCS documents that
  the channel was only delivered from ETS2 1.21 and ATS 1.08.
- Added the ETS2-only `bus_job` configuration (schema 1.20, no documented
  attributes yet) and the ATS-only `car_job` configuration (schema 1.07) with
  its `car_job.market` and three `customer.prio.*` attributes, 18 attribute
  associations, and the `CarJobMarket` known-value catalog.
- Added the ATS-only `car_job.cancelled` and `car_job.delivered` gameplay events
  (schema 1.07), the `vehicle.damage` gameplay attribute, and their payload
  associations.
- Added `ConfigurationEvent::car_job_market` to `scs-sdk-plugin`.
- Added a `scs-sdk-sys` header coverage test that parses the vendored SDK 1.16
  headers and requires every string and game-version macro to appear in the
  matching raw catalog in official header order. The test reads workspace-only
  files and is excluded from the published package.
- The telemetry example now also declares ATS (schema 1.01), subscribes to
  `mandatory.break` as an optional channel, and logs car-job and bus-job
  configurations and car-job gameplay events.
  It logs every attribute SCS omits as `n/a` instead of a type default; ATS
  1.62.2 omits `earned.xp` from `car_job.delivered`.
- Validated the Windows example in ETS2 1.62.2 (telemetry schema 1.22, newer
  than the SDK 1.16 headers) and ATS 1.62.2 (schema 1.08): `mandatory.break`
  values, the `car_job` configuration with `car_job.market=dispatch_job`,
  `car_job.delivered` with `vehicle.damage`, tollgate, fine, and ferry events,
  and clean shutdown.

### Changed

- **Breaking:** the public catalog arrays grow with SDK 1.16. Their array types
  change accordingly: 108 channels, 8 configuration IDs, 64 configuration
  attributes, 89 configuration associations, 8 gameplay events, 16 gameplay
  attributes, and 28 gameplay associations. Raw `scs-sdk-sys` catalogs change
  in the same way.
- Configuration, gameplay, and association availability may now be absent for
  one game: `bus_job` exists only in ETS2, and the car-job descriptors exist
  only in ATS.

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
