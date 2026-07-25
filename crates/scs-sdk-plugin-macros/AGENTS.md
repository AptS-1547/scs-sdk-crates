# scs-sdk-plugin-macros Instructions

This file supplements the repository-level AGENTS.md for the proc-macro crate.
The macro is small because its responsibility is intentionally narrow.

## Ownership

- Own syntax parsing and generated code for `export_plugin!` and
  `export_input_plugin!` only.
- Generate independent process-lifetime `Runtime` or `InputRuntime` storage and
  the corresponding fixed loader entry points: `scs_telemetry_init` plus
  `scs_telemetry_shutdown`, or `scs_input_init` plus `scs_input_shutdown`.
- Do not implement SDK version parsing, pointer validation, lifecycle policy,
  subscription behavior, callback dispatch, rollback, or product logic here.
  Delegate those mechanisms to scs-sdk-plugin Runtime.
- Do not add a dependency from this crate back to scs-sdk-plugin; that would form
  a Cargo dependency cycle. Generated paths resolve through the consumer's direct
  scs-sdk-plugin dependency.

## Expansion Contract

- Parse the macro input as one normal Rust expression. It is evaluated exactly
  once for each accepted initialization attempt.
- The generated factory must coerce the expression result to the corresponding
  framework `TelemetryPlugin` or `InputPlugin` trait object so a missing
  implementation fails at compile time.
- Each macro generates exactly one stable runtime static and exactly two fixed
  exports. Do not add aliases, platform-specific alternate names, or hidden
  extra loader symbols. One invocation of each different macro may coexist and
  must produce exactly four exports without static-name collisions.
- Preserve extern "system", public visibility, no_mangle export behavior, raw
  parameter types, return type, and safety documentation required by the SCS ABI.
- Application source remains safe. All unsafe tokens needed by the ABI must come
  from the expansion and call the audited runtime boundary.
- Generated paths must be absolute and hygienic. Do not assume imports, local type
  aliases, or renamed transitive dependencies in the consumer crate.
- Keep generated item names sufficiently private and specific to avoid collisions
  with ordinary application identifiers. Multiple invocations in one cdylib are
  unsupported because the SCS symbols are unique; retain clear documentation.
- Keep generated documentation accurate for both ETS2 and ATS and for each
  runtime's actual shutdown behavior, including the Input API's lack of an
  explicit unregister function.

## Testing Contract

- An ignored doctest is documentation only. It does not replace the independent
  consumer fixture owned under scs-sdk-plugin/tests/fixtures/export-plugin/.
- Telemetry and Input pass fixtures must use only the public scs-sdk-plugin
  dependency, implement the corresponding trait in safe source, expand the
  public macro, and build a real cdylib. The combined pass fixture invokes both.
- Each missing-trait fixture must reach the generated trait-object coercion and
  fail with E0277 for the corresponding trait. Path, parser, or dependency
  failures are not an acceptable negative test result.
- Windows PE, Linux ELF, and macOS Mach-O release fixtures must expose the exact
  two-symbol set for each individual fixture and the exact four-symbol set for
  the combined fixture after LTO and stripping.
- When changing generated tokens, inspect expanded behavior through fixtures and
  final symbol tables rather than relying only on unit parsing tests.

## Style and Dependencies

- Keep dependencies limited to proc-macro tooling that is needed for parsing and
  quoting. Do not pull runtime functionality into the proc-macro process.
- Return structured syn diagnostics for invalid input where practical. Do not
  introduce procedural-macro panics for ordinary user mistakes.
- Comment generated unsafe and storage invariants extensively because consumers
  debug the expansion indirectly through compiler errors and loader logs.
- Keep the MSRV compatible with the workspace toolchain and generated syntax.

## Validation

After changes, run at minimum:

    cargo fmt --all -- --check
    cargo test --locked -p scs-sdk-plugin-macros
    cargo clippy --locked -p scs-sdk-plugin-macros --all-targets -- -D warnings
    RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk-plugin-macros --no-deps
    scripts/check-plugin-macro-fixtures.sh
    scripts/check-plugin-boundary.sh

For any expansion or export change, also run the Windows, Linux, and macOS fixture
builds and their symbol-verification scripts.
