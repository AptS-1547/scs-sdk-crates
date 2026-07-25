//! Typed configuration catalog for the SCS Telemetry SDK.
//!
//! This module covers every public configuration ID, attribute, and
//! documented H-shifter value in the SDK 1.14 header bundle.

use core::ffi::CStr;
use core::str::FromStr;

use crate::{GameSchemaAvailability, UnknownStringValue, game};

// Configuration descriptors use the game telemetry schema, not the SDK
// archive number or the negotiated Telemetry API. ATS schema 1.00 corresponds
// to ETS2 schema 1.12, which is why the two minima often differ.
const INITIAL: GameSchemaAvailability =
    GameSchemaAvailability::new(Some(game::ets2::V1_00), Some(game::ats::V1_00));
const ETS2_1_01_ATS_1_00: GameSchemaAvailability =
    GameSchemaAvailability::new(Some(game::ets2::V1_01), Some(game::ats::V1_00));
const ETS2_1_09_ATS_1_00: GameSchemaAvailability =
    GameSchemaAvailability::new(Some(game::ets2::V1_09), Some(game::ats::V1_00));
const ETS2_1_10_ATS_1_00: GameSchemaAvailability =
    GameSchemaAvailability::new(Some(game::ets2::V1_10), Some(game::ats::V1_00));
const ETS2_1_12_ATS_1_00: GameSchemaAvailability =
    GameSchemaAvailability::new(Some(game::ets2::V1_12), Some(game::ats::V1_00));
const ETS2_1_14_ATS_1_01: GameSchemaAvailability =
    GameSchemaAvailability::new(Some(game::ets2::V1_14), Some(game::ats::V1_01));
const ETS2_1_15_ATS_1_02: GameSchemaAvailability =
    GameSchemaAvailability::new(Some(game::ets2::V1_15), Some(game::ats::V1_02));

/// Maximum number of trailers represented by the SDK.
pub const MAX_TRAILERS: usize = crate::TrailerIndex::COUNT;

/// Configuration group identifiers.
pub mod ids {
    use super::{ETS2_1_09_ATS_1_00, INITIAL};
    use crate::ConfigurationId;

    /// Number of official configuration identifiers.
    pub const COUNT: usize = 6;

    /// Configuration of the substances.
    ///
    /// Attribute index is index of the substance.
    ///
    /// Supported attributes:
    /// - `id`
    ///
    /// The upstream SDK header leaves the remaining substance attributes as a
    /// TODO, so version 1.14 does not define any additional public names here.
    pub const SUBSTANCES: ConfigurationId = ConfigurationId::new(c"substances", INITIAL);

    /// Static configuration of the controls.
    ///
    /// - `shifter_type`
    pub const CONTROLS: ConfigurationId = ConfigurationId::new(c"controls", INITIAL);

    /// Configuration of the h-shifter.
    ///
    /// When evaluating the selected gear, find slot which matches
    /// the handle position and bitmask of on/off state of selectors.
    /// If one is found, it contains the resulting gear. Otherwise
    /// a neutral is assumed.
    ///
    /// Supported attributes:
    /// - `selector_count`
    /// - resulting gear index for each slot
    /// - handle position index for each slot
    /// - bitmask of selectors for each slot
    pub const HSHIFTER: ConfigurationId = ConfigurationId::new(c"hshifter", INITIAL);

    /// Static configuration of the truck.
    ///
    /// If empty set of attributes is returned, there is no configured truck.
    ///
    /// Supported attributes:
    /// - `brand_id`
    /// - brand
    /// - id
    /// - name
    /// - `fuel_capacity`
    /// - `fuel_warning_factor`
    /// - `adblue_capacity`
    /// - `ablue_warning_factor`
    /// - `air_pressure_warning`
    /// - `air_pressure_emergency`
    /// - `oil_pressure_warning`
    /// - `water_temperature_warning`
    /// - `battery_voltage_warning`
    /// - `rpm_limit`
    /// - `foward_gear_count`
    /// - `reverse_gear_count`
    /// - `retarder_step_count`
    /// - `cabin_position`
    /// - `head_position`
    /// - `hook_position`
    /// - `license_plate`
    /// - `license_plate_country`
    /// - `license_plate_country_id`
    /// - `wheel_count`
    /// - wheel positions for `wheel_count` wheels
    pub const TRUCK: ConfigurationId = ConfigurationId::new(c"truck", INITIAL);

    /// Backward compatibility static configuration of the first trailer (attributes are equal to trailer.0).
    ///
    /// The trailers configurations are returned using `trailer.[index]`
    /// (e.g. trailer.0, trailer.1, ... trailer.9 ...)
    ///
    /// SDK currently can return up to `SCS_TELEMETRY_trailers_count` trailers.
    ///
    /// If there are less trailers in game than `SCS_TELEMETRY_trailers_count`
    /// telemetry will return all configurations however starting from the trailer after last
    /// existing one its attributes will be empty.
    ///
    /// Supported attributes:
    /// - id
    /// - `cargo_accessory_id`
    /// - `hook_position`
    /// - `brand_id`
    /// - brand
    /// - name
    /// - `chain_type` (reported only for first trailer)
    /// - `body_type` (reported only for first trailer)
    /// - `license_plate`
    /// - `license_plate_country`
    /// - `license_plate_country_id`
    /// - `wheel_count`
    /// - wheel offsets for `wheel_count` wheels
    pub const TRAILER: ConfigurationId = ConfigurationId::new(c"trailer", INITIAL);

    /// Static configuration of the job.
    ///
    /// If empty set of attributes is returned, there is no job.
    ///
    /// Supported attributes:
    /// - `cargo_id`
    /// - cargo
    /// - `cargo_mass`
    /// - `destination_city_id`
    /// - `destination_city`
    /// - `source_city_id`
    /// - `source_city`
    /// - `destination_company_id` (only available for non special transport jobs)
    /// - `destination_company` (only available for non special transport jobs)
    /// - `source_company_id` (only available for non special transport jobs)
    /// - `source_company` (only available for non special transport jobs)
    /// - income - represents expected income for the job without any penalties
    /// - `delivery_time`
    /// - `is_cargo_loaded`
    /// - `job_market`
    /// - `special_job`
    /// - `planned_distance_km`
    pub const JOB: ConfigurationId = ConfigurationId::new(c"job", ETS2_1_09_ATS_1_00);

    /// Every public configuration group in SDK 1.14 header order.
    ///
    /// The trailer entry is the backward-compatible `trailer` identifier. The
    /// game additionally emits `trailer.0` through `trailer.9`; consumers must
    /// still select those numbered groups explicitly when they need them.
    pub const ALL: [ConfigurationId; COUNT] = [SUBSTANCES, CONTROLS, HSHIFTER, TRUCK, TRAILER, JOB];
}

/// Typed attributes carried by configuration events.
pub mod attributes {
    use super::{
        ETS2_1_01_ATS_1_00, ETS2_1_09_ATS_1_00, ETS2_1_10_ATS_1_00, ETS2_1_12_ATS_1_00,
        ETS2_1_14_ATS_1_01, ETS2_1_15_ATS_1_02, INITIAL,
    };
    use crate::{AnyAttribute, Attribute};

    /// Number of official typed configuration attributes.
    pub const COUNT: usize = 60;

    /// Brand id for configuration purposes.
    ///
    /// Limited to C-identifier characters.
    ///
    /// Type: string
    pub const BRAND_ID: Attribute<crate::StringValue> = Attribute::new(c"brand_id", INITIAL);

    /// Brand for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const BRAND: Attribute<crate::StringValue> = Attribute::new(c"brand", INITIAL);

    /// Name for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const ID: Attribute<crate::StringValue> = Attribute::new(c"id", INITIAL);

    /// Name of cargo accessory for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CARGO_ACCESSORY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"cargo.accessory.id", INITIAL);

    /// Name of trailer chain type.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CHAIN_TYPE: Attribute<crate::StringValue> =
        Attribute::new(c"chain.type", ETS2_1_14_ATS_1_01);

    /// Name of trailer body type.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const BODY_TYPE: Attribute<crate::StringValue> =
        Attribute::new(c"body.type", ETS2_1_14_ATS_1_01);

    /// Vehicle license plate.
    ///
    /// Type: string
    pub const LICENSE_PLATE: Attribute<crate::StringValue> =
        Attribute::new(c"license.plate", ETS2_1_14_ATS_1_01);

    /// The id representing license plate country.
    ///
    /// Type: string
    pub const LICENSE_PLATE_COUNTRY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"license.plate.country.id", ETS2_1_14_ATS_1_01);

    /// The name of the license plate country.
    ///
    /// Type: string
    pub const LICENSE_PLATE_COUNTRY: Attribute<crate::StringValue> =
        Attribute::new(c"license.plate.country", ETS2_1_14_ATS_1_01);

    /// Name for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const NAME: Attribute<crate::StringValue> = Attribute::new(c"name", INITIAL);

    /// Fuel tank capacity in litres.
    ///
    /// Type: float
    pub const FUEL_CAPACITY: Attribute<f32> = Attribute::new(c"fuel.capacity", INITIAL);

    /// Fraction of the fuel capacity below which
    /// is activated the fuel warning.
    ///
    /// Type: float
    pub const FUEL_WARNING_FACTOR: Attribute<f32> = Attribute::new(c"fuel.warning.factor", INITIAL);

    /// `AdBlue` tank capacity in litres.
    ///
    /// Type: float
    pub const ADBLUE_CAPACITY: Attribute<f32> =
        Attribute::new(c"adblue.capacity", ETS2_1_12_ATS_1_00);

    /// Fraction of the adblue capacity below which
    /// is activated the adblue warning.
    ///
    /// Type: float
    pub const ADBLUE_WARNING_FACTOR: Attribute<f32> =
        Attribute::new(c"adblue.warning.factor", ETS2_1_12_ATS_1_00);

    /// Pressure of the air in the tank below which
    /// the warning activates.
    ///
    /// Type: float
    pub const AIR_PRESSURE_WARNING: Attribute<f32> =
        Attribute::new(c"brake.air.pressure.warning", INITIAL);

    /// Pressure of the air in the tank below which
    /// the emergency brakes activate.
    ///
    /// Type: float
    pub const AIR_PRESSURE_EMERGENCY: Attribute<f32> =
        Attribute::new(c"brake.air.pressure.emergency", ETS2_1_01_ATS_1_00);

    /// Pressure of the oil below which the warning activates.
    ///
    /// Type: float
    pub const OIL_PRESSURE_WARNING: Attribute<f32> =
        Attribute::new(c"oil.pressure.warning", INITIAL);

    /// Temperature of the water above which the warning activates.
    ///
    /// Type: float
    pub const WATER_TEMPERATURE_WARNING: Attribute<f32> =
        Attribute::new(c"water.temperature.warning", INITIAL);

    /// Voltage of the battery below which the warning activates.
    ///
    /// Type: float
    pub const BATTERY_VOLTAGE_WARNING: Attribute<f32> =
        Attribute::new(c"battery.voltage.warning", INITIAL);

    /// Maximum rpm value.
    ///
    /// Type: float
    pub const RPM_LIMIT: Attribute<f32> = Attribute::new(c"rpm.limit", INITIAL);

    /// Number of forward gears on undamaged truck.
    ///
    /// Type: u32
    pub const FORWARD_GEAR_COUNT: Attribute<u32> = Attribute::new(c"gears.forward", INITIAL);

    /// Number of reversee gears on undamaged truck.
    ///
    /// Type: u32
    pub const REVERSE_GEAR_COUNT: Attribute<u32> = Attribute::new(c"gears.reverse", INITIAL);

    /// Differential ratio of the truck.
    ///
    /// Type: float
    pub const DIFFERENTIAL_RATIO: Attribute<f32> =
        Attribute::new(c"differential.ratio", ETS2_1_12_ATS_1_00);

    /// Number of steps in the retarder.
    ///
    /// Set to zero if retarder is not mounted to the truck.
    ///
    /// Type: u32
    pub const RETARDER_STEP_COUNT: Attribute<u32> = Attribute::new(c"retarder.steps", INITIAL);

    /// Forward transmission ratios.
    ///
    /// Type: indexed float
    pub const FORWARD_RATIO: Attribute<f32> =
        Attribute::indexed(c"forward.ratio", ETS2_1_12_ATS_1_00);

    /// Reverse transmission ratios.
    ///
    /// Type: indexed float
    pub const REVERSE_RATIO: Attribute<f32> =
        Attribute::indexed(c"reverse.ratio", ETS2_1_12_ATS_1_00);

    /// Position of the cabin in the vehicle space.
    ///
    /// This is position of the joint around which the cabin rotates.
    /// This attribute might be not present if the vehicle does not
    /// have a separate cabin.
    ///
    /// Type: fvector
    pub const CABIN_POSITION: Attribute<crate::FVector> =
        Attribute::new(c"cabin.position", INITIAL);

    /// Default position of the head in the cabin space.
    ///
    /// Type: fvector
    pub const HEAD_POSITION: Attribute<crate::FVector> = Attribute::new(c"head.position", INITIAL);

    /// Position of the trailer connection hook in vehicle
    /// space.
    ///
    /// Type: fvector
    pub const HOOK_POSITION: Attribute<crate::FVector> = Attribute::new(c"hook.position", INITIAL);

    /// Number of wheels
    ///
    /// Type: u32
    pub const WHEEL_COUNT: Attribute<u32> = Attribute::new(c"wheels.count", INITIAL);

    /// Position of respective wheels in the vehicle space.
    ///
    /// Type: indexed fvector
    pub const WHEEL_POSITION: Attribute<crate::FVector> =
        Attribute::indexed(c"wheel.position", INITIAL);

    /// Is the wheel steerable?
    ///
    /// Type: indexed bool
    pub const WHEEL_STEERABLE: Attribute<bool> = Attribute::indexed(c"wheel.steerable", INITIAL);

    /// Is the wheel physicaly simulated?
    ///
    /// Type: indexed bool
    pub const WHEEL_SIMULATED: Attribute<bool> = Attribute::indexed(c"wheel.simulated", INITIAL);

    /// Radius of the wheel
    ///
    /// Type: indexed float
    pub const WHEEL_RADIUS: Attribute<f32> = Attribute::indexed(c"wheel.radius", INITIAL);

    /// Is the wheel powered?
    ///
    /// Type: indexed bool
    pub const WHEEL_POWERED: Attribute<bool> =
        Attribute::indexed(c"wheel.powered", ETS2_1_10_ATS_1_00);

    /// Is the wheel liftable?
    ///
    /// Type: indexed bool
    pub const WHEEL_LIFTABLE: Attribute<bool> =
        Attribute::indexed(c"wheel.liftable", ETS2_1_10_ATS_1_00);

    /// Number of selectors (e.g. range/splitter toggles).
    ///
    /// Type: u32
    pub const SELECTOR_COUNT: Attribute<u32> = Attribute::new(c"selector.count", INITIAL);

    /// Gear selected when requirements for this h-shifter slot are meet.
    ///
    /// Type: indexed s32
    pub const SLOT_GEAR: Attribute<i32> = Attribute::indexed(c"slot.gear", INITIAL);

    /// Position of h-shifter handle.
    ///
    /// Zero corresponds to neutral position. Mapping to physical position of
    /// the handle depends on input setup.
    ///
    /// Type: indexed u32
    pub const SLOT_HANDLE_POSITION: Attribute<u32> =
        Attribute::indexed(c"slot.handle.position", INITIAL);

    /// Bitmask of required on/off state of selectors.
    ///
    /// Only first `selector_count` bits are relevant.
    ///
    /// Type: indexed u32
    pub const SLOT_SELECTORS: Attribute<u32> = Attribute::indexed(c"slot.selectors", INITIAL);

    /// Type of the shifter.
    ///
    /// One from `SCS_SHIFTER_TYPE`_* values.
    ///
    /// Type: string
    pub const SHIFTER_TYPE: Attribute<crate::StringValue> =
        Attribute::new(c"shifter.type", INITIAL);

    /// Id of the cargo for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CARGO_ID: Attribute<crate::StringValue> =
        Attribute::new(c"cargo.id", ETS2_1_09_ATS_1_00);

    /// Name of the cargo for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const CARGO: Attribute<crate::StringValue> = Attribute::new(c"cargo", ETS2_1_09_ATS_1_00);

    /// Mass of the cargo in kilograms.
    ///
    /// Type: float
    pub const CARGO_MASS: Attribute<f32> = Attribute::new(c"cargo.mass", ETS2_1_09_ATS_1_00);

    /// Mass of the single unit of the cargo in kilograms.
    ///
    /// Type: float
    pub const CARGO_UNIT_MASS: Attribute<f32> =
        Attribute::new(c"cargo.unit.mass", ETS2_1_14_ATS_1_01);

    /// How many units of the cargo the job consist of.
    ///
    /// Type: u32
    pub const CARGO_UNIT_COUNT: Attribute<u32> =
        Attribute::new(c"cargo.unit.count", ETS2_1_14_ATS_1_01);

    /// Id of the destination city for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const DESTINATION_CITY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"destination.city.id", ETS2_1_09_ATS_1_00);

    /// Name of the destination city for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const DESTINATION_CITY: Attribute<crate::StringValue> =
        Attribute::new(c"destination.city", ETS2_1_09_ATS_1_00);

    /// Id of the destination company for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const DESTINATION_COMPANY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"destination.company.id", ETS2_1_09_ATS_1_00);

    /// Name of the destination company for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const DESTINATION_COMPANY: Attribute<crate::StringValue> =
        Attribute::new(c"destination.company", ETS2_1_09_ATS_1_00);

    /// Id of the source city for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const SOURCE_CITY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"source.city.id", ETS2_1_09_ATS_1_00);

    /// Name of the source city for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const SOURCE_CITY: Attribute<crate::StringValue> =
        Attribute::new(c"source.city", ETS2_1_09_ATS_1_00);

    /// Id of the source company for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const SOURCE_COMPANY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"source.company.id", ETS2_1_09_ATS_1_00);

    /// Name of the source company for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const SOURCE_COMPANY: Attribute<crate::StringValue> =
        Attribute::new(c"source.company", ETS2_1_09_ATS_1_00);

    /// Reward in internal game-specific currency.
    ///
    /// For detailed information about the currency see "Game specific units"
    /// documentation in `scssdk_telemetry`_<`game_id>.h`
    ///
    /// Type: u64
    pub const INCOME: Attribute<u64> = Attribute::new(c"income", ETS2_1_09_ATS_1_00);

    /// Absolute in-game time of end of job delivery window.
    ///
    /// Delivering the job after this time will cause it be late.
    ///
    /// See `SCS_TELEMETRY_CHANNEL_game_time` for more info about absolute time.
    /// Time remaining for delivery can be obtained like (`delivery_time` - `game_time`).
    ///
    /// Type: u32
    pub const DELIVERY_TIME: Attribute<u32> = Attribute::new(c"delivery.time", ETS2_1_09_ATS_1_00);

    /// Planned job distance in simulated kilometers.
    ///
    /// Does not include distance driven using ferry.
    ///
    /// Type: u32
    pub const PLANNED_DISTANCE_KM: Attribute<u32> =
        Attribute::new(c"planned_distance.km", ETS2_1_15_ATS_1_02);

    /// Is cargo loaded on the trailer?
    ///
    /// For non cargo market jobs this is always true
    ///
    /// Type: bool
    pub const IS_CARGO_LOADED: Attribute<bool> =
        Attribute::new(c"cargo.loaded", ETS2_1_14_ATS_1_01);

    /// The job market this job is from.
    ///
    /// The value is a string representing the type of the job market.
    /// Possible values:
    /// - `cargo_market`
    /// - `quick_job`
    /// - `freight_market`
    /// - `external_contracts`
    /// - `external_market`
    ///
    /// Type: string
    pub const JOB_MARKET: Attribute<crate::StringValue> =
        Attribute::new(c"job.market", ETS2_1_14_ATS_1_01);

    /// Flag indicating that the job is special transport job.
    ///
    /// Type: bool
    pub const SPECIAL_JOB: Attribute<bool> = Attribute::new(c"is.special.job", ETS2_1_14_ATS_1_01);

    /// Every public configuration attribute from the SDK 1.14 headers.
    ///
    /// Type erasure exists only to make the heterogeneous catalog enumerable.
    /// Each entry retains its exact SCS value type and indexed/scalar lookup
    /// mode; callback code should continue using the individual typed constants
    /// for decoding.
    pub const ALL: [AnyAttribute; COUNT] = [
        BRAND_ID.erase(),
        BRAND.erase(),
        ID.erase(),
        CARGO_ACCESSORY_ID.erase(),
        CHAIN_TYPE.erase(),
        BODY_TYPE.erase(),
        LICENSE_PLATE.erase(),
        LICENSE_PLATE_COUNTRY_ID.erase(),
        LICENSE_PLATE_COUNTRY.erase(),
        NAME.erase(),
        FUEL_CAPACITY.erase(),
        FUEL_WARNING_FACTOR.erase(),
        ADBLUE_CAPACITY.erase(),
        ADBLUE_WARNING_FACTOR.erase(),
        AIR_PRESSURE_WARNING.erase(),
        AIR_PRESSURE_EMERGENCY.erase(),
        OIL_PRESSURE_WARNING.erase(),
        WATER_TEMPERATURE_WARNING.erase(),
        BATTERY_VOLTAGE_WARNING.erase(),
        RPM_LIMIT.erase(),
        FORWARD_GEAR_COUNT.erase(),
        REVERSE_GEAR_COUNT.erase(),
        DIFFERENTIAL_RATIO.erase(),
        RETARDER_STEP_COUNT.erase(),
        FORWARD_RATIO.erase(),
        REVERSE_RATIO.erase(),
        CABIN_POSITION.erase(),
        HEAD_POSITION.erase(),
        HOOK_POSITION.erase(),
        WHEEL_COUNT.erase(),
        WHEEL_POSITION.erase(),
        WHEEL_STEERABLE.erase(),
        WHEEL_SIMULATED.erase(),
        WHEEL_RADIUS.erase(),
        WHEEL_POWERED.erase(),
        WHEEL_LIFTABLE.erase(),
        SELECTOR_COUNT.erase(),
        SLOT_GEAR.erase(),
        SLOT_HANDLE_POSITION.erase(),
        SLOT_SELECTORS.erase(),
        SHIFTER_TYPE.erase(),
        CARGO_ID.erase(),
        CARGO.erase(),
        CARGO_MASS.erase(),
        CARGO_UNIT_MASS.erase(),
        CARGO_UNIT_COUNT.erase(),
        DESTINATION_CITY_ID.erase(),
        DESTINATION_CITY.erase(),
        DESTINATION_COMPANY_ID.erase(),
        DESTINATION_COMPANY.erase(),
        SOURCE_CITY_ID.erase(),
        SOURCE_CITY.erase(),
        SOURCE_COMPANY_ID.erase(),
        SOURCE_COMPANY.erase(),
        INCOME.erase(),
        DELIVERY_TIME.erase(),
        PLANNED_DISTANCE_KM.erase(),
        IS_CARGO_LOADED.erase(),
        JOB_MARKET.erase(),
        SPECIAL_JOB.erase(),
    ];
}

/// Configuration-group membership for every typed attribute.
///
/// The common header declares attribute names in one flat namespace, while
/// configuration callbacks deliver subsets of those names under a specific
/// group ID. This catalog records that missing relational layer. It is grouped
/// in configuration-header order; attributes within each group follow the
/// documented list and then the attribute declaration order for official
/// values which the prose list omits (notably transmission and wheel data).
pub mod associations {
    use super::{
        ETS2_1_01_ATS_1_00, ETS2_1_09_ATS_1_00, ETS2_1_10_ATS_1_00, ETS2_1_12_ATS_1_00,
        ETS2_1_14_ATS_1_01, ETS2_1_15_ATS_1_02, INITIAL, attributes, ids,
    };
    use crate::{
        Attribute, ConfigurationAttributeAssociation, ConfigurationId, GameSchemaAvailability,
        SdkValue,
    };

    /// Total number of configuration-to-attribute relationships.
    ///
    /// This is larger than the 60-name attribute catalog because identifiers
    /// such as `id`, `brand`, license-plate data, hook position, and wheel
    /// geometry occur in more than one configuration group.
    pub const COUNT: usize = 71;

    const fn association<T: SdkValue>(
        configuration: ConfigurationId,
        attribute: Attribute<T>,
        availability: GameSchemaAvailability,
    ) -> ConfigurationAttributeAssociation {
        ConfigurationAttributeAssociation::new(configuration, attribute, availability)
    }

    /// Attributes carried by the `substances` configuration.
    pub const SUBSTANCES: [ConfigurationAttributeAssociation; 1] =
        [association(ids::SUBSTANCES, attributes::ID, INITIAL)];

    /// Attributes carried by the static `controls` configuration.
    pub const CONTROLS: [ConfigurationAttributeAssociation; 1] = [association(
        ids::CONTROLS,
        attributes::SHIFTER_TYPE,
        INITIAL,
    )];

    /// Attributes carried by the H-shifter configuration.
    pub const HSHIFTER: [ConfigurationAttributeAssociation; 4] = [
        association(ids::HSHIFTER, attributes::SELECTOR_COUNT, INITIAL),
        association(ids::HSHIFTER, attributes::SLOT_GEAR, INITIAL),
        association(ids::HSHIFTER, attributes::SLOT_HANDLE_POSITION, INITIAL),
        association(ids::HSHIFTER, attributes::SLOT_SELECTORS, INITIAL),
    ];

    /// Attributes carried by the static truck configuration.
    ///
    /// The header's short "Supported attributes" prose predates several flat
    /// attribute macros and was not kept exhaustive. The game-specific schema
    /// changelog explicitly associates transmission information with the truck,
    /// while the wheel descriptors use the truck wheel index domain. Those
    /// official relationships are retained here instead of repeating the
    /// incomplete prose list verbatim.
    pub const TRUCK: [ConfigurationAttributeAssociation; 33] = [
        association(ids::TRUCK, attributes::BRAND_ID, INITIAL),
        association(ids::TRUCK, attributes::BRAND, INITIAL),
        association(ids::TRUCK, attributes::ID, INITIAL),
        association(ids::TRUCK, attributes::NAME, INITIAL),
        association(ids::TRUCK, attributes::FUEL_CAPACITY, INITIAL),
        association(ids::TRUCK, attributes::FUEL_WARNING_FACTOR, INITIAL),
        association(ids::TRUCK, attributes::ADBLUE_CAPACITY, ETS2_1_12_ATS_1_00),
        association(
            ids::TRUCK,
            attributes::ADBLUE_WARNING_FACTOR,
            ETS2_1_12_ATS_1_00,
        ),
        association(ids::TRUCK, attributes::AIR_PRESSURE_WARNING, INITIAL),
        association(
            ids::TRUCK,
            attributes::AIR_PRESSURE_EMERGENCY,
            ETS2_1_01_ATS_1_00,
        ),
        association(ids::TRUCK, attributes::OIL_PRESSURE_WARNING, INITIAL),
        association(ids::TRUCK, attributes::WATER_TEMPERATURE_WARNING, INITIAL),
        association(ids::TRUCK, attributes::BATTERY_VOLTAGE_WARNING, INITIAL),
        association(ids::TRUCK, attributes::RPM_LIMIT, INITIAL),
        association(ids::TRUCK, attributes::FORWARD_GEAR_COUNT, INITIAL),
        association(ids::TRUCK, attributes::REVERSE_GEAR_COUNT, INITIAL),
        association(
            ids::TRUCK,
            attributes::DIFFERENTIAL_RATIO,
            ETS2_1_12_ATS_1_00,
        ),
        association(ids::TRUCK, attributes::RETARDER_STEP_COUNT, INITIAL),
        association(ids::TRUCK, attributes::FORWARD_RATIO, ETS2_1_12_ATS_1_00),
        association(ids::TRUCK, attributes::REVERSE_RATIO, ETS2_1_12_ATS_1_00),
        association(ids::TRUCK, attributes::CABIN_POSITION, INITIAL),
        association(ids::TRUCK, attributes::HEAD_POSITION, INITIAL),
        association(ids::TRUCK, attributes::HOOK_POSITION, INITIAL),
        association(ids::TRUCK, attributes::LICENSE_PLATE, ETS2_1_14_ATS_1_01),
        association(
            ids::TRUCK,
            attributes::LICENSE_PLATE_COUNTRY,
            ETS2_1_14_ATS_1_01,
        ),
        association(
            ids::TRUCK,
            attributes::LICENSE_PLATE_COUNTRY_ID,
            ETS2_1_14_ATS_1_01,
        ),
        association(ids::TRUCK, attributes::WHEEL_COUNT, INITIAL),
        association(ids::TRUCK, attributes::WHEEL_POSITION, INITIAL),
        association(ids::TRUCK, attributes::WHEEL_STEERABLE, INITIAL),
        association(ids::TRUCK, attributes::WHEEL_SIMULATED, INITIAL),
        association(ids::TRUCK, attributes::WHEEL_RADIUS, INITIAL),
        association(ids::TRUCK, attributes::WHEEL_POWERED, ETS2_1_10_ATS_1_00),
        association(ids::TRUCK, attributes::WHEEL_LIFTABLE, ETS2_1_10_ATS_1_00),
    ];

    /// Attributes carried by the legacy `trailer` configuration and by each
    /// numbered trailer configuration once that namespace is available.
    ///
    /// For `trailer.[index]`, consumers must additionally apply
    /// [`crate::game::capabilities::MULTI_TRAILER`]. The relationship minima
    /// below still describe when each attribute joined the trailer schema, so
    /// the legacy unnumbered alias is represented accurately as well.
    pub const TRAILER: [ConfigurationAttributeAssociation; 13] = [
        association(ids::TRAILER, attributes::ID, INITIAL),
        association(ids::TRAILER, attributes::CARGO_ACCESSORY_ID, INITIAL),
        association(ids::TRAILER, attributes::HOOK_POSITION, INITIAL),
        association(ids::TRAILER, attributes::BRAND_ID, ETS2_1_14_ATS_1_01),
        association(ids::TRAILER, attributes::BRAND, ETS2_1_14_ATS_1_01),
        association(ids::TRAILER, attributes::NAME, ETS2_1_14_ATS_1_01),
        association(ids::TRAILER, attributes::CHAIN_TYPE, ETS2_1_14_ATS_1_01),
        association(ids::TRAILER, attributes::BODY_TYPE, ETS2_1_14_ATS_1_01),
        association(ids::TRAILER, attributes::LICENSE_PLATE, ETS2_1_14_ATS_1_01),
        association(
            ids::TRAILER,
            attributes::LICENSE_PLATE_COUNTRY,
            ETS2_1_14_ATS_1_01,
        ),
        association(
            ids::TRAILER,
            attributes::LICENSE_PLATE_COUNTRY_ID,
            ETS2_1_14_ATS_1_01,
        ),
        association(ids::TRAILER, attributes::WHEEL_COUNT, INITIAL),
        association(ids::TRAILER, attributes::WHEEL_POSITION, INITIAL),
    ];

    /// Attributes carried by the active-job configuration.
    pub const JOB: [ConfigurationAttributeAssociation; 19] = [
        association(ids::JOB, attributes::CARGO_ID, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::CARGO, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::CARGO_MASS, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::CARGO_UNIT_MASS, ETS2_1_14_ATS_1_01),
        association(ids::JOB, attributes::CARGO_UNIT_COUNT, ETS2_1_14_ATS_1_01),
        association(
            ids::JOB,
            attributes::DESTINATION_CITY_ID,
            ETS2_1_09_ATS_1_00,
        ),
        association(ids::JOB, attributes::DESTINATION_CITY, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::SOURCE_CITY_ID, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::SOURCE_CITY, ETS2_1_09_ATS_1_00),
        association(
            ids::JOB,
            attributes::DESTINATION_COMPANY_ID,
            ETS2_1_09_ATS_1_00,
        ),
        association(
            ids::JOB,
            attributes::DESTINATION_COMPANY,
            ETS2_1_09_ATS_1_00,
        ),
        association(ids::JOB, attributes::SOURCE_COMPANY_ID, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::SOURCE_COMPANY, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::INCOME, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::DELIVERY_TIME, ETS2_1_09_ATS_1_00),
        association(ids::JOB, attributes::IS_CARGO_LOADED, ETS2_1_14_ATS_1_01),
        association(ids::JOB, attributes::JOB_MARKET, ETS2_1_14_ATS_1_01),
        association(ids::JOB, attributes::SPECIAL_JOB, ETS2_1_14_ATS_1_01),
        association(
            ids::JOB,
            attributes::PLANNED_DISTANCE_KM,
            ETS2_1_15_ATS_1_02,
        ),
    ];

    /// Every official configuration-to-attribute relationship in SDK 1.14.
    pub const ALL: [ConfigurationAttributeAssociation; COUNT] = [
        SUBSTANCES[0],
        CONTROLS[0],
        HSHIFTER[0],
        HSHIFTER[1],
        HSHIFTER[2],
        HSHIFTER[3],
        TRUCK[0],
        TRUCK[1],
        TRUCK[2],
        TRUCK[3],
        TRUCK[4],
        TRUCK[5],
        TRUCK[6],
        TRUCK[7],
        TRUCK[8],
        TRUCK[9],
        TRUCK[10],
        TRUCK[11],
        TRUCK[12],
        TRUCK[13],
        TRUCK[14],
        TRUCK[15],
        TRUCK[16],
        TRUCK[17],
        TRUCK[18],
        TRUCK[19],
        TRUCK[20],
        TRUCK[21],
        TRUCK[22],
        TRUCK[23],
        TRUCK[24],
        TRUCK[25],
        TRUCK[26],
        TRUCK[27],
        TRUCK[28],
        TRUCK[29],
        TRUCK[30],
        TRUCK[31],
        TRUCK[32],
        TRAILER[0],
        TRAILER[1],
        TRAILER[2],
        TRAILER[3],
        TRAILER[4],
        TRAILER[5],
        TRAILER[6],
        TRAILER[7],
        TRAILER[8],
        TRAILER[9],
        TRAILER[10],
        TRAILER[11],
        TRAILER[12],
        JOB[0],
        JOB[1],
        JOB[2],
        JOB[3],
        JOB[4],
        JOB[5],
        JOB[6],
        JOB[7],
        JOB[8],
        JOB[9],
        JOB[10],
        JOB[11],
        JOB[12],
        JOB[13],
        JOB[14],
        JOB[15],
        JOB[16],
        JOB[17],
        JOB[18],
    ];
}

/// Documented values of the shifter.type configuration attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShifterType {
    /// Simplified arcade transmission.
    Arcade,
    /// Automatic transmission.
    Automatic,
    /// Manual transmission controlled without an H-shifter.
    Manual,
    /// Physical H-pattern shifter.
    HShifter,
}

impl ShifterType {
    /// Number of shifter values declared by the official header.
    pub const COUNT: usize = 4;

    /// Every documented shifter type in header order.
    pub const ALL: [Self; Self::COUNT] =
        [Self::Arcade, Self::Automatic, Self::Manual, Self::HShifter];

    /// First game schemas which provide the shifter-type configuration value.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        let _ = self;
        INITIAL
    }

    /// Canonical Rust string used by the SDK.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Arcade => "arcade",
            Self::Automatic => "automatic",
            Self::Manual => "manual",
            Self::HShifter => "hshifter",
        }
    }

    #[must_use]
    pub const fn as_c_str(self) -> &'static CStr {
        match self {
            Self::Arcade => c"arcade",
            Self::Automatic => c"automatic",
            Self::Manual => c"manual",
            Self::HShifter => c"hshifter",
        }
    }

    #[must_use]
    pub fn from_c_str(value: &CStr) -> Option<Self> {
        value.to_str().ok()?.parse().ok()
    }
}

impl FromStr for ShifterType {
    type Err = UnknownStringValue;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "arcade" => Ok(Self::Arcade),
            "automatic" => Ok(Self::Automatic),
            "manual" => Ok(Self::Manual),
            "hshifter" => Ok(Self::HShifter),
            _ => Err(UnknownStringValue),
        }
    }
}

/// Documented values of the `job.market` configuration attribute.
///
/// Unknown future strings remain available through the generic typed string
/// attribute API. This enum intentionally recognizes only the values listed by
/// the vendored SDK 1.14 header, allowing callers to distinguish a known market
/// from a future additive value without losing the original text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobMarket {
    CargoMarket,
    QuickJob,
    FreightMarket,
    ExternalContracts,
    ExternalMarket,
}

impl JobMarket {
    /// Number of job-market strings documented by SDK 1.14.
    pub const COUNT: usize = 5;

    /// Every documented job-market value in header order.
    pub const ALL: [Self; Self::COUNT] = [
        Self::CargoMarket,
        Self::QuickJob,
        Self::FreightMarket,
        Self::ExternalContracts,
        Self::ExternalMarket,
    ];

    /// First game schemas which provide the `job.market` value.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        let _ = self;
        ETS2_1_14_ATS_1_01
    }

    /// Canonical Rust string used by the SDK.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CargoMarket => "cargo_market",
            Self::QuickJob => "quick_job",
            Self::FreightMarket => "freight_market",
            Self::ExternalContracts => "external_contracts",
            Self::ExternalMarket => "external_market",
        }
    }

    /// Canonical NUL-terminated string used by the C SDK.
    #[must_use]
    pub const fn as_c_str(self) -> &'static CStr {
        match self {
            Self::CargoMarket => c"cargo_market",
            Self::QuickJob => c"quick_job",
            Self::FreightMarket => c"freight_market",
            Self::ExternalContracts => c"external_contracts",
            Self::ExternalMarket => c"external_market",
        }
    }

    /// Parses one canonical job-market value from an SDK C string.
    #[must_use]
    pub fn from_c_str(value: &CStr) -> Option<Self> {
        value.to_str().ok()?.parse().ok()
    }
}

impl FromStr for JobMarket {
    type Err = UnknownStringValue;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "cargo_market" => Ok(Self::CargoMarket),
            "quick_job" => Ok(Self::QuickJob),
            "freight_market" => Ok(Self::FreightMarket),
            "external_contracts" => Ok(Self::ExternalContracts),
            "external_market" => Ok(Self::ExternalMarket),
            _ => Err(UnknownStringValue),
        }
    }
}

const _: [(); 6] = [(); ids::COUNT];
const _: [(); 60] = [(); attributes::COUNT];
const _: [(); 71] = [(); associations::COUNT];
const _: [(); 4] = [(); ShifterType::COUNT];
const _: [(); 5] = [(); JobMarket::COUNT];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameSchemaVersion;

    fn assert_since<T: crate::SdkValue>(
        attribute: crate::Attribute<T>,
        ets2: GameSchemaVersion,
        ats: GameSchemaVersion,
    ) {
        assert_eq!(attribute.availability().available_since_ets2(), Some(ets2));
        assert_eq!(attribute.availability().available_since_ats(), Some(ats));
    }

    #[test]
    fn typed_catalog_counts_match_the_raw_sdk_catalog() {
        assert_eq!(ids::COUNT, crate::sys::configuration::ids::COUNT);
        assert_eq!(
            attributes::COUNT,
            crate::sys::configuration::attributes::COUNT
        );
        assert_eq!(ids::COUNT, 6);
        assert_eq!(attributes::COUNT, 60);
        assert_eq!(MAX_TRAILERS, crate::sys::configuration::MAX_TRAILERS);
        assert_eq!(associations::COUNT, 71);
        assert_eq!(associations::SUBSTANCES.len(), 1);
        assert_eq!(associations::CONTROLS.len(), 1);
        assert_eq!(associations::HSHIFTER.len(), 4);
        assert_eq!(associations::TRUCK.len(), 33);
        assert_eq!(associations::TRAILER.len(), 13);
        assert_eq!(associations::JOB.len(), 19);
    }

    #[test]
    fn association_catalog_has_complete_membership_without_duplicate_pairs() {
        for association in associations::ALL {
            assert!(
                ids::ALL.contains(&association.configuration()),
                "association uses unknown configuration: {:?}",
                association.configuration().name()
            );
            assert!(
                attributes::ALL.contains(&association.attribute()),
                "association uses unknown attribute: {:?}",
                association.attribute().name()
            );
        }

        for attribute in attributes::ALL {
            assert!(
                associations::ALL
                    .iter()
                    .any(|association| association.attribute() == attribute),
                "attribute has no configuration membership: {:?}",
                attribute.name()
            );
        }

        for (position, association) in associations::ALL.iter().enumerate() {
            assert!(
                associations::ALL[..position].iter().all(|earlier| {
                    earlier.configuration() != association.configuration()
                        || earlier.attribute() != association.attribute()
                }),
                "duplicate configuration association: {:?} -> {:?}",
                association.configuration().name(),
                association.attribute().name()
            );
        }
    }

    #[test]
    fn association_catalog_preserves_configuration_group_order() {
        let groups = [
            (ids::SUBSTANCES, 0..1),
            (ids::CONTROLS, 1..2),
            (ids::HSHIFTER, 2..6),
            (ids::TRUCK, 6..39),
            (ids::TRAILER, 39..52),
            (ids::JOB, 52..71),
        ];

        for (configuration, range) in groups {
            assert!(
                associations::ALL[range]
                    .iter()
                    .all(|association| association.configuration() == configuration)
            );
        }

        assert_eq!(
            associations::SUBSTANCES[0].attribute(),
            attributes::ID.erase()
        );
        assert_eq!(
            associations::CONTROLS[0].attribute(),
            attributes::SHIFTER_TYPE.erase()
        );
        assert_eq!(
            associations::TRUCK[16].attribute(),
            attributes::DIFFERENTIAL_RATIO.erase()
        );
        assert_eq!(
            associations::TRAILER[3].attribute(),
            attributes::BRAND_ID.erase()
        );
        assert_eq!(
            associations::JOB[18].attribute(),
            attributes::PLANNED_DISTANCE_KM.erase()
        );
    }

    #[test]
    fn association_availability_is_never_earlier_than_its_descriptors() {
        for association in associations::ALL {
            let availability = association.availability();
            let configuration = association.configuration().availability();
            let attribute = association.attribute().availability();

            assert!(matches!(
                (
                    availability.available_since_ets2(),
                    configuration.available_since_ets2(),
                    attribute.available_since_ets2()
                ),
                (Some(relation), Some(group), Some(value)) if relation >= group && relation >= value
            ));
            assert!(matches!(
                (
                    availability.available_since_ats(),
                    configuration.available_since_ats(),
                    attribute.available_since_ats()
                ),
                (Some(relation), Some(group), Some(value)) if relation >= group && relation >= value
            ));
        }
    }

    #[test]
    fn shared_attribute_relationships_keep_independent_history() {
        assert_eq!(
            associations::TRUCK[0].availability(),
            INITIAL,
            "truck brand_id belongs to the initial truck schema"
        );
        assert_eq!(
            associations::TRAILER[3].availability(),
            game::capabilities::MULTI_TRAILER,
            "trailer brand_id arrived with the multi-trailer schema"
        );
        assert_eq!(
            associations::TRAILER[0].availability(),
            INITIAL,
            "the legacy trailer id relationship predates numbered trailers"
        );
        assert_eq!(
            associations::JOB[3].availability(),
            game::capabilities::MULTI_TRAILER,
            "cargo unit mass first appears in the same historical archive"
        );
        assert_eq!(
            game::capabilities::MULTI_TRAILER.available_since_ets2(),
            Some(game::ets2::V1_14)
        );
        assert_eq!(
            game::capabilities::MULTI_TRAILER.available_since_ats(),
            Some(game::ats::V1_01)
        );
    }

    #[test]
    fn every_typed_descriptor_matches_the_raw_header_catalog() {
        for (descriptor, raw_name) in ids::ALL.iter().zip(crate::sys::configuration::ids::ALL) {
            assert_eq!(descriptor.name().to_bytes_with_nul(), raw_name);
        }
        for (descriptor, raw_name) in attributes::ALL
            .iter()
            .zip(crate::sys::configuration::attributes::ALL)
        {
            assert_eq!(descriptor.name().to_bytes_with_nul(), raw_name);
        }

        for (position, descriptor) in attributes::ALL.iter().enumerate() {
            assert!(
                attributes::ALL[..position]
                    .iter()
                    .all(|earlier| earlier.name() != descriptor.name()),
                "duplicate configuration attribute: {:?}",
                descriptor.name()
            );
        }
        assert_eq!(
            attributes::ALL
                .iter()
                .filter(|attribute| attribute.is_indexed())
                .count(),
            11
        );
    }

    #[test]
    fn representative_descriptors_preserve_header_metadata() {
        assert_eq!(ids::JOB.name(), c"job");
        assert_eq!(attributes::CARGO_MASS.name(), c"cargo.mass");
        assert_eq!(
            attributes::CARGO_MASS.value_type(),
            crate::sys::SCS_VALUE_TYPE_FLOAT
        );
        assert!(!attributes::CARGO_MASS.is_indexed());
        assert_eq!(attributes::WHEEL_POSITION.name(), c"wheel.position");
        assert!(attributes::WHEEL_POSITION.is_indexed());
    }

    #[test]
    fn configuration_availability_follows_official_game_schema_history() {
        assert_eq!(
            ids::JOB.availability().available_since_ets2(),
            Some(game::ets2::V1_09)
        );
        assert_eq!(
            ids::JOB.availability().available_since_ats(),
            Some(game::ats::V1_00)
        );
        assert_since(attributes::CARGO_ID, game::ets2::V1_09, game::ats::V1_00);
        assert_since(attributes::INCOME, game::ets2::V1_09, game::ats::V1_00);

        assert_since(
            attributes::AIR_PRESSURE_EMERGENCY,
            game::ets2::V1_01,
            game::ats::V1_00,
        );
        for attribute in [attributes::WHEEL_POWERED, attributes::WHEEL_LIFTABLE] {
            assert_since(attribute, game::ets2::V1_10, game::ats::V1_00);
        }
        for attribute in [
            attributes::ADBLUE_CAPACITY,
            attributes::ADBLUE_WARNING_FACTOR,
            attributes::DIFFERENTIAL_RATIO,
            attributes::FORWARD_RATIO,
            attributes::REVERSE_RATIO,
        ] {
            assert_since(attribute, game::ets2::V1_12, game::ats::V1_00);
        }
        for attribute in [
            attributes::CHAIN_TYPE,
            attributes::BODY_TYPE,
            attributes::LICENSE_PLATE,
            attributes::LICENSE_PLATE_COUNTRY_ID,
            attributes::LICENSE_PLATE_COUNTRY,
        ] {
            assert_since(attribute, game::ets2::V1_14, game::ats::V1_01);
        }
        assert_since(
            attributes::CARGO_UNIT_MASS,
            game::ets2::V1_14,
            game::ats::V1_01,
        );
        assert_since(
            attributes::CARGO_UNIT_COUNT,
            game::ets2::V1_14,
            game::ats::V1_01,
        );
        assert_since(
            attributes::IS_CARGO_LOADED,
            game::ets2::V1_14,
            game::ats::V1_01,
        );
        assert_since(attributes::JOB_MARKET, game::ets2::V1_14, game::ats::V1_01);
        assert_since(attributes::SPECIAL_JOB, game::ets2::V1_14, game::ats::V1_01);
        assert_since(
            attributes::PLANNED_DISTANCE_KM,
            game::ets2::V1_15,
            game::ats::V1_02,
        );

        assert!(ids::ALL.iter().all(|descriptor| {
            descriptor.availability().available_since_ets2().is_some()
                && descriptor.availability().available_since_ats().is_some()
        }));
        assert!(attributes::ALL.iter().all(|descriptor| {
            descriptor.availability().available_since_ets2().is_some()
                && descriptor.availability().available_since_ats().is_some()
        }));
    }

    #[test]
    fn every_documented_shifter_type_round_trips() {
        assert_eq!(ShifterType::COUNT, 4);
        for value in ShifterType::ALL {
            assert_eq!(ShifterType::from_c_str(value.as_c_str()), Some(value));
            assert_eq!(value.as_str().parse(), Ok(value));
            assert_eq!(value.availability(), INITIAL);
        }
        assert_eq!(ShifterType::from_c_str(c"future-shifter"), None);
        assert_eq!(
            "future-shifter".parse::<ShifterType>(),
            Err(UnknownStringValue)
        );
    }

    #[test]
    fn every_documented_job_market_round_trips() {
        assert_eq!(JobMarket::COUNT, 5);
        for value in JobMarket::ALL {
            assert_eq!(JobMarket::from_c_str(value.as_c_str()), Some(value));
            assert_eq!(value.as_str().parse(), Ok(value));
            assert_eq!(value.availability(), ETS2_1_14_ATS_1_01);
        }
        assert_eq!(JobMarket::from_c_str(c"future_market"), None);
        assert_eq!(
            "future_market".parse::<JobMarket>(),
            Err(UnknownStringValue)
        );
    }
}
