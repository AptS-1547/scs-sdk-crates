//! Game identifiers and telemetry game-version constants.
//!
//! A game version is distinct from the Telemetry API version and from the
//! public patch number shown by the game. SCS increments the minor component
//! when it adds compatible channels or fixes telemetry semantics. Plugins use
//! these values to select game-specific behavior without parsing display names.

/// Euro Truck Simulator 2 telemetry metadata.
pub mod ets2 {
    use crate::make_version;

    /// Stable game identifier supplied in `scs_sdk_init_params_v100_t::game_id`.
    ///
    /// This ergonomic module name aliases the header-shaped raw constant. It
    /// deliberately does not repeat the byte string, so both public paths stay
    /// tied to one declaration of the foreign identifier.
    pub const GAME_ID: &[u8] = crate::SCS_GAME_ID_EUT2;

    /// Initial ETS2 telemetry schema.
    pub const VERSION_1_00: u32 = make_version(1, 0);
    /// Added emergency brake-air pressure information.
    pub const VERSION_1_01: u32 = make_version(1, 1);
    /// Replaced cabin orientation with cabin offset.
    pub const VERSION_1_02: u32 = make_version(1, 2);
    /// Corrected the invalid index used by the wheel-count attribute.
    pub const VERSION_1_03: u32 = make_version(1, 3);
    /// Added physical left- and right-blinker light channels.
    pub const VERSION_1_04: u32 = make_version(1, 4);
    /// Corrected truck brand identifiers and localized names.
    pub const VERSION_1_05: u32 = make_version(1, 5);
    /// Corrected the selector-count index while retaining a compatibility copy.
    pub const VERSION_1_06: u32 = make_version(1, 6);
    /// Corrected cabin angular acceleration.
    pub const VERSION_1_07: u32 = make_version(1, 7);
    /// Added empty truck and trailer configurations when a vehicle is removed.
    pub const VERSION_1_08: u32 = make_version(1, 8);
    /// Added game-time and job-related telemetry.
    pub const VERSION_1_09: u32 = make_version(1, 9);
    /// Added liftable axle configuration information.
    pub const VERSION_1_10: u32 = make_version(1, 10);
    /// Added `u64` conversion for `u32` channels and displayed gear.
    pub const VERSION_1_11: u32 = make_version(1, 11);
    /// Added transmission ratios, navigation, and `AdBlue` telemetry.
    pub const VERSION_1_12: u32 = make_version(1, 12);
    /// Corrected trailer and cargo-accessory identifiers.
    pub const VERSION_1_13: u32 = make_version(1, 13);
    /// Added multiple trailers, trailer ownership, and gameplay events.
    pub const VERSION_1_14: u32 = make_version(1, 14);
    /// Added planned job distance.
    pub const VERSION_1_15: u32 = make_version(1, 15);
    /// Added additional fine offence identifiers.
    pub const VERSION_1_16: u32 = make_version(1, 16);
    /// Added differential lock, lift axle, and hazard-warning channels.
    pub const VERSION_1_17: u32 = make_version(1, 17);
    /// Added multiplayer time offset and corrected trailer wear channels.
    pub const VERSION_1_18: u32 = make_version(1, 18);

    /// Latest ETS2 telemetry game version declared by SDK 1.14.
    pub const VERSION_CURRENT: u32 = VERSION_1_18;
}

/// American Truck Simulator telemetry metadata.
pub mod ats {
    use crate::make_version;

    /// Stable game identifier supplied in `scs_sdk_init_params_v100_t::game_id`.
    ///
    /// This ergonomic module name aliases the header-shaped raw constant. It
    /// deliberately does not repeat the byte string, so both public paths stay
    /// tied to one declaration of the foreign identifier.
    pub const GAME_ID: &[u8] = crate::SCS_GAME_ID_ATS;

    /// Initial ATS schema, corresponding to ETS2 telemetry version 1.12.
    pub const VERSION_1_00: u32 = make_version(1, 0);
    /// Added multiple trailers, trailer ownership, and gameplay events.
    pub const VERSION_1_01: u32 = make_version(1, 1);
    /// Added planned job distance.
    pub const VERSION_1_02: u32 = make_version(1, 2);
    /// Added additional fine offence identifiers.
    pub const VERSION_1_03: u32 = make_version(1, 3);
    /// Added differential lock, lift axle, and hazard-warning channels.
    pub const VERSION_1_04: u32 = make_version(1, 4);
    /// Added multiplayer time offset and corrected trailer wear channels.
    pub const VERSION_1_05: u32 = make_version(1, 5);

    /// Latest ATS telemetry game version declared by SDK 1.14.
    pub const VERSION_CURRENT: u32 = VERSION_1_05;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_game_ids_alias_the_header_shaped_constants() {
        assert_eq!(ets2::GAME_ID, crate::SCS_GAME_ID_EUT2);
        assert_eq!(ats::GAME_ID, crate::SCS_GAME_ID_ATS);
    }
}
