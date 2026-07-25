//! Typed game-specific telemetry schema versions.
//!
//! SCS publishes a separate schema history for each game. Equal numeric values
//! across ETS2 and ATS do not describe the same release or feature set, so the
//! constants remain in distinct modules while sharing [`GameSchemaVersion`] as
//! their strong representation. Every value projects directly from the raw
//! header constant; this module does not maintain another numeric inventory.

use crate::GameSchemaVersion;

/// Cross-cutting game-schema capabilities shared by several SDK catalogs.
///
/// These are kept outside channel, configuration, and gameplay modules because
/// one game feature can introduce names in all three catalogs at once. A
/// numbered trailer configuration and a numbered trailer channel, for example,
/// must use the same official schema boundary rather than maintain two copies
/// which can drift independently.
pub mod capabilities {
    use crate::{GameSchemaAvailability, game};

    /// First schemas which expose the numbered `trailer.[index]` namespace.
    ///
    /// Static `trailer` configuration and `trailer.*` channel names predate
    /// this feature. Code which constructs a numbered name must combine the
    /// descriptor or association availability with this namespace capability.
    pub const MULTI_TRAILER: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_14), Some(game::ats::V1_01));

    /// First schemas which emit gameplay-event descriptors and payloads.
    ///
    /// Gameplay also requires Telemetry API 1.01. That independent ABI-level
    /// requirement remains represented by `Event::minimum_api_version` and
    /// signed-value metadata rather than being folded into this schema value.
    pub const GAMEPLAY_EVENTS: GameSchemaAvailability = MULTI_TRAILER;
}

/// Euro Truck Simulator 2 telemetry schema history.
pub mod ets2 {
    use super::GameSchemaVersion;
    use crate::sys;

    /// Initial ETS2 telemetry schema.
    pub const V1_00: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_00);
    /// Added emergency brake-air pressure information.
    pub const V1_01: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_01);
    /// Replaced cabin orientation with cabin offset.
    pub const V1_02: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_02);
    /// Corrected the invalid index used by the wheel-count attribute.
    pub const V1_03: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_03);
    /// Added physical left- and right-blinker light channels.
    pub const V1_04: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_04);
    /// Corrected truck brand identifiers and localized names.
    pub const V1_05: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_05);
    /// Corrected the selector-count index while retaining a compatibility copy.
    pub const V1_06: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_06);
    /// Corrected cabin angular acceleration.
    pub const V1_07: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_07);
    /// Added empty truck and trailer configurations when a vehicle is removed.
    pub const V1_08: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_08);
    /// Added game-time and job-related telemetry.
    pub const V1_09: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_09);
    /// Added liftable axle configuration information.
    pub const V1_10: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_10);
    /// Added `u64` conversion for `u32` channels and displayed gear.
    pub const V1_11: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_11);
    /// Added transmission ratios, navigation, and `AdBlue` telemetry.
    pub const V1_12: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_12);
    /// Corrected trailer and cargo-accessory identifiers.
    pub const V1_13: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_13);
    /// Added multiple trailers, trailer ownership, and gameplay events.
    pub const V1_14: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_14);
    /// Added planned job distance.
    pub const V1_15: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_15);
    /// Added additional fine-offence identifiers.
    pub const V1_16: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_16);
    /// Added differential lock, lift axle, and hazard-warning channels.
    pub const V1_17: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_17);
    /// Added multiplayer time offset and corrected trailer wear channels.
    pub const V1_18: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_1_18);

    /// Latest ETS2 schema declared by the vendored SDK 1.14 headers.
    pub const CURRENT: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ets2::VERSION_CURRENT);
}

/// American Truck Simulator telemetry schema history.
pub mod ats {
    use super::GameSchemaVersion;
    use crate::sys;

    /// Initial ATS schema, corresponding to ETS2 telemetry schema 1.12.
    pub const V1_00: GameSchemaVersion = GameSchemaVersion::from_raw(sys::games::ats::VERSION_1_00);
    /// Added multiple trailers, trailer ownership, and gameplay events.
    pub const V1_01: GameSchemaVersion = GameSchemaVersion::from_raw(sys::games::ats::VERSION_1_01);
    /// Added planned job distance.
    pub const V1_02: GameSchemaVersion = GameSchemaVersion::from_raw(sys::games::ats::VERSION_1_02);
    /// Added additional fine-offence identifiers.
    pub const V1_03: GameSchemaVersion = GameSchemaVersion::from_raw(sys::games::ats::VERSION_1_03);
    /// Added differential lock, lift axle, and hazard-warning channels.
    pub const V1_04: GameSchemaVersion = GameSchemaVersion::from_raw(sys::games::ats::VERSION_1_04);
    /// Added multiplayer time offset and corrected trailer wear channels.
    pub const V1_05: GameSchemaVersion = GameSchemaVersion::from_raw(sys::games::ats::VERSION_1_05);

    /// Latest ATS schema declared by the vendored SDK 1.14 headers.
    pub const CURRENT: GameSchemaVersion =
        GameSchemaVersion::from_raw(sys::games::ats::VERSION_CURRENT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_game_versions_project_every_raw_header_constant() {
        assert_eq!(ets2::V1_00.raw(), crate::sys::games::ets2::VERSION_1_00);
        assert_eq!(ets2::V1_01.raw(), crate::sys::games::ets2::VERSION_1_01);
        assert_eq!(ets2::V1_02.raw(), crate::sys::games::ets2::VERSION_1_02);
        assert_eq!(ets2::V1_03.raw(), crate::sys::games::ets2::VERSION_1_03);
        assert_eq!(ets2::V1_04.raw(), crate::sys::games::ets2::VERSION_1_04);
        assert_eq!(ets2::V1_05.raw(), crate::sys::games::ets2::VERSION_1_05);
        assert_eq!(ets2::V1_06.raw(), crate::sys::games::ets2::VERSION_1_06);
        assert_eq!(ets2::V1_07.raw(), crate::sys::games::ets2::VERSION_1_07);
        assert_eq!(ets2::V1_08.raw(), crate::sys::games::ets2::VERSION_1_08);
        assert_eq!(ets2::V1_09.raw(), crate::sys::games::ets2::VERSION_1_09);
        assert_eq!(ets2::V1_10.raw(), crate::sys::games::ets2::VERSION_1_10);
        assert_eq!(ets2::V1_11.raw(), crate::sys::games::ets2::VERSION_1_11);
        assert_eq!(ets2::V1_12.raw(), crate::sys::games::ets2::VERSION_1_12);
        assert_eq!(ets2::V1_13.raw(), crate::sys::games::ets2::VERSION_1_13);
        assert_eq!(ets2::V1_14.raw(), crate::sys::games::ets2::VERSION_1_14);
        assert_eq!(ets2::V1_15.raw(), crate::sys::games::ets2::VERSION_1_15);
        assert_eq!(ets2::V1_16.raw(), crate::sys::games::ets2::VERSION_1_16);
        assert_eq!(ets2::V1_17.raw(), crate::sys::games::ets2::VERSION_1_17);
        assert_eq!(ets2::V1_18.raw(), crate::sys::games::ets2::VERSION_1_18);
        assert_eq!(ets2::CURRENT, ets2::V1_18);

        assert_eq!(ats::V1_00.raw(), crate::sys::games::ats::VERSION_1_00);
        assert_eq!(ats::V1_01.raw(), crate::sys::games::ats::VERSION_1_01);
        assert_eq!(ats::V1_02.raw(), crate::sys::games::ats::VERSION_1_02);
        assert_eq!(ats::V1_03.raw(), crate::sys::games::ats::VERSION_1_03);
        assert_eq!(ats::V1_04.raw(), crate::sys::games::ats::VERSION_1_04);
        assert_eq!(ats::V1_05.raw(), crate::sys::games::ats::VERSION_1_05);
        assert_eq!(ats::CURRENT, ats::V1_05);
    }
}
