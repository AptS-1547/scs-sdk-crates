//! Strongly typed versions used by the telemetry stack.
//!
//! SCS exposes several independently versioned contracts. In particular, the
//! telemetry API version passed to `scs_telemetry_init` is not the same value
//! as the game-specific telemetry schema stored in the initialization
//! parameters. Keeping both as bare `u32` values makes it too easy to compare
//! unrelated contracts or report the wrong version in diagnostics.

use core::fmt;

use crate::sys;

/// Version of the ABI negotiated through `scs_telemetry_init`.
///
/// The game tries supported API versions from newest to oldest. A plugin must
/// accept only layouts it has explicitly audited and return `unsupported` for
/// every other value so the game can continue its normal fallback sequence.
///
/// This is intentionally a newtype rather than a closed enum. Future SCS
/// versions can be represented and logged before the wrapper has added a safe
/// adapter for their initialization structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TelemetryApiVersion(u32);

impl TelemetryApiVersion {
    /// Initial telemetry API layout.
    pub const V1_00: Self = Self(sys::SCS_TELEMETRY_VERSION_1_00);

    /// Adds signed 64-bit values and gameplay events.
    pub const V1_01: Self = Self(sys::SCS_TELEMETRY_VERSION_1_01);

    /// Latest telemetry API declared by the vendored SCS SDK headers.
    pub const CURRENT: Self = Self(sys::SCS_TELEMETRY_VERSION_CURRENT);

    /// Creates a packed SCS version from its two components.
    #[must_use]
    pub const fn new(major: u32, minor: u32) -> Self {
        Self(sys::make_version(major, minor))
    }

    /// Wraps the packed version supplied by the game without claiming support.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the packed representation used by the SCS ABI.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Major component. A change normally indicates an incompatible API.
    #[must_use]
    pub const fn major(self) -> u32 {
        sys::version_major(self.0)
    }

    /// Minor component. SCS normally uses this for additive API changes.
    #[must_use]
    pub const fn minor(self) -> u32 {
        sys::version_minor(self.0)
    }
}

impl fmt::Display for TelemetryApiVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major(), self.minor())
    }
}

/// Game-specific telemetry schema version from the initialization parameters.
///
/// This is separate from both [`TelemetryApiVersion`] and the public game patch
/// displayed to players. SCS treats a schema-major change as potentially
/// incompatible while schema-minor changes describe compatible additions or
/// semantic corrections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameSchemaVersion(u32);

impl GameSchemaVersion {
    /// Creates a packed game-schema version from its two components.
    #[must_use]
    pub const fn new(major: u32, minor: u32) -> Self {
        Self(sys::make_version(major, minor))
    }

    /// Wraps the packed version supplied in SCS initialization parameters.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the packed representation used by the SCS ABI.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Major component used to identify incompatible schema generations.
    #[must_use]
    pub const fn major(self) -> u32 {
        sys::version_major(self.0)
    }

    /// Minor component used for compatible schema evolution.
    #[must_use]
    pub const fn minor(self) -> u32 {
        sys::version_minor(self.0)
    }
}

impl fmt::Display for GameSchemaVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major(), self.minor())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::string::ToString;

    #[test]
    fn telemetry_api_version_preserves_unknown_values_for_negotiation() {
        let future = TelemetryApiVersion::new(2, 7);

        assert_eq!(future.major(), 2);
        assert_eq!(future.minor(), 7);
        assert_eq!(TelemetryApiVersion::from_raw(future.raw()), future);
        assert_eq!(future.to_string(), "2.7");
    }

    #[test]
    fn game_schema_version_is_not_interchangeable_with_api_version() {
        let schema = GameSchemaVersion::new(1, 19);

        assert_eq!(schema.major(), 1);
        assert_eq!(schema.minor(), 19);
        assert_eq!(GameSchemaVersion::from_raw(schema.raw()), schema);
        assert_eq!(schema.to_string(), "1.19");
    }
}
