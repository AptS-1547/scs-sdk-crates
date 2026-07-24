//! Raw configuration identifiers and attribute names from the SCS Telemetry SDK.
//!
//! Values and documentation mirror `scssdk_telemetry_common_configs.h`. All
//! string constants are NUL-terminated for direct use with the C ABI.

/// Maximum number of trailers reported by the SDK.
pub const MAX_TRAILERS: usize = 10;

/// Configuration group identifiers.
pub mod ids {
    /// Number of configuration IDs declared by the official header.
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
    pub const SUBSTANCES: &[u8] = b"substances\0";

    /// Static configuration of the controls.
    ///
    /// - `shifter_type`
    pub const CONTROLS: &[u8] = b"controls\0";

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
    pub const HSHIFTER: &[u8] = b"hshifter\0";

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
    pub const TRUCK: &[u8] = b"truck\0";

    /// Backward compatibility static configuration of the first trailer (attributes are equal to trailer.0).
    ///
    /// The trailers configurations are returned using trailer.[index]
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
    pub const TRAILER: &[u8] = b"trailer\0";

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
    pub const JOB: &[u8] = b"job\0";

    /// All raw configuration identifiers in official header order.
    pub const ALL: [&[u8]; COUNT] = [SUBSTANCES, CONTROLS, HSHIFTER, TRUCK, TRAILER, JOB];
}

/// Attribute names used by configuration events.
pub mod attributes {
    /// Number of configuration attributes declared by the official header.
    pub const COUNT: usize = 60;

    /// Brand id for configuration purposes.
    ///
    /// Limited to C-identifier characters.
    ///
    /// Type: string
    pub const BRAND_ID: &[u8] = b"brand_id\0";

    /// Brand for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const BRAND: &[u8] = b"brand\0";

    /// Name for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const ID: &[u8] = b"id\0";

    /// Name of cargo accessory for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CARGO_ACCESSORY_ID: &[u8] = b"cargo.accessory.id\0";

    /// Name of trailer chain type.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CHAIN_TYPE: &[u8] = b"chain.type\0";

    /// Name of trailer body type.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const BODY_TYPE: &[u8] = b"body.type\0";

    /// Vehicle license plate.
    ///
    /// Type: string
    pub const LICENSE_PLATE: &[u8] = b"license.plate\0";

    /// The id representing license plate country.
    ///
    /// Type: string
    pub const LICENSE_PLATE_COUNTRY_ID: &[u8] = b"license.plate.country.id\0";

    /// The name of the license plate country.
    ///
    /// Type: string
    pub const LICENSE_PLATE_COUNTRY: &[u8] = b"license.plate.country\0";

    /// Name for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const NAME: &[u8] = b"name\0";

    /// Fuel tank capacity in litres.
    ///
    /// Type: float
    pub const FUEL_CAPACITY: &[u8] = b"fuel.capacity\0";

    /// Fraction of the fuel capacity below which
    /// is activated the fuel warning.
    ///
    /// Type: float
    pub const FUEL_WARNING_FACTOR: &[u8] = b"fuel.warning.factor\0";

    /// `AdBlue` tank capacity in litres.
    ///
    /// Type: float
    pub const ADBLUE_CAPACITY: &[u8] = b"adblue.capacity\0";

    /// Fraction of the adblue capacity below which
    /// is activated the adblue warning.
    ///
    /// Type: float
    pub const ADBLUE_WARNING_FACTOR: &[u8] = b"adblue.warning.factor\0";

    /// Pressure of the air in the tank below which
    /// the warning activates.
    ///
    /// Type: float
    pub const AIR_PRESSURE_WARNING: &[u8] = b"brake.air.pressure.warning\0";

    /// Pressure of the air in the tank below which
    /// the emergency brakes activate.
    ///
    /// Type: float
    pub const AIR_PRESSURE_EMERGENCY: &[u8] = b"brake.air.pressure.emergency\0";

    /// Pressure of the oil below which the warning activates.
    ///
    /// Type: float
    pub const OIL_PRESSURE_WARNING: &[u8] = b"oil.pressure.warning\0";

    /// Temperature of the water above which the warning activates.
    ///
    /// Type: float
    pub const WATER_TEMPERATURE_WARNING: &[u8] = b"water.temperature.warning\0";

    /// Voltage of the battery below which the warning activates.
    ///
    /// Type: float
    pub const BATTERY_VOLTAGE_WARNING: &[u8] = b"battery.voltage.warning\0";

    /// Maximum rpm value.
    ///
    /// Type: float
    pub const RPM_LIMIT: &[u8] = b"rpm.limit\0";

    /// Number of forward gears on undamaged truck.
    ///
    /// Type: u32
    pub const FORWARD_GEAR_COUNT: &[u8] = b"gears.forward\0";

    /// Number of reversee gears on undamaged truck.
    ///
    /// Type: u32
    pub const REVERSE_GEAR_COUNT: &[u8] = b"gears.reverse\0";

    /// Differential ratio of the truck.
    ///
    /// Type: float
    pub const DIFFERENTIAL_RATIO: &[u8] = b"differential.ratio\0";

    /// Number of steps in the retarder.
    ///
    /// Set to zero if retarder is not mounted to the truck.
    ///
    /// Type: u32
    pub const RETARDER_STEP_COUNT: &[u8] = b"retarder.steps\0";

    /// Forward transmission ratios.
    ///
    /// Type: indexed float
    pub const FORWARD_RATIO: &[u8] = b"forward.ratio\0";

    /// Reverse transmission ratios.
    ///
    /// Type: indexed float
    pub const REVERSE_RATIO: &[u8] = b"reverse.ratio\0";

    /// Position of the cabin in the vehicle space.
    ///
    /// This is position of the joint around which the cabin rotates.
    /// This attribute might be not present if the vehicle does not
    /// have a separate cabin.
    ///
    /// Type: fvector
    pub const CABIN_POSITION: &[u8] = b"cabin.position\0";

    /// Default position of the head in the cabin space.
    ///
    /// Type: fvector
    pub const HEAD_POSITION: &[u8] = b"head.position\0";

    /// Position of the trailer connection hook in vehicle
    /// space.
    ///
    /// Type: fvector
    pub const HOOK_POSITION: &[u8] = b"hook.position\0";

    /// Number of wheels
    ///
    /// Type: u32
    pub const WHEEL_COUNT: &[u8] = b"wheels.count\0";

    /// Position of respective wheels in the vehicle space.
    ///
    /// Type: indexed fvector
    pub const WHEEL_POSITION: &[u8] = b"wheel.position\0";

    /// Is the wheel steerable?
    ///
    /// Type: indexed bool
    pub const WHEEL_STEERABLE: &[u8] = b"wheel.steerable\0";

    /// Is the wheel physicaly simulated?
    ///
    /// Type: indexed bool
    pub const WHEEL_SIMULATED: &[u8] = b"wheel.simulated\0";

    /// Radius of the wheel
    ///
    /// Type: indexed float
    pub const WHEEL_RADIUS: &[u8] = b"wheel.radius\0";

    /// Is the wheel powered?
    ///
    /// Type: indexed bool
    pub const WHEEL_POWERED: &[u8] = b"wheel.powered\0";

    /// Is the wheel liftable?
    ///
    /// Type: indexed bool
    pub const WHEEL_LIFTABLE: &[u8] = b"wheel.liftable\0";

    /// Number of selectors (e.g. range/splitter toggles).
    ///
    /// Type: u32
    pub const SELECTOR_COUNT: &[u8] = b"selector.count\0";

    /// Gear selected when requirements for this h-shifter slot are meet.
    ///
    /// Type: indexed s32
    pub const SLOT_GEAR: &[u8] = b"slot.gear\0";

    /// Position of h-shifter handle.
    ///
    /// Zero corresponds to neutral position. Mapping to physical position of
    /// the handle depends on input setup.
    ///
    /// Type: indexed u32
    pub const SLOT_HANDLE_POSITION: &[u8] = b"slot.handle.position\0";

    /// Bitmask of required on/off state of selectors.
    ///
    /// Only first `selector_count` bits are relevant.
    ///
    /// Type: indexed u32
    pub const SLOT_SELECTORS: &[u8] = b"slot.selectors\0";

    /// Type of the shifter.
    ///
    /// One from `SCS_SHIFTER_TYPE`_* values.
    ///
    /// Type: string
    pub const SHIFTER_TYPE: &[u8] = b"shifter.type\0";

    /// Id of the cargo for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const CARGO_ID: &[u8] = b"cargo.id\0";

    /// Name of the cargo for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const CARGO: &[u8] = b"cargo\0";

    /// Mass of the cargo in kilograms.
    ///
    /// Type: float
    pub const CARGO_MASS: &[u8] = b"cargo.mass\0";

    /// Mass of the single unit of the cargo in kilograms.
    ///
    /// Type: float
    pub const CARGO_UNIT_MASS: &[u8] = b"cargo.unit.mass\0";

    /// How many units of the cargo the job consist of.
    ///
    /// Type: u32
    pub const CARGO_UNIT_COUNT: &[u8] = b"cargo.unit.count\0";

    /// Id of the destination city for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const DESTINATION_CITY_ID: &[u8] = b"destination.city.id\0";

    /// Name of the destination city for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const DESTINATION_CITY: &[u8] = b"destination.city\0";

    /// Id of the destination company for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const DESTINATION_COMPANY_ID: &[u8] = b"destination.company.id\0";

    /// Name of the destination company for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const DESTINATION_COMPANY: &[u8] = b"destination.company\0";

    /// Id of the source city for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const SOURCE_CITY_ID: &[u8] = b"source.city.id\0";

    /// Name of the source city for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const SOURCE_CITY: &[u8] = b"source.city\0";

    /// Id of the source company for internal use by code.
    ///
    /// Limited to C-identifier characters and dots.
    ///
    /// Type: string
    pub const SOURCE_COMPANY_ID: &[u8] = b"source.company.id\0";

    /// Name of the source company for display purposes.
    ///
    /// Localized using the current in-game language.
    ///
    /// Type: string
    pub const SOURCE_COMPANY: &[u8] = b"source.company\0";

    /// Reward in internal game-specific currency.
    ///
    /// For detailed information about the currency see "Game specific units"
    /// documentation in `scssdk_telemetry`_<`game_id>.h`
    ///
    /// Type: u64
    pub const INCOME: &[u8] = b"income\0";

    /// Absolute in-game time of end of job delivery window.
    ///
    /// Delivering the job after this time will cause it be late.
    ///
    /// See `SCS_TELEMETRY_CHANNEL_game_time` for more info about absolute time.
    /// Time remaining for delivery can be obtained like (`delivery_time` - `game_time`).
    ///
    /// Type: u32
    pub const DELIVERY_TIME: &[u8] = b"delivery.time\0";

    /// Planned job distance in simulated kilometers.
    ///
    /// Does not include distance driven using ferry.
    ///
    /// Type: u32
    pub const PLANNED_DISTANCE_KM: &[u8] = b"planned_distance.km\0";

    /// Is cargo loaded on the trailer?
    ///
    /// For non cargo market jobs this is always true
    ///
    /// Type: bool
    pub const IS_CARGO_LOADED: &[u8] = b"cargo.loaded\0";

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
    pub const JOB_MARKET: &[u8] = b"job.market\0";

    /// Flag indicating that the job is special transport job.
    ///
    /// Type: bool
    pub const SPECIAL_JOB: &[u8] = b"is.special.job\0";

    /// All raw configuration attribute names in official header order.
    pub const ALL: [&[u8]; COUNT] = [
        BRAND_ID,
        BRAND,
        ID,
        CARGO_ACCESSORY_ID,
        CHAIN_TYPE,
        BODY_TYPE,
        LICENSE_PLATE,
        LICENSE_PLATE_COUNTRY_ID,
        LICENSE_PLATE_COUNTRY,
        NAME,
        FUEL_CAPACITY,
        FUEL_WARNING_FACTOR,
        ADBLUE_CAPACITY,
        ADBLUE_WARNING_FACTOR,
        AIR_PRESSURE_WARNING,
        AIR_PRESSURE_EMERGENCY,
        OIL_PRESSURE_WARNING,
        WATER_TEMPERATURE_WARNING,
        BATTERY_VOLTAGE_WARNING,
        RPM_LIMIT,
        FORWARD_GEAR_COUNT,
        REVERSE_GEAR_COUNT,
        DIFFERENTIAL_RATIO,
        RETARDER_STEP_COUNT,
        FORWARD_RATIO,
        REVERSE_RATIO,
        CABIN_POSITION,
        HEAD_POSITION,
        HOOK_POSITION,
        WHEEL_COUNT,
        WHEEL_POSITION,
        WHEEL_STEERABLE,
        WHEEL_SIMULATED,
        WHEEL_RADIUS,
        WHEEL_POWERED,
        WHEEL_LIFTABLE,
        SELECTOR_COUNT,
        SLOT_GEAR,
        SLOT_HANDLE_POSITION,
        SLOT_SELECTORS,
        SHIFTER_TYPE,
        CARGO_ID,
        CARGO,
        CARGO_MASS,
        CARGO_UNIT_MASS,
        CARGO_UNIT_COUNT,
        DESTINATION_CITY_ID,
        DESTINATION_CITY,
        DESTINATION_COMPANY_ID,
        DESTINATION_COMPANY,
        SOURCE_CITY_ID,
        SOURCE_CITY,
        SOURCE_COMPANY_ID,
        SOURCE_COMPANY,
        INCOME,
        DELIVERY_TIME,
        PLANNED_DISTANCE_KM,
        IS_CARGO_LOADED,
        JOB_MARKET,
        SPECIAL_JOB,
    ];
}

/// Values of the shifter.type configuration attribute.
pub mod shifter_types {
    /// Number of documented shifter type values.
    pub const COUNT: usize = 4;

    /// Documented shifter type ARCADE.
    pub const ARCADE: &[u8] = b"arcade\0";

    /// Documented shifter type AUTOMATIC.
    pub const AUTOMATIC: &[u8] = b"automatic\0";

    /// Documented shifter type MANUAL.
    pub const MANUAL: &[u8] = b"manual\0";

    /// Documented shifter type HSHIFTER.
    pub const HSHIFTER: &[u8] = b"hshifter\0";

    /// All documented raw shifter-type values in official header order.
    pub const ALL: [&[u8]; COUNT] = [ARCADE, AUTOMATIC, MANUAL, HSHIFTER];
}

/// Total number of string macros covered by this module.
pub const STRING_COUNT: usize = ids::COUNT + attributes::COUNT + shifter_types::COUNT;

const _: [(); 70] = [(); STRING_COUNT];
