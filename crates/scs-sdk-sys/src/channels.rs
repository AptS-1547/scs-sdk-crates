//! Raw channel names from the SCS Telemetry SDK headers.
//!
//! Every byte string is NUL-terminated for direct use with the C ABI. The
//! documentation is derived from the official SDK 1.14 header bundle in
//! third-party/scs_sdk_1_14/include/common.

/// Channels declared by `scssdk_telemetry_common_channels.h`.
pub mod common {
    /// Number of public channel-name macros in the matching official header.
    pub const COUNT: usize = 4;

    /// Scale applied to distance and time to compensate
    /// for the scale of the map (e.g. 1s of real time corresponds to `local_scale`
    /// seconds of simulated game time).
    ///
    /// Games which use real 1:1 maps will not provide this
    /// channel.
    ///
    /// Type: float
    pub const LOCAL_SCALE: &[u8] = b"local.scale\0";

    /// Absolute in-game time.
    ///
    /// Represented in number of in-game minutes since beginning (i.e. 00:00)
    /// of the first in-game day.
    ///
    /// Type: u32
    pub const GAME_TIME: &[u8] = b"game.time\0";

    /// Offset from the `game_time` simulated in the local economy to the
    /// game time of the Convoy multiplayer server.
    ///
    /// The value of this channel can change frequently during the Convoy
    /// session. For example when the user enters the desktop, the local
    /// economy time stops however the multiplayer time continues to run
    /// so the value will start to change.
    ///
    /// Represented in in-game minutes. Set to 0 when multiplayer is not active.
    ///
    /// Type: s32
    pub const MULTIPLAYER_TIME_OFFSET: &[u8] = b"multiplayer.time.offset\0";

    /// Time until next rest stop.
    ///
    /// When the fatique simulation is disabled, the behavior of this channel
    /// is implementation dependent. The game might provide the value which would
    /// apply if it was enabled or provide no value at all.
    ///
    /// Represented in in-game minutes.
    ///
    /// Type: s32
    pub const NEXT_REST_STOP: &[u8] = b"rest.stop\0";

    /// All raw common channel names in official header order.
    pub const ALL: [&[u8]; COUNT] = [
        LOCAL_SCALE,
        GAME_TIME,
        MULTIPLAYER_TIME_OFFSET,
        NEXT_REST_STOP,
    ];
}

/// Channels declared by `scssdk_telemetry_truck_common_channels.h`.
pub mod truck {
    /// Number of public channel-name macros in the matching official header.
    pub const COUNT: usize = 84;

    /// Represents world space position and orientation of the truck.
    ///
    /// Type: dplacement
    pub const WORLD_PLACEMENT: &[u8] = b"truck.world.placement\0";

    /// Represents vehicle space linear velocity of the truck measured
    /// in meters per second.
    ///
    /// Type: fvector
    pub const LOCAL_LINEAR_VELOCITY: &[u8] = b"truck.local.velocity.linear\0";

    /// Represents vehicle space angular velocity of the truck measured
    /// in rotations per second.
    ///
    /// Type: fvector
    pub const LOCAL_ANGULAR_VELOCITY: &[u8] = b"truck.local.velocity.angular\0";

    /// Represents vehicle space linear acceleration of the truck measured
    /// in meters per second^2
    ///
    /// Type: fvector
    pub const LOCAL_LINEAR_ACCELERATION: &[u8] = b"truck.local.acceleration.linear\0";

    /// Represents vehicle space angular acceleration of the truck meassured
    /// in rotations per second^2
    ///
    /// Type: fvector
    pub const LOCAL_ANGULAR_ACCELERATION: &[u8] = b"truck.local.acceleration.angular\0";

    /// Represents a vehicle space position and orientation delta
    /// of the cabin from its default position.
    ///
    /// Type: fplacement
    pub const CABIN_OFFSET: &[u8] = b"truck.cabin.offset\0";

    /// Represents cabin space angular velocity of the cabin measured
    /// in rotations per second.
    ///
    /// Type: fvector
    pub const CABIN_ANGULAR_VELOCITY: &[u8] = b"truck.cabin.velocity.angular\0";

    /// Represents cabin space angular acceleration of the cabin
    /// measured in rotations per second^2
    ///
    /// Type: fvector
    pub const CABIN_ANGULAR_ACCELERATION: &[u8] = b"truck.cabin.acceleration.angular\0";

    /// Represents a cabin space position and orientation delta
    /// of the driver head from its default position.
    ///
    /// Note that this value might change rapidly as result of
    /// the user switching between cameras or camera presets.
    ///
    /// Type: fplacement
    pub const HEAD_OFFSET: &[u8] = b"truck.head.offset\0";

    /// Speedometer speed in meters per second.
    ///
    /// Uses negative value to represent reverse movement.
    ///
    /// Type: float
    pub const SPEED: &[u8] = b"truck.speed\0";

    /// RPM of the engine.
    ///
    /// Type: float
    pub const ENGINE_RPM: &[u8] = b"truck.engine.rpm\0";

    /// Gear currently selected in the engine.
    ///
    /// - >0 - Forwad gears
    /// - 0 - Neutral
    /// - <0 - Reverse gears
    ///
    /// Type: s32
    pub const ENGINE_GEAR: &[u8] = b"truck.engine.gear\0";

    /// Gear currently displayed on dashboard.
    ///
    /// - >0 - Forwad gears
    /// - 0 - Neutral
    /// - <0 - Reverse gears
    ///
    /// Type: s32
    pub const DISPLAYED_GEAR: &[u8] = b"truck.displayed.gear\0";

    /// Steering received from input <-1;1>.
    ///
    /// Note that it is interpreted counterclockwise.
    ///
    /// If the user presses the steer right button on digital input
    /// (e.g. keyboard) this value goes immediatelly to -1.0
    ///
    /// Type: float
    pub const INPUT_STEERING: &[u8] = b"truck.input.steering\0";

    /// Throttle received from input <0;1>
    ///
    /// If the user presses the forward button on digital input
    /// (e.g. keyboard) this value goes immediatelly to 1.0
    ///
    /// Type: float
    pub const INPUT_THROTTLE: &[u8] = b"truck.input.throttle\0";

    /// Brake received from input <0;1>
    ///
    /// If the user presses the brake button on digital input
    /// (e.g. keyboard) this value goes immediatelly to 1.0
    ///
    /// Type: float
    pub const INPUT_BRAKE: &[u8] = b"truck.input.brake\0";

    /// Clutch received from input <0;1>
    ///
    /// If the user presses the clutch button on digital input
    /// (e.g. keyboard) this value goes immediatelly to 1.0
    ///
    /// Type: float
    pub const INPUT_CLUTCH: &[u8] = b"truck.input.clutch\0";

    /// Steering as used by the simulation <-1;1>
    ///
    /// Note that it is interpreted counterclockwise.
    ///
    /// Accounts for interpolation speeds and simulated
    /// counterfoces for digital inputs.
    ///
    /// Type: float
    pub const EFFECTIVE_STEERING: &[u8] = b"truck.effective.steering\0";

    /// Throttle pedal input as used by the simulation <0;1>
    ///
    /// Accounts for the press attack curve for digital inputs
    /// or cruise-control input.
    ///
    /// Type: float
    pub const EFFECTIVE_THROTTLE: &[u8] = b"truck.effective.throttle\0";

    /// Brake pedal input as used by the simulation <0;1>
    ///
    /// Accounts for the press attack curve for digital inputs. Does
    /// not contain retarder, parking or engine brake.
    ///
    /// Type: float
    pub const EFFECTIVE_BRAKE: &[u8] = b"truck.effective.brake\0";

    /// Clutch pedal input as used by the simulation <0;1>
    ///
    /// Accounts for the automatic shifting or interpolation of
    /// player input.
    ///
    /// Type: float
    pub const EFFECTIVE_CLUTCH: &[u8] = b"truck.effective.clutch\0";

    /// Speed selected for the cruise control in m/s
    ///
    /// Is zero if cruise control is disabled.
    ///
    /// Type: float
    pub const CRUISE_CONTROL: &[u8] = b"truck.cruise_control\0";

    /// Gearbox slot the h-shifter handle is currently in.
    ///
    /// 0 means that no slot is selected.
    ///
    /// Type: u32
    pub const HSHIFTER_SLOT: &[u8] = b"truck.hshifter.slot\0";

    /// Enabled state of range/splitter selector toggles.
    ///
    /// Mapping between the range/splitter functionality and
    /// selector index is described by HSHIFTER configuration.
    ///
    /// Type: indexed bool
    pub const HSHIFTER_SELECTOR: &[u8] = b"truck.hshifter.select\0";

    /// Is the parking brake enabled?
    ///
    /// Type: bool
    pub const PARKING_BRAKE: &[u8] = b"truck.brake.parking\0";

    /// Is the engine brake enabled?
    ///
    /// Type: bool
    pub const MOTOR_BRAKE: &[u8] = b"truck.brake.motor\0";

    /// Current level of the retarder.
    ///
    /// <0;max> where 0 is disabled retarder and max is maximum
    /// value found in TRUCK configuration.
    ///
    /// Type: u32
    pub const RETARDER_LEVEL: &[u8] = b"truck.brake.retarder\0";

    /// Pressure in the brake air tank in psi
    ///
    /// Type: float
    pub const BRAKE_AIR_PRESSURE: &[u8] = b"truck.brake.air.pressure\0";

    /// Is the air pressure warning active?
    ///
    /// Type: bool
    pub const BRAKE_AIR_PRESSURE_WARNING: &[u8] = b"truck.brake.air.pressure.warning\0";

    /// Are the emergency brakes active as result of low air pressure?
    ///
    /// Type: bool
    pub const BRAKE_AIR_PRESSURE_EMERGENCY: &[u8] = b"truck.brake.air.pressure.emergency\0";

    /// Temperature of the brakes in degrees celsius.
    ///
    /// Aproximated for entire truck, not at the wheel level.
    ///
    /// Type: float
    pub const BRAKE_TEMPERATURE: &[u8] = b"truck.brake.temperature\0";

    /// Amount of fuel in liters
    ///
    /// Type: float
    pub const FUEL: &[u8] = b"truck.fuel.amount\0";

    /// Is the low fuel warning active?
    ///
    /// Type: bool
    pub const FUEL_WARNING: &[u8] = b"truck.fuel.warning\0";

    /// Average consumption of the fuel in liters/km
    ///
    /// Type: float
    pub const FUEL_AVERAGE_CONSUMPTION: &[u8] = b"truck.fuel.consumption.average\0";

    /// Estimated range of truck with current amount of fuel in km
    ///
    /// Type: float
    pub const FUEL_RANGE: &[u8] = b"truck.fuel.range\0";

    /// Amount of `AdBlue` in liters
    ///
    /// Type: float
    pub const ADBLUE: &[u8] = b"truck.adblue\0";

    /// Is the low adblue warning active?
    ///
    /// Type: bool
    pub const ADBLUE_WARNING: &[u8] = b"truck.adblue.warning\0";

    /// Average consumption of the adblue in liters/km
    ///
    /// Type: float
    pub const ADBLUE_AVERAGE_CONSUMPTION: &[u8] = b"truck.adblue.consumption.average\0";

    /// Pressure of the oil in psi
    ///
    /// Type: float
    pub const OIL_PRESSURE: &[u8] = b"truck.oil.pressure\0";

    /// Is the oil pressure warning active?
    ///
    /// Type: bool
    pub const OIL_PRESSURE_WARNING: &[u8] = b"truck.oil.pressure.warning\0";

    /// Temperature of the oil in degrees celsius.
    ///
    /// Type: float
    pub const OIL_TEMPERATURE: &[u8] = b"truck.oil.temperature\0";

    /// Temperature of the water in degrees celsius.
    ///
    /// Type: float
    pub const WATER_TEMPERATURE: &[u8] = b"truck.water.temperature\0";

    /// Is the water temperature warning active?
    ///
    /// Type: bool
    pub const WATER_TEMPERATURE_WARNING: &[u8] = b"truck.water.temperature.warning\0";

    /// Voltage of the battery in volts.
    ///
    /// Type: float
    pub const BATTERY_VOLTAGE: &[u8] = b"truck.battery.voltage\0";

    /// Is the battery voltage/not charging warning active?
    ///
    /// Type: bool
    pub const BATTERY_VOLTAGE_WARNING: &[u8] = b"truck.battery.voltage.warning\0";

    /// Is the electric enabled?
    ///
    /// Type: bool
    pub const ELECTRIC_ENABLED: &[u8] = b"truck.electric.enabled\0";

    /// Is the engine enabled?
    ///
    /// Type: bool
    pub const ENGINE_ENABLED: &[u8] = b"truck.engine.enabled\0";

    /// Is the left blinker enabled?
    ///
    /// This represents the logical enable state of the blinker. It
    /// it is true as long the blinker is enabled regardless of the
    /// physical enabled state of the light (i.e. it does not blink
    /// and ignores enable state of electric).
    ///
    /// Type: bool
    pub const LBLINKER: &[u8] = b"truck.lblinker\0";

    /// Is the right blinker enabled?
    ///
    /// This represents the logical enable state of the blinker. It
    /// it is true as long the blinker is enabled regardless of the
    /// physical enabled state of the light (i.e. it does not blink
    /// and ignores enable state of electric).
    ///
    /// Type: bool
    pub const RBLINKER: &[u8] = b"truck.rblinker\0";

    /// Are the hazard warning light enabled?
    ///
    /// This represents the logical enable state of the hazard warning.
    /// It it is true as long it is enabled regardless of the physical
    /// enabled state of the light (i.e. it does not blink).
    ///
    /// Type: bool
    pub const HAZARD_WARNING: &[u8] = b"truck.hazard.warning\0";

    /// Is the light in the left blinker currently on?
    ///
    /// Type: bool
    pub const LIGHT_LBLINKER: &[u8] = b"truck.light.lblinker\0";

    /// Is the light in the right blinker currently on?
    ///
    /// Type: bool
    pub const LIGHT_RBLINKER: &[u8] = b"truck.light.rblinker\0";

    /// Are the parking lights enabled?
    ///
    /// Type: bool
    pub const LIGHT_PARKING: &[u8] = b"truck.light.parking\0";

    /// Are the low beam lights enabled?
    ///
    /// Type: bool
    pub const LIGHT_LOW_BEAM: &[u8] = b"truck.light.beam.low\0";

    /// Are the high beam lights enabled?
    ///
    /// Type: bool
    pub const LIGHT_HIGH_BEAM: &[u8] = b"truck.light.beam.high\0";

    /// Are the auxiliary front lights active?
    ///
    /// Those lights have several intensity levels:
    /// - 1 - dimmed state
    /// - 2 - full state
    ///
    /// Type: u32
    pub const LIGHT_AUX_FRONT: &[u8] = b"truck.light.aux.front\0";

    /// Are the auxiliary roof lights active?
    ///
    /// Those lights have several intensity levels:
    /// - 1 - dimmed state
    /// - 2 - full state
    ///
    /// Type: u32
    pub const LIGHT_AUX_ROOF: &[u8] = b"truck.light.aux.roof\0";

    /// Are the beacon lights enabled?
    ///
    /// Type: bool
    pub const LIGHT_BEACON: &[u8] = b"truck.light.beacon\0";

    /// Is the brake light active?
    ///
    /// Type: bool
    pub const LIGHT_BRAKE: &[u8] = b"truck.light.brake\0";

    /// Is the reverse light active?
    ///
    /// Type: bool
    pub const LIGHT_REVERSE: &[u8] = b"truck.light.reverse\0";

    /// Are the wipers enabled?
    ///
    /// Type: bool
    pub const WIPERS: &[u8] = b"truck.wipers\0";

    /// Intensity of the dashboard backlight as factor <0;1>
    ///
    /// Type: float
    pub const DASHBOARD_BACKLIGHT: &[u8] = b"truck.dashboard.backlight\0";

    /// Is the differential lock enabled?
    ///
    /// Type: bool
    pub const DIFFERENTIAL_LOCK: &[u8] = b"truck.differential_lock\0";

    /// Is the lift axle control set to lifted state?
    ///
    /// Type: bool
    pub const LIFT_AXLE: &[u8] = b"truck.lift_axle\0";

    /// Is the lift axle indicator lit?
    ///
    /// Type: bool
    pub const LIFT_AXLE_INDICATOR: &[u8] = b"truck.lift_axle.indicator\0";

    /// Is the trailer lift axle control set to lifted state?
    ///
    /// Type: bool
    pub const TRAILER_LIFT_AXLE: &[u8] = b"truck.trailer.lift_axle\0";

    /// Is the trailer lift axle indicator lit?
    ///
    /// Type: bool
    pub const TRAILER_LIFT_AXLE_INDICATOR: &[u8] = b"truck.trailer.lift_axle.indicator\0";

    /// Wear of the engine accessory as <0;1>
    ///
    /// Type: float
    pub const WEAR_ENGINE: &[u8] = b"truck.wear.engine\0";

    /// Wear of the transmission accessory as <0;1>
    ///
    /// Type: float
    pub const WEAR_TRANSMISSION: &[u8] = b"truck.wear.transmission\0";

    /// Wear of the cabin accessory as <0;1>
    ///
    /// Type: float
    pub const WEAR_CABIN: &[u8] = b"truck.wear.cabin\0";

    /// Wear of the chassis accessory as <0;1>
    ///
    /// Type: float
    pub const WEAR_CHASSIS: &[u8] = b"truck.wear.chassis\0";

    /// Average wear across the wheel accessories as <0;1>
    ///
    /// Type: float
    pub const WEAR_WHEELS: &[u8] = b"truck.wear.wheels\0";

    /// The value of the odometer in km.
    ///
    /// Type: float
    pub const ODOMETER: &[u8] = b"truck.odometer\0";

    /// The value of truck's navigation distance (in meters).
    ///
    /// This is the value used by the advisor.
    ///
    /// Type: float
    pub const NAVIGATION_DISTANCE: &[u8] = b"truck.navigation.distance\0";

    /// The value of truck's navigation eta (in second).
    ///
    /// This is the value used by the advisor.
    ///
    /// Type: float
    pub const NAVIGATION_TIME: &[u8] = b"truck.navigation.time\0";

    /// The value of truck's navigation speed limit (in m/s).
    ///
    /// This is the value used by the advisor and respects the
    /// current state of the "Route Advisor speed limit" option.
    ///
    /// Type: float
    pub const NAVIGATION_SPEED_LIMIT: &[u8] = b"truck.navigation.speed.limit\0";

    /// Vertical displacement of the wheel from its
    /// axis in meters.
    ///
    /// Type: indexed float
    pub const WHEEL_SUSP_DEFLECTION: &[u8] = b"truck.wheel.suspension.deflection\0";

    /// Is the wheel in contact with ground?
    ///
    /// Type: indexed bool
    pub const WHEEL_ON_GROUND: &[u8] = b"truck.wheel.on_ground\0";

    /// Substance below the whell.
    ///
    /// Index of substance as delivered trough SUBSTANCE config.
    ///
    /// Type: indexed u32
    pub const WHEEL_SUBSTANCE: &[u8] = b"truck.wheel.substance\0";

    /// Angular velocity of the wheel in rotations per
    /// second.
    ///
    /// Positive velocity corresponds to forward movement.
    ///
    /// Type: indexed float
    pub const WHEEL_VELOCITY: &[u8] = b"truck.wheel.angular_velocity\0";

    /// Steering rotation of the wheel in rotations.
    ///
    /// Value is from <-0.25,0.25> range in counterclockwise direction
    /// when looking from top (e.g. 0.25 corresponds to left and
    /// -0.25 corresponds to right).
    ///
    /// Set to zero for non-steered wheels.
    ///
    /// Type: indexed float
    pub const WHEEL_STEERING: &[u8] = b"truck.wheel.steering\0";

    /// Rolling rotation of the wheel in rotations.
    ///
    /// Value is from <0.0,1.0) range in which value
    /// increase corresponds to forward movement.
    ///
    /// Type: indexed float
    pub const WHEEL_ROTATION: &[u8] = b"truck.wheel.rotation\0";

    /// Lift state of the wheel <0;1>
    ///
    /// For use with simple lifted/non-lifted test or logical
    /// visualization of the lifting progress.
    ///
    /// Value of 0 corresponds to non-lifted axle.
    /// Value of 1 corresponds to fully lifted axle.
    ///
    /// Set to zero or not provided for non-liftable axles.
    ///
    /// Type: indexed float
    pub const WHEEL_LIFT: &[u8] = b"truck.wheel.lift\0";

    /// Vertical displacement of the wheel axle
    /// from its normal position in meters as result of
    /// lifting.
    ///
    /// Might have non-linear relation to lift ratio.
    ///
    /// Set to zero or not provided for non-liftable axles.
    ///
    /// Type: indexed float
    pub const WHEEL_LIFT_OFFSET: &[u8] = b"truck.wheel.lift.offset\0";

    /// All raw truck channel names in official header order.
    pub const ALL: [&[u8]; COUNT] = [
        WORLD_PLACEMENT,
        LOCAL_LINEAR_VELOCITY,
        LOCAL_ANGULAR_VELOCITY,
        LOCAL_LINEAR_ACCELERATION,
        LOCAL_ANGULAR_ACCELERATION,
        CABIN_OFFSET,
        CABIN_ANGULAR_VELOCITY,
        CABIN_ANGULAR_ACCELERATION,
        HEAD_OFFSET,
        SPEED,
        ENGINE_RPM,
        ENGINE_GEAR,
        DISPLAYED_GEAR,
        INPUT_STEERING,
        INPUT_THROTTLE,
        INPUT_BRAKE,
        INPUT_CLUTCH,
        EFFECTIVE_STEERING,
        EFFECTIVE_THROTTLE,
        EFFECTIVE_BRAKE,
        EFFECTIVE_CLUTCH,
        CRUISE_CONTROL,
        HSHIFTER_SLOT,
        HSHIFTER_SELECTOR,
        PARKING_BRAKE,
        MOTOR_BRAKE,
        RETARDER_LEVEL,
        BRAKE_AIR_PRESSURE,
        BRAKE_AIR_PRESSURE_WARNING,
        BRAKE_AIR_PRESSURE_EMERGENCY,
        BRAKE_TEMPERATURE,
        FUEL,
        FUEL_WARNING,
        FUEL_AVERAGE_CONSUMPTION,
        FUEL_RANGE,
        ADBLUE,
        ADBLUE_WARNING,
        ADBLUE_AVERAGE_CONSUMPTION,
        OIL_PRESSURE,
        OIL_PRESSURE_WARNING,
        OIL_TEMPERATURE,
        WATER_TEMPERATURE,
        WATER_TEMPERATURE_WARNING,
        BATTERY_VOLTAGE,
        BATTERY_VOLTAGE_WARNING,
        ELECTRIC_ENABLED,
        ENGINE_ENABLED,
        LBLINKER,
        RBLINKER,
        HAZARD_WARNING,
        LIGHT_LBLINKER,
        LIGHT_RBLINKER,
        LIGHT_PARKING,
        LIGHT_LOW_BEAM,
        LIGHT_HIGH_BEAM,
        LIGHT_AUX_FRONT,
        LIGHT_AUX_ROOF,
        LIGHT_BEACON,
        LIGHT_BRAKE,
        LIGHT_REVERSE,
        WIPERS,
        DASHBOARD_BACKLIGHT,
        DIFFERENTIAL_LOCK,
        LIFT_AXLE,
        LIFT_AXLE_INDICATOR,
        TRAILER_LIFT_AXLE,
        TRAILER_LIFT_AXLE_INDICATOR,
        WEAR_ENGINE,
        WEAR_TRANSMISSION,
        WEAR_CABIN,
        WEAR_CHASSIS,
        WEAR_WHEELS,
        ODOMETER,
        NAVIGATION_DISTANCE,
        NAVIGATION_TIME,
        NAVIGATION_SPEED_LIMIT,
        WHEEL_SUSP_DEFLECTION,
        WHEEL_ON_GROUND,
        WHEEL_SUBSTANCE,
        WHEEL_VELOCITY,
        WHEEL_STEERING,
        WHEEL_ROTATION,
        WHEEL_LIFT,
        WHEEL_LIFT_OFFSET,
    ];
}

/// Channels declared by `scssdk_telemetry_trailer_common_channels.h`.
pub mod trailer {
    /// Number of public channel-name macros in the matching official header.
    pub const COUNT: usize = 18;

    /// Is the trailer connected to the truck?
    ///
    /// Type: bool
    pub const CONNECTED: &[u8] = b"trailer.connected\0";

    /// How much is the cargo damaged that is loaded to this trailer in <0.0, 1.0> range.
    ///
    /// Type: float
    pub const CARGO_DAMAGE: &[u8] = b"trailer.cargo.damage\0";

    /// World-space position and orientation of the trailer.
    ///
    /// Native type: `dplacement`.
    pub const WORLD_PLACEMENT: &[u8] = b"trailer.world.placement\0";

    /// Trailer-space linear velocity in metres per second.
    ///
    /// Native type: `fvector`.
    pub const LOCAL_LINEAR_VELOCITY: &[u8] = b"trailer.velocity.linear\0";

    /// Trailer-space angular velocity in rotations per second.
    ///
    /// Native type: `fvector`.
    pub const LOCAL_ANGULAR_VELOCITY: &[u8] = b"trailer.velocity.angular\0";

    /// Trailer-space linear acceleration in metres per second squared.
    ///
    /// Native type: `fvector`.
    pub const LOCAL_LINEAR_ACCELERATION: &[u8] = b"trailer.acceleration.linear\0";

    /// Trailer-space angular acceleration in rotations per second squared.
    ///
    /// Native type: `fvector`.
    pub const LOCAL_ANGULAR_ACCELERATION: &[u8] = b"trailer.acceleration.angular\0";

    /// Trailer body wear in the `[0.0, 1.0]` range.
    ///
    /// Native type: `float`.
    pub const WEAR_BODY: &[u8] = b"trailer.wear.body\0";

    /// Trailer chassis wear in the `[0.0, 1.0]` range.
    ///
    /// Native type: `float`.
    pub const WEAR_CHASSIS: &[u8] = b"trailer.wear.chassis\0";

    /// Trailer wheel wear in the `[0.0, 1.0]` range.
    ///
    /// Native type: `float`.
    pub const WEAR_WHEELS: &[u8] = b"trailer.wear.wheels\0";

    /// Indexed trailer wheel suspension deflection.
    ///
    /// Native type: indexed `float`.
    pub const WHEEL_SUSP_DEFLECTION: &[u8] = b"trailer.wheel.suspension.deflection\0";

    /// Whether an indexed trailer wheel is on the ground.
    ///
    /// Native type: indexed `bool`.
    pub const WHEEL_ON_GROUND: &[u8] = b"trailer.wheel.on_ground\0";

    /// Substance under an indexed trailer wheel.
    ///
    /// Native type: indexed `u32`.
    pub const WHEEL_SUBSTANCE: &[u8] = b"trailer.wheel.substance\0";

    /// Indexed trailer wheel angular velocity.
    ///
    /// Native type: indexed `float`.
    pub const WHEEL_VELOCITY: &[u8] = b"trailer.wheel.angular_velocity\0";

    /// Indexed trailer wheel steering angle.
    ///
    /// Native type: indexed `float`.
    pub const WHEEL_STEERING: &[u8] = b"trailer.wheel.steering\0";

    /// Indexed trailer wheel rolling rotation.
    ///
    /// Native type: indexed `float`.
    pub const WHEEL_ROTATION: &[u8] = b"trailer.wheel.rotation\0";

    /// Indexed trailer wheel lift state.
    ///
    /// Native type: indexed `float`.
    pub const WHEEL_LIFT: &[u8] = b"trailer.wheel.lift\0";

    /// Indexed trailer wheel lift offset.
    ///
    /// Native type: indexed `float`.
    pub const WHEEL_LIFT_OFFSET: &[u8] = b"trailer.wheel.lift.offset\0";

    /// All raw trailer channel names in official header order.
    pub const ALL: [&[u8]; COUNT] = [
        CONNECTED,
        CARGO_DAMAGE,
        WORLD_PLACEMENT,
        LOCAL_LINEAR_VELOCITY,
        LOCAL_ANGULAR_VELOCITY,
        LOCAL_LINEAR_ACCELERATION,
        LOCAL_ANGULAR_ACCELERATION,
        WEAR_BODY,
        WEAR_CHASSIS,
        WEAR_WHEELS,
        WHEEL_SUSP_DEFLECTION,
        WHEEL_ON_GROUND,
        WHEEL_SUBSTANCE,
        WHEEL_VELOCITY,
        WHEEL_STEERING,
        WHEEL_ROTATION,
        WHEEL_LIFT,
        WHEEL_LIFT_OFFSET,
    ];
}

/// Channels declared by `scssdk_telemetry_job_common_channels.h`.
pub mod job {
    /// Number of public channel-name macros in the matching official header.
    pub const COUNT: usize = 1;

    /// The total damage of the cargo in range 0.0 to 1.0.
    ///
    /// Type: float
    pub const CARGO_DAMAGE: &[u8] = b"job.cargo.damage\0";

    /// All raw job channel names in official header order.
    pub const ALL: [&[u8]; COUNT] = [CARGO_DAMAGE];
}

/// Total number of channel-name macros covered by this module.
pub const COUNT: usize = common::COUNT + truck::COUNT + trailer::COUNT + job::COUNT;

const _: [(); 107] = [(); COUNT];
