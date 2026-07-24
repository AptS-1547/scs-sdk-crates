//! Typed gameplay-event catalog for the SCS Telemetry SDK.
//!
//! Event and attribute descriptors mirror every macro in
//! `scssdk_telemetry_common_gameplay_events.h`.

use core::ffi::CStr;

/// Gameplay event identifiers.
pub mod events {
    use crate::GameplayEventId;

    /// Number of official gameplay event identifiers.
    pub const COUNT: usize = 6;

    /// Event called when job is cancelled.
    ///
    /// Attributes:
    /// - `cancel_penalty`
    pub const JOB_CANCELLED: GameplayEventId = GameplayEventId::new(c"job.cancelled");

    /// Event called when job is delivered.
    ///
    /// Attributes:
    /// - revenue
    /// - `earned_xp`
    /// - `cargo_damage`
    /// - `distance_km`
    /// - `delivery_time`
    /// - `autopark_used`
    /// - `autoload_used`
    pub const JOB_DELIVERED: GameplayEventId = GameplayEventId::new(c"job.delivered");

    /// Event called when player gets fined.
    ///
    /// Attributes:
    /// - `fine_offence`
    /// - `fine_amount`
    pub const PLAYER_FINED: GameplayEventId = GameplayEventId::new(c"player.fined");

    /// Event called when player pays for a tollgate.
    ///
    /// Attributes:
    /// - `pay_amount`
    pub const PLAYER_TOLLGATE_PAID: GameplayEventId = GameplayEventId::new(c"player.tollgate.paid");

    /// Event called when player uses a ferry.
    ///
    /// Attributes:
    /// - `pay_amount`
    /// - `source_name`
    /// - `target_name`
    /// - `source_id`
    /// - `target_id`
    pub const PLAYER_USE_FERRY: GameplayEventId = GameplayEventId::new(c"player.use.ferry");

    /// Event called when player uses a train.
    ///
    /// Attributes:
    /// - `pay_amount`
    /// - `source_name`
    /// - `target_name`
    /// - `source_id`
    /// - `target_id`
    pub const PLAYER_USE_TRAIN: GameplayEventId = GameplayEventId::new(c"player.use.train");

    /// Every public gameplay event in SDK 1.14 header order.
    pub const ALL: [GameplayEventId; COUNT] = [
        JOB_CANCELLED,
        JOB_DELIVERED,
        PLAYER_FINED,
        PLAYER_TOLLGATE_PAID,
        PLAYER_USE_FERRY,
        PLAYER_USE_TRAIN,
    ];
}

/// Typed attributes carried by gameplay events.
pub mod attributes {
    use crate::{AnyAttribute, Attribute};

    /// Number of official gameplay attributes.
    pub const COUNT: usize = 15;

    /// The penalty for cancelling the job in native game currency. (Can be 0)
    ///
    /// Type: s64
    pub const CANCEL_PENALTY: Attribute<i64> = Attribute::new(c"cancel.penalty");

    /// The job revenue in native game currency.
    ///
    /// Type: s64
    pub const REVENUE: Attribute<i64> = Attribute::new(c"revenue");

    /// How much XP player received for the job.
    ///
    /// Type: s32
    pub const EARNED_XP: Attribute<i32> = Attribute::new(c"earned.xp");

    /// Total cargo damage. (Range <0.0, 1.0>)
    ///
    /// Type: float
    pub const CARGO_DAMAGE: Attribute<f32> = Attribute::new(c"cargo.damage");

    /// The real distance in km on the job.
    ///
    /// Type: float
    pub const DISTANCE_KM: Attribute<f32> = Attribute::new(c"distance.km");

    /// Total time spend on the job in game minutes.
    ///
    /// Type: u32
    pub const DELIVERY_TIME: Attribute<u32> = Attribute::new(c"delivery.time");

    /// Was auto parking used on this job?
    ///
    /// Type: bool
    pub const AUTO_PARK_USED: Attribute<bool> = Attribute::new(c"auto.park.used");

    /// Was auto loading used on this job? (always `true` for non cargo market jobs)
    ///
    /// Type: bool
    pub const AUTO_LOAD_USED: Attribute<bool> = Attribute::new(c"auto.load.used");

    /// Fine offence type.
    ///
    /// Possible values:
    /// - crash
    /// - `avoid_sleeping`
    /// - `wrong_way`
    /// - `speeding_camera`
    /// - `no_lights`
    /// - `red_signal`
    /// - speeding
    /// - `avoid_weighing`
    /// - `illegal_trailer`
    /// - `avoid_inspection`
    /// - `illegal_border_crossing`
    /// - `hard_shoulder_violation`
    /// - `damaged_vehicle_usage`
    /// - generic
    ///
    /// Type: string
    pub const FINE_OFFENCE: Attribute<crate::StringValue> = Attribute::new(c"fine.offence");

    /// Fine offence amount in native game currency.
    ///
    /// Type: s64
    pub const FINE_AMOUNT: Attribute<i64> = Attribute::new(c"fine.amount");

    /// How much player was charged for this action (in native game currency)
    ///
    /// Type: s64
    pub const PAY_AMOUNT: Attribute<i64> = Attribute::new(c"pay.amount");

    /// The name of the transportation source.
    ///
    /// Type: string
    pub const SOURCE_NAME: Attribute<crate::StringValue> = Attribute::new(c"source.name");

    /// The name of the transportation target.
    ///
    /// Type: string
    pub const TARGET_NAME: Attribute<crate::StringValue> = Attribute::new(c"target.name");

    /// The id of the transportation source.
    ///
    /// Type: string
    pub const SOURCE_ID: Attribute<crate::StringValue> = Attribute::new(c"source.id");

    /// The id of the transportation target.
    ///
    /// Type: string
    pub const TARGET_ID: Attribute<crate::StringValue> = Attribute::new(c"target.id");

    /// Every public gameplay-event attribute from the SDK 1.14 header.
    ///
    /// This heterogeneous catalog is intended for enumeration and coverage
    /// checks. Individual typed constants remain the decoding API.
    pub const ALL: [AnyAttribute; COUNT] = [
        CANCEL_PENALTY.erase(),
        REVENUE.erase(),
        EARNED_XP.erase(),
        CARGO_DAMAGE.erase(),
        DISTANCE_KM.erase(),
        DELIVERY_TIME.erase(),
        AUTO_PARK_USED.erase(),
        AUTO_LOAD_USED.erase(),
        FINE_OFFENCE.erase(),
        FINE_AMOUNT.erase(),
        PAY_AMOUNT.erase(),
        SOURCE_NAME.erase(),
        TARGET_NAME.erase(),
        SOURCE_ID.erase(),
        TARGET_ID.erase(),
    ];
}

/// Fine offence identifiers documented by the SDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FineOffence {
    Crash,
    AvoidSleeping,
    WrongWay,
    SpeedingCamera,
    NoLights,
    RedSignal,
    Speeding,
    AvoidWeighing,
    IllegalTrailer,
    AvoidInspection,
    IllegalBorderCrossing,
    HardShoulderViolation,
    DamagedVehicleUsage,
    Generic,
}

impl FineOffence {
    #[must_use]
    pub const fn as_c_str(self) -> &'static CStr {
        match self {
            Self::Crash => c"crash",
            Self::AvoidSleeping => c"avoid_sleeping",
            Self::WrongWay => c"wrong_way",
            Self::SpeedingCamera => c"speeding_camera",
            Self::NoLights => c"no_lights",
            Self::RedSignal => c"red_signal",
            Self::Speeding => c"speeding",
            Self::AvoidWeighing => c"avoid_weighing",
            Self::IllegalTrailer => c"illegal_trailer",
            Self::AvoidInspection => c"avoid_inspection",
            Self::IllegalBorderCrossing => c"illegal_border_crossing",
            Self::HardShoulderViolation => c"hard_shoulder_violation",
            Self::DamagedVehicleUsage => c"damaged_vehicle_usage",
            Self::Generic => c"generic",
        }
    }

    #[must_use]
    pub fn from_c_str(value: &CStr) -> Option<Self> {
        match value.to_bytes() {
            b"crash" => Some(Self::Crash),
            b"avoid_sleeping" => Some(Self::AvoidSleeping),
            b"wrong_way" => Some(Self::WrongWay),
            b"speeding_camera" => Some(Self::SpeedingCamera),
            b"no_lights" => Some(Self::NoLights),
            b"red_signal" => Some(Self::RedSignal),
            b"speeding" => Some(Self::Speeding),
            b"avoid_weighing" => Some(Self::AvoidWeighing),
            b"illegal_trailer" => Some(Self::IllegalTrailer),
            b"avoid_inspection" => Some(Self::AvoidInspection),
            b"illegal_border_crossing" => Some(Self::IllegalBorderCrossing),
            b"hard_shoulder_violation" => Some(Self::HardShoulderViolation),
            b"damaged_vehicle_usage" => Some(Self::DamagedVehicleUsage),
            b"generic" => Some(Self::Generic),
            _ => None,
        }
    }
}

const _: [(); 6] = [(); events::COUNT];
const _: [(); 15] = [(); attributes::COUNT];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_catalog_counts_match_the_raw_sdk_catalog() {
        assert_eq!(events::COUNT, crate::sys::gameplay::events::COUNT);
        assert_eq!(attributes::COUNT, crate::sys::gameplay::attributes::COUNT);
        assert_eq!(events::COUNT, 6);
        assert_eq!(attributes::COUNT, 15);
    }

    #[test]
    fn every_typed_descriptor_matches_the_raw_header_catalog() {
        for (descriptor, raw_name) in events::ALL.iter().zip(crate::sys::gameplay::events::ALL) {
            assert_eq!(descriptor.name().to_bytes_with_nul(), raw_name);
        }
        for (descriptor, raw_name) in attributes::ALL
            .iter()
            .zip(crate::sys::gameplay::attributes::ALL)
        {
            assert_eq!(descriptor.name().to_bytes_with_nul(), raw_name);
            assert!(!descriptor.is_indexed());
        }

        for (position, descriptor) in attributes::ALL.iter().enumerate() {
            assert!(
                attributes::ALL[..position]
                    .iter()
                    .all(|earlier| earlier.name() != descriptor.name()),
                "duplicate gameplay attribute: {:?}",
                descriptor.name()
            );
        }
    }

    #[test]
    fn representative_descriptors_preserve_header_metadata() {
        assert_eq!(events::JOB_DELIVERED.name(), c"job.delivered");
        assert_eq!(attributes::REVENUE.name(), c"revenue");
        assert_eq!(
            attributes::REVENUE.value_type(),
            crate::sys::SCS_VALUE_TYPE_S64
        );
        assert!(!attributes::REVENUE.is_indexed());
    }

    #[test]
    fn every_documented_fine_offence_round_trips() {
        let values = [
            FineOffence::Crash,
            FineOffence::AvoidSleeping,
            FineOffence::WrongWay,
            FineOffence::SpeedingCamera,
            FineOffence::NoLights,
            FineOffence::RedSignal,
            FineOffence::Speeding,
            FineOffence::AvoidWeighing,
            FineOffence::IllegalTrailer,
            FineOffence::AvoidInspection,
            FineOffence::IllegalBorderCrossing,
            FineOffence::HardShoulderViolation,
            FineOffence::DamagedVehicleUsage,
            FineOffence::Generic,
        ];
        for value in values {
            assert_eq!(FineOffence::from_c_str(value.as_c_str()), Some(value));
        }
        assert_eq!(FineOffence::from_c_str(c"future-offence"), None);
    }
}
