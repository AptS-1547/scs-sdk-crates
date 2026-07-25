//! Typed gameplay-event catalog for the SCS Telemetry SDK.
//!
//! Event and attribute descriptors mirror every macro in
//! `scssdk_telemetry_common_gameplay_events.h`.

use core::ffi::CStr;
use core::str::FromStr;

use crate::{GameSchemaAvailability, UnknownStringValue, game};

// Gameplay descriptors appeared together with multi-trailer support. Their
// representation additionally requires Telemetry API 1.01, which remains a
// separate capability check owned by Event and ValueType.
const GAMEPLAY_AVAILABILITY: GameSchemaAvailability = game::capabilities::GAMEPLAY_EVENTS;

/// Gameplay event identifiers.
pub mod events {
    use super::GAMEPLAY_AVAILABILITY;
    use crate::GameplayEventId;

    /// Number of official gameplay event identifiers.
    pub const COUNT: usize = 6;

    /// Event called when job is cancelled.
    ///
    /// Attributes:
    /// - `cancel_penalty`
    pub const JOB_CANCELLED: GameplayEventId =
        GameplayEventId::new(c"job.cancelled", GAMEPLAY_AVAILABILITY);

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
    pub const JOB_DELIVERED: GameplayEventId =
        GameplayEventId::new(c"job.delivered", GAMEPLAY_AVAILABILITY);

    /// Event called when player gets fined.
    ///
    /// Attributes:
    /// - `fine_offence`
    /// - `fine_amount`
    pub const PLAYER_FINED: GameplayEventId =
        GameplayEventId::new(c"player.fined", GAMEPLAY_AVAILABILITY);

    /// Event called when player pays for a tollgate.
    ///
    /// Attributes:
    /// - `pay_amount`
    pub const PLAYER_TOLLGATE_PAID: GameplayEventId =
        GameplayEventId::new(c"player.tollgate.paid", GAMEPLAY_AVAILABILITY);

    /// Event called when player uses a ferry.
    ///
    /// Attributes:
    /// - `pay_amount`
    /// - `source_name`
    /// - `target_name`
    /// - `source_id`
    /// - `target_id`
    pub const PLAYER_USE_FERRY: GameplayEventId =
        GameplayEventId::new(c"player.use.ferry", GAMEPLAY_AVAILABILITY);

    /// Event called when player uses a train.
    ///
    /// Attributes:
    /// - `pay_amount`
    /// - `source_name`
    /// - `target_name`
    /// - `source_id`
    /// - `target_id`
    pub const PLAYER_USE_TRAIN: GameplayEventId =
        GameplayEventId::new(c"player.use.train", GAMEPLAY_AVAILABILITY);

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
    use super::GAMEPLAY_AVAILABILITY;
    use crate::{AnyAttribute, Attribute};

    /// Number of official gameplay attributes.
    pub const COUNT: usize = 15;

    /// The penalty for cancelling the job in native game currency. (Can be 0)
    ///
    /// Type: s64
    pub const CANCEL_PENALTY: Attribute<i64> =
        Attribute::new(c"cancel.penalty", GAMEPLAY_AVAILABILITY);

    /// The job revenue in native game currency.
    ///
    /// Type: s64
    pub const REVENUE: Attribute<i64> = Attribute::new(c"revenue", GAMEPLAY_AVAILABILITY);

    /// How much XP player received for the job.
    ///
    /// Type: s32
    pub const EARNED_XP: Attribute<i32> = Attribute::new(c"earned.xp", GAMEPLAY_AVAILABILITY);

    /// Total cargo damage. (Range <0.0, 1.0>)
    ///
    /// Type: float
    pub const CARGO_DAMAGE: Attribute<f32> = Attribute::new(c"cargo.damage", GAMEPLAY_AVAILABILITY);

    /// The real distance in km on the job.
    ///
    /// Type: float
    pub const DISTANCE_KM: Attribute<f32> = Attribute::new(c"distance.km", GAMEPLAY_AVAILABILITY);

    /// Total time spend on the job in game minutes.
    ///
    /// Type: u32
    pub const DELIVERY_TIME: Attribute<u32> =
        Attribute::new(c"delivery.time", GAMEPLAY_AVAILABILITY);

    /// Was auto parking used on this job?
    ///
    /// Type: bool
    pub const AUTO_PARK_USED: Attribute<bool> =
        Attribute::new(c"auto.park.used", GAMEPLAY_AVAILABILITY);

    /// Was auto loading used on this job? (always `true` for non cargo market jobs)
    ///
    /// Type: bool
    pub const AUTO_LOAD_USED: Attribute<bool> =
        Attribute::new(c"auto.load.used", GAMEPLAY_AVAILABILITY);

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
    pub const FINE_OFFENCE: Attribute<crate::StringValue> =
        Attribute::new(c"fine.offence", GAMEPLAY_AVAILABILITY);

    /// Fine offence amount in native game currency.
    ///
    /// Type: s64
    pub const FINE_AMOUNT: Attribute<i64> = Attribute::new(c"fine.amount", GAMEPLAY_AVAILABILITY);

    /// How much player was charged for this action (in native game currency)
    ///
    /// Type: s64
    pub const PAY_AMOUNT: Attribute<i64> = Attribute::new(c"pay.amount", GAMEPLAY_AVAILABILITY);

    /// The name of the transportation source.
    ///
    /// Type: string
    pub const SOURCE_NAME: Attribute<crate::StringValue> =
        Attribute::new(c"source.name", GAMEPLAY_AVAILABILITY);

    /// The name of the transportation target.
    ///
    /// Type: string
    pub const TARGET_NAME: Attribute<crate::StringValue> =
        Attribute::new(c"target.name", GAMEPLAY_AVAILABILITY);

    /// The id of the transportation source.
    ///
    /// Type: string
    pub const SOURCE_ID: Attribute<crate::StringValue> =
        Attribute::new(c"source.id", GAMEPLAY_AVAILABILITY);

    /// The id of the transportation target.
    ///
    /// Type: string
    pub const TARGET_ID: Attribute<crate::StringValue> =
        Attribute::new(c"target.id", GAMEPLAY_AVAILABILITY);

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

/// Gameplay-event membership for every payload attribute.
///
/// The SDK declares event IDs and attribute names as separate flat macro
/// catalogs. These associations preserve the event payload schema, including
/// attributes shared by ferry and train events, in an enumerable form suitable
/// for diagnostics and schema generation.
pub mod associations {
    use super::{GAMEPLAY_AVAILABILITY, attributes, events};
    use crate::{
        Attribute, GameSchemaAvailability, GameplayAttributeAssociation, GameplayEventId, SdkValue,
    };

    /// Total number of gameplay-event to attribute relationships.
    pub const COUNT: usize = 21;

    const fn association<T: SdkValue>(
        event: GameplayEventId,
        attribute: Attribute<T>,
        availability: GameSchemaAvailability,
    ) -> GameplayAttributeAssociation {
        GameplayAttributeAssociation::new(event, attribute, availability)
    }

    /// Payload schema for `job.cancelled`.
    pub const JOB_CANCELLED: [GameplayAttributeAssociation; 1] = [association(
        events::JOB_CANCELLED,
        attributes::CANCEL_PENALTY,
        GAMEPLAY_AVAILABILITY,
    )];

    /// Payload schema for `job.delivered`.
    pub const JOB_DELIVERED: [GameplayAttributeAssociation; 7] = [
        association(
            events::JOB_DELIVERED,
            attributes::REVENUE,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::JOB_DELIVERED,
            attributes::EARNED_XP,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::JOB_DELIVERED,
            attributes::CARGO_DAMAGE,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::JOB_DELIVERED,
            attributes::DISTANCE_KM,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::JOB_DELIVERED,
            attributes::DELIVERY_TIME,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::JOB_DELIVERED,
            attributes::AUTO_PARK_USED,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::JOB_DELIVERED,
            attributes::AUTO_LOAD_USED,
            GAMEPLAY_AVAILABILITY,
        ),
    ];

    /// Payload schema for `player.fined`.
    pub const PLAYER_FINED: [GameplayAttributeAssociation; 2] = [
        association(
            events::PLAYER_FINED,
            attributes::FINE_OFFENCE,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_FINED,
            attributes::FINE_AMOUNT,
            GAMEPLAY_AVAILABILITY,
        ),
    ];

    /// Payload schema for `player.tollgate.paid`.
    pub const PLAYER_TOLLGATE_PAID: [GameplayAttributeAssociation; 1] = [association(
        events::PLAYER_TOLLGATE_PAID,
        attributes::PAY_AMOUNT,
        GAMEPLAY_AVAILABILITY,
    )];

    /// Payload schema for `player.use.ferry`.
    pub const PLAYER_USE_FERRY: [GameplayAttributeAssociation; 5] = [
        association(
            events::PLAYER_USE_FERRY,
            attributes::PAY_AMOUNT,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_USE_FERRY,
            attributes::SOURCE_NAME,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_USE_FERRY,
            attributes::TARGET_NAME,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_USE_FERRY,
            attributes::SOURCE_ID,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_USE_FERRY,
            attributes::TARGET_ID,
            GAMEPLAY_AVAILABILITY,
        ),
    ];

    /// Payload schema for `player.use.train`.
    pub const PLAYER_USE_TRAIN: [GameplayAttributeAssociation; 5] = [
        association(
            events::PLAYER_USE_TRAIN,
            attributes::PAY_AMOUNT,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_USE_TRAIN,
            attributes::SOURCE_NAME,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_USE_TRAIN,
            attributes::TARGET_NAME,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_USE_TRAIN,
            attributes::SOURCE_ID,
            GAMEPLAY_AVAILABILITY,
        ),
        association(
            events::PLAYER_USE_TRAIN,
            attributes::TARGET_ID,
            GAMEPLAY_AVAILABILITY,
        ),
    ];

    /// Every official gameplay-event to attribute relationship in event-header
    /// order, preserving each event's documented payload order.
    pub const ALL: [GameplayAttributeAssociation; COUNT] = [
        JOB_CANCELLED[0],
        JOB_DELIVERED[0],
        JOB_DELIVERED[1],
        JOB_DELIVERED[2],
        JOB_DELIVERED[3],
        JOB_DELIVERED[4],
        JOB_DELIVERED[5],
        JOB_DELIVERED[6],
        PLAYER_FINED[0],
        PLAYER_FINED[1],
        PLAYER_TOLLGATE_PAID[0],
        PLAYER_USE_FERRY[0],
        PLAYER_USE_FERRY[1],
        PLAYER_USE_FERRY[2],
        PLAYER_USE_FERRY[3],
        PLAYER_USE_FERRY[4],
        PLAYER_USE_TRAIN[0],
        PLAYER_USE_TRAIN[1],
        PLAYER_USE_TRAIN[2],
        PLAYER_USE_TRAIN[3],
        PLAYER_USE_TRAIN[4],
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
    /// Number of fine-offence strings documented by SDK 1.14.
    pub const COUNT: usize = 14;

    /// Every documented fine-offence value in header order.
    pub const ALL: [Self; Self::COUNT] = [
        Self::Crash,
        Self::AvoidSleeping,
        Self::WrongWay,
        Self::SpeedingCamera,
        Self::NoLights,
        Self::RedSignal,
        Self::Speeding,
        Self::AvoidWeighing,
        Self::IllegalTrailer,
        Self::AvoidInspection,
        Self::IllegalBorderCrossing,
        Self::HardShoulderViolation,
        Self::DamagedVehicleUsage,
        Self::Generic,
    ];

    /// First game schemas which may emit this exact string value.
    ///
    /// This is value-level history, independent of the `fine.offence`
    /// attribute descriptor and its `player.fined` association. Four values
    /// were appended after gameplay events were introduced, so treating every
    /// currently known string as available since the event itself would be
    /// inaccurate for schema-aware decoders.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        match self {
            Self::AvoidInspection
            | Self::IllegalBorderCrossing
            | Self::HardShoulderViolation
            | Self::DamagedVehicleUsage => {
                GameSchemaAvailability::new(Some(game::ets2::V1_16), Some(game::ats::V1_03))
            }
            Self::Crash
            | Self::AvoidSleeping
            | Self::WrongWay
            | Self::SpeedingCamera
            | Self::NoLights
            | Self::RedSignal
            | Self::Speeding
            | Self::AvoidWeighing
            | Self::IllegalTrailer
            | Self::Generic => GAMEPLAY_AVAILABILITY,
        }
    }

    /// Canonical Rust string used by the SDK.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Crash => "crash",
            Self::AvoidSleeping => "avoid_sleeping",
            Self::WrongWay => "wrong_way",
            Self::SpeedingCamera => "speeding_camera",
            Self::NoLights => "no_lights",
            Self::RedSignal => "red_signal",
            Self::Speeding => "speeding",
            Self::AvoidWeighing => "avoid_weighing",
            Self::IllegalTrailer => "illegal_trailer",
            Self::AvoidInspection => "avoid_inspection",
            Self::IllegalBorderCrossing => "illegal_border_crossing",
            Self::HardShoulderViolation => "hard_shoulder_violation",
            Self::DamagedVehicleUsage => "damaged_vehicle_usage",
            Self::Generic => "generic",
        }
    }

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
        value.to_str().ok()?.parse().ok()
    }
}

impl FromStr for FineOffence {
    type Err = UnknownStringValue;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "crash" => Ok(Self::Crash),
            "avoid_sleeping" => Ok(Self::AvoidSleeping),
            "wrong_way" => Ok(Self::WrongWay),
            "speeding_camera" => Ok(Self::SpeedingCamera),
            "no_lights" => Ok(Self::NoLights),
            "red_signal" => Ok(Self::RedSignal),
            "speeding" => Ok(Self::Speeding),
            "avoid_weighing" => Ok(Self::AvoidWeighing),
            "illegal_trailer" => Ok(Self::IllegalTrailer),
            "avoid_inspection" => Ok(Self::AvoidInspection),
            "illegal_border_crossing" => Ok(Self::IllegalBorderCrossing),
            "hard_shoulder_violation" => Ok(Self::HardShoulderViolation),
            "damaged_vehicle_usage" => Ok(Self::DamagedVehicleUsage),
            "generic" => Ok(Self::Generic),
            _ => Err(UnknownStringValue),
        }
    }
}

const _: [(); 6] = [(); events::COUNT];
const _: [(); 15] = [(); attributes::COUNT];
const _: [(); 21] = [(); associations::COUNT];
const _: [(); 14] = [(); FineOffence::COUNT];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_catalog_counts_match_the_raw_sdk_catalog() {
        assert_eq!(events::COUNT, crate::sys::gameplay::events::COUNT);
        assert_eq!(attributes::COUNT, crate::sys::gameplay::attributes::COUNT);
        assert_eq!(events::COUNT, 6);
        assert_eq!(attributes::COUNT, 15);
        assert_eq!(associations::COUNT, 21);
        assert_eq!(associations::JOB_CANCELLED.len(), 1);
        assert_eq!(associations::JOB_DELIVERED.len(), 7);
        assert_eq!(associations::PLAYER_FINED.len(), 2);
        assert_eq!(associations::PLAYER_TOLLGATE_PAID.len(), 1);
        assert_eq!(associations::PLAYER_USE_FERRY.len(), 5);
        assert_eq!(associations::PLAYER_USE_TRAIN.len(), 5);
    }

    #[test]
    fn gameplay_associations_cover_every_descriptor_without_duplicate_pairs() {
        for association in associations::ALL {
            assert!(events::ALL.contains(&association.event()));
            assert!(attributes::ALL.contains(&association.attribute()));
            assert_eq!(association.availability(), GAMEPLAY_AVAILABILITY);
        }

        for event in events::ALL {
            assert!(
                associations::ALL
                    .iter()
                    .any(|association| association.event() == event),
                "gameplay event has no payload schema: {:?}",
                event.name()
            );
        }
        for attribute in attributes::ALL {
            assert!(
                associations::ALL
                    .iter()
                    .any(|association| association.attribute() == attribute),
                "gameplay attribute has no event membership: {:?}",
                attribute.name()
            );
        }

        for (position, association) in associations::ALL.iter().enumerate() {
            assert!(
                associations::ALL[..position].iter().all(|earlier| {
                    earlier.event() != association.event()
                        || earlier.attribute() != association.attribute()
                }),
                "duplicate gameplay association: {:?} -> {:?}",
                association.event().name(),
                association.attribute().name()
            );
        }
    }

    #[test]
    fn gameplay_association_catalog_preserves_event_and_payload_order() {
        let groups = [
            (events::JOB_CANCELLED, 0..1),
            (events::JOB_DELIVERED, 1..8),
            (events::PLAYER_FINED, 8..10),
            (events::PLAYER_TOLLGATE_PAID, 10..11),
            (events::PLAYER_USE_FERRY, 11..16),
            (events::PLAYER_USE_TRAIN, 16..21),
        ];

        for (event, range) in groups {
            assert!(
                associations::ALL[range]
                    .iter()
                    .all(|association| association.event() == event)
            );
        }

        assert_eq!(
            associations::JOB_DELIVERED.map(crate::GameplayAttributeAssociation::attribute),
            [
                attributes::REVENUE.erase(),
                attributes::EARNED_XP.erase(),
                attributes::CARGO_DAMAGE.erase(),
                attributes::DISTANCE_KM.erase(),
                attributes::DELIVERY_TIME.erase(),
                attributes::AUTO_PARK_USED.erase(),
                attributes::AUTO_LOAD_USED.erase(),
            ]
        );
        assert_eq!(
            associations::PLAYER_USE_TRAIN.map(crate::GameplayAttributeAssociation::attribute),
            [
                attributes::PAY_AMOUNT.erase(),
                attributes::SOURCE_NAME.erase(),
                attributes::TARGET_NAME.erase(),
                attributes::SOURCE_ID.erase(),
                attributes::TARGET_ID.erase(),
            ]
        );
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
    fn gameplay_descriptors_share_the_official_introduction_schema() {
        for event in events::ALL {
            assert_eq!(
                event.availability().available_since_ets2(),
                Some(game::ets2::V1_14)
            );
            assert_eq!(
                event.availability().available_since_ats(),
                Some(game::ats::V1_01)
            );
        }
        for attribute in attributes::ALL {
            assert_eq!(
                attribute.availability().available_since_ets2(),
                Some(game::ets2::V1_14)
            );
            assert_eq!(
                attribute.availability().available_since_ats(),
                Some(game::ats::V1_01)
            );
        }
    }

    #[test]
    fn every_documented_fine_offence_round_trips() {
        assert_eq!(FineOffence::COUNT, 14);
        for value in FineOffence::ALL {
            assert_eq!(FineOffence::from_c_str(value.as_c_str()), Some(value));
            assert_eq!(value.as_str().parse(), Ok(value));
        }
        assert_eq!(FineOffence::from_c_str(c"future-offence"), None);
        assert_eq!(
            "future-offence".parse::<FineOffence>(),
            Err(UnknownStringValue)
        );
    }

    #[test]
    fn fine_offence_values_keep_value_level_schema_history() {
        for value in FineOffence::ALL {
            let expected = match value {
                FineOffence::AvoidInspection
                | FineOffence::IllegalBorderCrossing
                | FineOffence::HardShoulderViolation
                | FineOffence::DamagedVehicleUsage => {
                    GameSchemaAvailability::new(Some(game::ets2::V1_16), Some(game::ats::V1_03))
                }
                FineOffence::Crash
                | FineOffence::AvoidSleeping
                | FineOffence::WrongWay
                | FineOffence::SpeedingCamera
                | FineOffence::NoLights
                | FineOffence::RedSignal
                | FineOffence::Speeding
                | FineOffence::AvoidWeighing
                | FineOffence::IllegalTrailer
                | FineOffence::Generic => GAMEPLAY_AVAILABILITY,
            };
            assert_eq!(value.availability(), expected, "offence {value:?}");
        }
    }
}
