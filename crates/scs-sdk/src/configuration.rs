//! Typed configuration catalog for the SCS Telemetry SDK.
//!
//! This module covers every public configuration ID, attribute, and
//! documented H-shifter value in the SDK 1.14 header bundle.

use core::ffi::CStr;

/// Maximum number of trailers represented by the SDK.
pub const MAX_TRAILERS: usize = 10;

/// Configuration group identifiers.
pub mod ids {
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
    pub const SUBSTANCES: ConfigurationId = ConfigurationId::new(c"substances");

    /// Static configuration of the controls.
    ///
    /// - `shifter_type`
    pub const CONTROLS: ConfigurationId = ConfigurationId::new(c"controls");

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
    pub const HSHIFTER: ConfigurationId = ConfigurationId::new(c"hshifter");

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
    pub const TRUCK: ConfigurationId = ConfigurationId::new(c"truck");

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
    pub const TRAILER: ConfigurationId = ConfigurationId::new(c"trailer");

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
    pub const JOB: ConfigurationId = ConfigurationId::new(c"job");

    /// Every public configuration group in SDK 1.14 header order.
    ///
    /// The trailer entry is the backward-compatible `trailer` identifier. The
    /// game additionally emits `trailer.0` through `trailer.9`; consumers must
    /// still select those numbered groups explicitly when they need them.
    pub const ALL: [ConfigurationId; COUNT] = [SUBSTANCES, CONTROLS, HSHIFTER, TRUCK, TRAILER, JOB];
}

/// Typed attributes carried by configuration events.
pub mod attributes {
    use crate::{AnyAttribute, Attribute};

    /// Number of official typed configuration attributes.
    pub const COUNT: usize = 60;

    /// Brand id for configuration purposes.
    ///
    /// Limited to C-identifier characters.
    ///
    /// Type: string
    pub const BRAND_ID: Attribute<crate::StringValue> = Attribute::new(c"brand_id");

    /// Brand for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const BRAND: Attribute<crate::StringValue> = Attribute::new(c"brand");

    /// Name for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const ID: Attribute<crate::StringValue> = Attribute::new(c"id");

    /// Name of cargo accessory for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CARGO_ACCESSORY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"cargo.accessory.id");

    /// Name of trailer chain type.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CHAIN_TYPE: Attribute<crate::StringValue> = Attribute::new(c"chain.type");

    /// Name of trailer body type.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const BODY_TYPE: Attribute<crate::StringValue> = Attribute::new(c"body.type");

    /// Vehicle license plate.
    ///
    /// Type: string
    pub const LICENSE_PLATE: Attribute<crate::StringValue> = Attribute::new(c"license.plate");

    /// The id representing license plate country.
    ///
    /// Type: string
    pub const LICENSE_PLATE_COUNTRY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"license.plate.country.id");

    /// The name of the license plate country.
    ///
    /// Type: string
    pub const LICENSE_PLATE_COUNTRY: Attribute<crate::StringValue> =
        Attribute::new(c"license.plate.country");

    /// Name for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const NAME: Attribute<crate::StringValue> = Attribute::new(c"name");

    /// Fuel tank capacity in litres.
    ///
    /// Type: float
    pub const FUEL_CAPACITY: Attribute<f32> = Attribute::new(c"fuel.capacity");

    /// Fraction of the fuel capacity below which
    /// is activated the fuel warning.
    ///
    /// Type: float
    pub const FUEL_WARNING_FACTOR: Attribute<f32> = Attribute::new(c"fuel.warning.factor");

    /// `AdBlue` tank capacity in litres.
    ///
    /// Type: float
    pub const ADBLUE_CAPACITY: Attribute<f32> = Attribute::new(c"adblue.capacity");

    /// Fraction of the adblue capacity below which
    /// is activated the adblue warning.
    ///
    /// Type: float
    pub const ADBLUE_WARNING_FACTOR: Attribute<f32> = Attribute::new(c"adblue.warning.factor");

    /// Pressure of the air in the tank below which
    /// the warning activates.
    ///
    /// Type: float
    pub const AIR_PRESSURE_WARNING: Attribute<f32> = Attribute::new(c"brake.air.pressure.warning");

    /// Pressure of the air in the tank below which
    /// the emergency brakes activate.
    ///
    /// Type: float
    pub const AIR_PRESSURE_EMERGENCY: Attribute<f32> =
        Attribute::new(c"brake.air.pressure.emergency");

    /// Pressure of the oil below which the warning activates.
    ///
    /// Type: float
    pub const OIL_PRESSURE_WARNING: Attribute<f32> = Attribute::new(c"oil.pressure.warning");

    /// Temperature of the water above which the warning activates.
    ///
    /// Type: float
    pub const WATER_TEMPERATURE_WARNING: Attribute<f32> =
        Attribute::new(c"water.temperature.warning");

    /// Voltage of the battery below which the warning activates.
    ///
    /// Type: float
    pub const BATTERY_VOLTAGE_WARNING: Attribute<f32> = Attribute::new(c"battery.voltage.warning");

    /// Maximum rpm value.
    ///
    /// Type: float
    pub const RPM_LIMIT: Attribute<f32> = Attribute::new(c"rpm.limit");

    /// Number of forward gears on undamaged truck.
    ///
    /// Type: u32
    pub const FORWARD_GEAR_COUNT: Attribute<u32> = Attribute::new(c"gears.forward");

    /// Number of reversee gears on undamaged truck.
    ///
    /// Type: u32
    pub const REVERSE_GEAR_COUNT: Attribute<u32> = Attribute::new(c"gears.reverse");

    /// Differential ratio of the truck.
    ///
    /// Type: float
    pub const DIFFERENTIAL_RATIO: Attribute<f32> = Attribute::new(c"differential.ratio");

    /// Number of steps in the retarder.
    ///
    /// Set to zero if retarder is not mounted to the truck.
    ///
    /// Type: u32
    pub const RETARDER_STEP_COUNT: Attribute<u32> = Attribute::new(c"retarder.steps");

    /// Forward transmission ratios.
    ///
    /// Type: indexed float
    pub const FORWARD_RATIO: Attribute<f32> = Attribute::indexed(c"forward.ratio");

    /// Reverse transmission ratios.
    ///
    /// Type: indexed float
    pub const REVERSE_RATIO: Attribute<f32> = Attribute::indexed(c"reverse.ratio");

    /// Position of the cabin in the vehicle space.
    ///
    /// This is position of the joint around which the cabin rotates.
    /// This attribute might be not present if the vehicle does not
    /// have a separate cabin.
    ///
    /// Type: fvector
    pub const CABIN_POSITION: Attribute<crate::FVector> = Attribute::new(c"cabin.position");

    /// Default position of the head in the cabin space.
    ///
    /// Type: fvector
    pub const HEAD_POSITION: Attribute<crate::FVector> = Attribute::new(c"head.position");

    /// Position of the trailer connection hook in vehicle
    /// space.
    ///
    /// Type: fvector
    pub const HOOK_POSITION: Attribute<crate::FVector> = Attribute::new(c"hook.position");

    /// Number of wheels
    ///
    /// Type: u32
    pub const WHEEL_COUNT: Attribute<u32> = Attribute::new(c"wheels.count");

    /// Position of respective wheels in the vehicle space.
    ///
    /// Type: indexed fvector
    pub const WHEEL_POSITION: Attribute<crate::FVector> = Attribute::indexed(c"wheel.position");

    /// Is the wheel steerable?
    ///
    /// Type: indexed bool
    pub const WHEEL_STEERABLE: Attribute<bool> = Attribute::indexed(c"wheel.steerable");

    /// Is the wheel physicaly simulated?
    ///
    /// Type: indexed bool
    pub const WHEEL_SIMULATED: Attribute<bool> = Attribute::indexed(c"wheel.simulated");

    /// Radius of the wheel
    ///
    /// Type: indexed float
    pub const WHEEL_RADIUS: Attribute<f32> = Attribute::indexed(c"wheel.radius");

    /// Is the wheel powered?
    ///
    /// Type: indexed bool
    pub const WHEEL_POWERED: Attribute<bool> = Attribute::indexed(c"wheel.powered");

    /// Is the wheel liftable?
    ///
    /// Type: indexed bool
    pub const WHEEL_LIFTABLE: Attribute<bool> = Attribute::indexed(c"wheel.liftable");

    /// Number of selectors (e.g. range/splitter toggles).
    ///
    /// Type: u32
    pub const SELECTOR_COUNT: Attribute<u32> = Attribute::new(c"selector.count");

    /// Gear selected when requirements for this h-shifter slot are meet.
    ///
    /// Type: indexed s32
    pub const SLOT_GEAR: Attribute<i32> = Attribute::indexed(c"slot.gear");

    /// Position of h-shifter handle.
    ///
    /// Zero corresponds to neutral position. Mapping to physical position of
    /// the handle depends on input setup.
    ///
    /// Type: indexed u32
    pub const SLOT_HANDLE_POSITION: Attribute<u32> = Attribute::indexed(c"slot.handle.position");

    /// Bitmask of required on/off state of selectors.
    ///
    /// Only first `selector_count` bits are relevant.
    ///
    /// Type: indexed u32
    pub const SLOT_SELECTORS: Attribute<u32> = Attribute::indexed(c"slot.selectors");

    /// Type of the shifter.
    ///
    /// One from `SCS_SHIFTER_TYPE`_* values.
    ///
    /// Type: string
    pub const SHIFTER_TYPE: Attribute<crate::StringValue> = Attribute::new(c"shifter.type");

    /// Id of the cargo for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CARGO_ID: Attribute<crate::StringValue> = Attribute::new(c"cargo.id");

    /// Name of the cargo for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const CARGO: Attribute<crate::StringValue> = Attribute::new(c"cargo");

    /// Mass of the cargo in kilograms.
    ///
    /// Type: float
    pub const CARGO_MASS: Attribute<f32> = Attribute::new(c"cargo.mass");

    /// Mass of the single unit of the cargo in kilograms.
    ///
    /// Type: float
    pub const CARGO_UNIT_MASS: Attribute<f32> = Attribute::new(c"cargo.unit.mass");

    /// How many units of the cargo the job consist of.
    ///
    /// Type: u32
    pub const CARGO_UNIT_COUNT: Attribute<u32> = Attribute::new(c"cargo.unit.count");

    /// Id of the destination city for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const DESTINATION_CITY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"destination.city.id");

    /// Name of the destination city for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const DESTINATION_CITY: Attribute<crate::StringValue> = Attribute::new(c"destination.city");

    /// Id of the destination company for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const DESTINATION_COMPANY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"destination.company.id");

    /// Name of the destination company for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const DESTINATION_COMPANY: Attribute<crate::StringValue> =
        Attribute::new(c"destination.company");

    /// Id of the source city for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const SOURCE_CITY_ID: Attribute<crate::StringValue> = Attribute::new(c"source.city.id");

    /// Name of the source city for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const SOURCE_CITY: Attribute<crate::StringValue> = Attribute::new(c"source.city");

    /// Id of the source company for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const SOURCE_COMPANY_ID: Attribute<crate::StringValue> =
        Attribute::new(c"source.company.id");

    /// Name of the source company for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const SOURCE_COMPANY: Attribute<crate::StringValue> = Attribute::new(c"source.company");

    /// Reward in internal game-specific currency.
    ///
    /// For detailed information about the currency see "Game specific units"
    /// documentation in `scssdk_telemetry`_<`game_id>.h`
    ///
    /// Type: u64
    pub const INCOME: Attribute<u64> = Attribute::new(c"income");

    /// Absolute in-game time of end of job delivery window.
    ///
    /// Delivering the job after this time will cause it be late.
    ///
    /// See `SCS_TELEMETRY_CHANNEL_game_time` for more info about absolute time.
    /// Time remaining for delivery can be obtained like (`delivery_time` - `game_time`).
    ///
    /// Type: u32
    pub const DELIVERY_TIME: Attribute<u32> = Attribute::new(c"delivery.time");

    /// Planned job distance in simulated kilometers.
    ///
    /// Does not include distance driven using ferry.
    ///
    /// Type: u32
    pub const PLANNED_DISTANCE_KM: Attribute<u32> = Attribute::new(c"planned_distance.km");

    /// Is cargo loaded on the trailer?
    ///
    /// For non cargo market jobs this is always true
    ///
    /// Type: bool
    pub const IS_CARGO_LOADED: Attribute<bool> = Attribute::new(c"cargo.loaded");

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
    pub const JOB_MARKET: Attribute<crate::StringValue> = Attribute::new(c"job.market");

    /// Flag indicating that the job is special transport job.
    ///
    /// Type: bool
    pub const SPECIAL_JOB: Attribute<bool> = Attribute::new(c"is.special.job");

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
        match value.to_bytes() {
            b"arcade" => Some(Self::Arcade),
            b"automatic" => Some(Self::Automatic),
            b"manual" => Some(Self::Manual),
            b"hshifter" => Some(Self::HShifter),
            _ => None,
        }
    }
}

const _: [(); 6] = [(); ids::COUNT];
const _: [(); 60] = [(); attributes::COUNT];

#[cfg(test)]
mod tests {
    use super::*;

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
    fn every_documented_shifter_type_round_trips() {
        let values = [
            ShifterType::Arcade,
            ShifterType::Automatic,
            ShifterType::Manual,
            ShifterType::HShifter,
        ];
        for value in values {
            assert_eq!(ShifterType::from_c_str(value.as_c_str()), Some(value));
        }
        assert_eq!(ShifterType::from_c_str(c"future-shifter"), None);
    }
}
