# scs-sdk Instructions

This file supplements the repository-level AGENTS.md for the scs-sdk crate.
This crate is the safe, typed, no_std interpretation of scs-sdk-sys.

## Ownership

- Own typed SDK result codes, Telemetry and Input versions, values, geometry,
  descriptors, catalogs, callback-time borrowed views, scoped logging, and
  scoped registration calls.
- Do not own process-global state, exported symbols, plugin trait objects,
  registration transactions, callback context allocations, panic containment, or
  product behavior. Those belong in scs-sdk-plugin or the application.
- Keep the crate no_std. Do not add std or alloc merely for convenient owned
  strings or collections; application ownership conversion belongs above this
  layer.
- Re-export scs-sdk-sys only as the explicit raw escape hatch. New normal usage
  should go through typed APIs rather than growing dependence on sys details.

## Safe Wrapper Contract

- A public safe function must validate every condition needed before reading a
  pointer or union member. Unknown or mismatched value tags return an error or
  None without touching inactive storage.
- Borrowed callback views must not outlive the SDK callback scope. Tie lifetimes
  to input references and do not create 'static references from foreign pointers.
- `SdkCall`, `InputCall`, and `InputInitCall` must remain scoped, non-storable,
  non-Send, and non-Sync. SDK calls are allowed only while SCS directly invokes
  init, callback, or shutdown on its main thread. Device registration is exposed
  only through `InputInitCall` because the official Input API limits it to init.
- Keep Rust-owned geometry values free of ABI padding. Copy meaningful fields
  individually rather than copying bytes whose initialization is not guaranteed.
- NamedValues iteration must honor the SDK sentinel contract, validate each value
  independently, and never search beyond the foreign array terminator.
- ScopedLogger must build a valid temporary C string for the duration of the SDK
  call and must not expose CStr or CString to application code.
- Preserve every SDK result code as a distinct SdkError variant. Do not collapse
  errors merely to simplify callers.
- Validate `InputIndex` against the official 400-input limit and validate the
  event value type before writing the corresponding raw union member. Unknown
  raw input value types remain unknown rather than being interpreted as float.
- Keep safe float input events normalized through `InputAxisValue`: accept only
  finite values in the inclusive -1.0 through 1.0 interval, reject rather than
  clamp invalid values, and leave arbitrary raw floats to the sys escape hatch.

## Typed Catalog Design

- Channel<T> and Attribute<T> encode the expected SDK value representation.
  AnyChannel and AnyAttribute preserve enough metadata for enumeration,
  diagnostics, and coverage testing.
- Do not replace typed descriptors with plain strings or a map of untyped values.
- Keep scalar, SDK-indexed, and trailer-related metadata explicit. Registration
  convenience must not erase which index domain a channel uses.
- Represent explicit SDK array slots with `SdkIndex`, numbered trailer names
  with `TrailerIndex`, and trailer configuration callback identity with
  `TrailerConfigurationId`. Public wrapper APIs must not accept bare integers
  for those domains. Preserve legacy `trailer` separately from `trailer.0`.
- Channel::requesting<U>() is an explicit request for SDK conversion; do not infer
  conversions from callback code or silently reinterpret returned tags.
- Raw and typed ALL catalogs must agree exactly on names, counts, order, value
  types, and index behavior. Add paired tests for every catalog change.
- Version types must preserve unknown future raw values for diagnostics while
  leaving compatibility policy to the caller/runtime.
- Event is the sole typed event-identifier catalog. Keep its raw discriminator
  and official minimum-API metadata together; higher layers may re-export the
  type but must not mirror its variants in another registration enum.
- ValueType owns official representation-level availability metadata. Signed
  64-bit values require Telemetry API 1.01; do not confuse that API-level fact
  with SCS's separate per-channel conversion decision.
- Project every raw ETS2 and ATS schema-history constant through the typed
  game::* modules. Application compatibility declarations should use those
  named constants instead of repeating packed version numbers when the official
  header already names the version.
- Keep `GameSchemaAvailability` on every built-in channel, configuration, and
  gameplay descriptor. ETS2 and ATS minima must come from official historical
  headers; `None` is reserved for an explicit game-header exclusion rather than
  missing research.
- Keep configuration/event membership in the association catalogs rather than
  embedding one owner in `Attribute<T>`. Shared attributes may have different
  relationship minima, and numbered trailer relationships must compose with
  the canonical `game::capabilities::MULTI_TRAILER` namespace boundary.
- Preserve value-level schema history when an enum-like string catalog evolves
  without adding a new descriptor. `FineOffence::availability` is the current
  example and must not be flattened to the original gameplay-event version.
- Every closed SDK string catalog must expose `ALL`, `COUNT`, `as_str`,
  `FromStr`, and per-value schema availability. Parsing failure stays compact
  through `UnknownStringValue`; callers retain the original text rather than
  forcing a future value into a known variant.

## Unsafe Boundary

- Unsafe should be confined to raw-to-typed construction, validated union reads,
  and calls through SDK function pointers.
- Each unsafe block needs a local SAFETY explanation covering the exact preceding
  validation and the lifetime of the foreign storage.
- Do not make a constructor safe because its current caller happens to validate
  inputs. Encode the validation inside the constructor or retain an unsafe API
  with a complete contract.
- Changes to pointer provenance, padding handling, union reads, sentinel
  iteration, or SdkCall lifetimes require Miri tests, including strict provenance
  where relevant.

## Documentation

- Document units, coordinate conventions, value representations, index semantics,
  supported callback phase, and failure behavior on public APIs.
- Explain why a lifetime or marker prevents misuse instead of merely saying that
  it is safe.
- Keep the complete catalog inventory and version terminology synchronized with
  both READMEs and scs-sdk-sys.

## Validation

After changes, run at minimum:

    cargo fmt --all -- --check
    cargo test --locked -p scs-sdk
    cargo clippy --locked -p scs-sdk --all-targets -- -D warnings
    RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk --no-deps
    cargo +nightly-2026-04-12 miri test --locked -p scs-sdk

For any public API, descriptor, registration, version, or value change, also run:

    cargo test --locked -p scs-sdk-plugin
    scripts/check-plugin-boundary.sh

This confirms that the framework can still expose the wrapper without pushing raw
types or unsafe responsibilities into application code.
