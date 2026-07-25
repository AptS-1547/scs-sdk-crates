use core::ffi::{CStr, c_void};
use core::marker::PhantomData;
use core::ops::{BitOr, BitOrAssign};

use crate::{
    GameSchemaAvailability, SdkCall, SdkError, SdkIndex, SdkResult, SdkValue, ValueRef, ValueType,
    sys,
};

/// Value types which can be requested from an SCS telemetry channel.
///
/// The game validates whether a particular channel supports the requested
/// representation. For example, placement channels commonly accept placement,
/// vector, or Euler representations, while `u32` channels commonly accept
/// `u64`. Unsupported requests return [`SdkError::UnsupportedType`].
pub trait ChannelValue: SdkValue {}

impl<T: SdkValue> ChannelValue for T {}

#[derive(Debug, PartialEq, Eq)]
pub struct Channel<T> {
    name: &'static CStr,
    indexed: bool,
    availability: GameSchemaAvailability,
    marker: PhantomData<fn() -> T>,
}

/// A telemetry channel descriptor after erasing its Rust marker type.
///
/// Framework code needs a uniform representation because a plugin can
/// subscribe to channels carrying many different SDK value types. Erasure does
/// not discard the ABI type discriminator: [`AnyChannel::value_type`] remains
/// available and callback values must still carry that exact tag before typed
/// decoding succeeds.
///
/// Application plugins normally construct this value through
/// [`Channel::erase`] rather than directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnyChannel {
    name: &'static CStr,
    value_type: ValueType,
    indexed: bool,
    availability: GameSchemaAvailability,
}

impl AnyChannel {
    /// Returns the canonical, NUL-terminated channel name from the SDK header.
    #[must_use]
    pub const fn name(self) -> &'static CStr {
        self.name
    }

    /// Returns the concrete tagged-union member requested during registration.
    #[must_use]
    pub const fn value_type(self) -> ValueType {
        self.value_type
    }

    /// Whether the channel requires a separate zero-based SDK index.
    ///
    /// This index is used for arrays such as wheels. It is distinct from the
    /// trailer number embedded in a multi-trailer channel name.
    #[must_use]
    pub const fn is_indexed(self) -> bool {
        self.indexed
    }

    /// Returns the official per-game schema history for this channel.
    ///
    /// Type erasure retains availability so framework code can reject a
    /// required channel, or skip an optional channel, before asking an older
    /// game schema to register a name it cannot expose.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        self.availability
    }
}

impl<T> Clone for Channel<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Channel<T> {}

impl<T: ChannelValue> Channel<T> {
    #[must_use]
    pub const fn new(name: &'static CStr, availability: GameSchemaAvailability) -> Self {
        Self {
            name,
            indexed: false,
            availability,
            marker: PhantomData,
        }
    }

    /// Creates a descriptor whose values are selected through the SDK `index`
    /// parameter, such as individual wheel or H-shifter selector channels.
    #[must_use]
    pub const fn indexed(name: &'static CStr, availability: GameSchemaAvailability) -> Self {
        Self {
            name,
            indexed: true,
            availability,
            marker: PhantomData,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static CStr {
        self.name
    }

    #[must_use]
    pub const fn value_type(self) -> sys::ScsValueType {
        T::TYPE
    }

    /// Whether registration requires an explicit zero-based SDK index.
    #[must_use]
    pub const fn is_indexed(self) -> bool {
        self.indexed
    }

    /// Returns the first official ETS2 and ATS schemas for this channel.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        self.availability
    }

    /// Requests the same channel using another SDK value representation.
    ///
    /// SCS performs the authoritative compatibility check during registration
    /// and reports `SCS_RESULT_unsupported_type` when conversion is unavailable.
    #[must_use]
    pub const fn requesting<U: ChannelValue>(self) -> Channel<U> {
        Channel {
            name: self.name,
            indexed: self.indexed,
            availability: self.availability,
            marker: PhantomData,
        }
    }

    /// Erases the Rust marker type while preserving the SDK value discriminator.
    ///
    /// This is the representation used by the plugin framework's heterogeneous
    /// subscription table. A value can later be decoded with the original
    /// typed descriptor only when the names, index mode, and value types match.
    #[must_use]
    pub const fn erase(self) -> AnyChannel {
        AnyChannel {
            name: self.name,
            value_type: T::VALUE_TYPE,
            indexed: self.indexed,
            availability: self.availability,
        }
    }

    /// Decodes a callback value according to this descriptor's requested type.
    #[must_use]
    pub fn decode(self, value: ValueRef<'_>) -> Option<T::Decoded<'_>> {
        T::decode(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ChannelFlags(sys::ScsU32);

impl ChannelFlags {
    pub const NONE: Self = Self(sys::SCS_TELEMETRY_CHANNEL_FLAG_NONE);
    pub const EACH_FRAME: Self = Self(sys::SCS_TELEMETRY_CHANNEL_FLAG_EACH_FRAME);
    pub const NO_VALUE: Self = Self(sys::SCS_TELEMETRY_CHANNEL_FLAG_NO_VALUE);

    #[must_use]
    pub const fn bits(self) -> sys::ScsU32 {
        self.0
    }
}

impl BitOr for ChannelFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for ChannelFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl SdkCall<'_> {
    /// Registers a typed telemetry channel callback.
    ///
    /// # Safety
    ///
    /// `callback` must implement the exact SDK ABI, must decode the value
    /// according to `T`, and must not unwind across the ABI boundary. `context`
    /// must remain valid for every invocation of `callback`. This method must
    /// be called from initialization or an SCS event callback, never from a
    /// worker thread or another channel callback.
    ///
    /// # Errors
    ///
    /// Returns the result reported by the game's registration function.
    pub unsafe fn register_channel<T: ChannelValue>(
        &self,
        channel: Channel<T>,
        index: Option<SdkIndex>,
        flags: ChannelFlags,
        callback: sys::ScsTelemetryChannelCallback,
        context: *mut c_void,
    ) -> SdkResult {
        // SAFETY: The caller upholds the SDK callback, context, and call-site
        // restrictions documented above.
        let result = unsafe {
            (self.table.register_for_channel)(
                channel.name().as_ptr(),
                index.map_or(sys::SCS_U32_NIL, SdkIndex::raw),
                channel.value_type(),
                flags.bits(),
                callback,
                context,
            )
        };
        SdkError::from_code(result)
    }

    /// Unregisters a telemetry channel callback.
    ///
    /// # Safety
    ///
    /// Must be called from initialization, shutdown, or an SCS event callback,
    /// never from a channel callback or a worker thread. Context storage
    /// associated with the old registration must remain alive until this
    /// function succeeds and no invocation of that callback is active.
    ///
    /// # Errors
    ///
    /// Returns the result reported by the game's unregistration function.
    pub unsafe fn unregister_channel<T: ChannelValue>(
        &self,
        channel: Channel<T>,
        index: Option<SdkIndex>,
    ) -> SdkResult {
        // SAFETY: The caller upholds the SDK call-site restriction.
        let result = unsafe {
            (self.table.unregister_from_channel)(
                channel.name().as_ptr(),
                index.map_or(sys::SCS_U32_NIL, SdkIndex::raw),
                channel.value_type(),
            )
        };
        SdkError::from_code(result)
    }

    /// Registers a channel whose Rust marker type has been erased by a
    /// framework subscription table.
    ///
    /// Unlike [`SdkCall::register_channel`], the channel name may be owned by
    /// the caller. This is required for multi-trailer names such as
    /// `trailer.3.connected`, which do not exist as static macros in the C
    /// header.
    ///
    /// # Safety
    ///
    /// `callback` must implement the exact SDK ABI, verify every received value
    /// against `value_type`, and prevent unwinding across the ABI boundary.
    /// `name` and `context` must remain valid for the complete registration
    /// lifetime. This method may only be called at an SDK-approved registration
    /// point on the game main thread.
    ///
    /// # Errors
    ///
    /// Returns the exact result reported by the game's registration function.
    pub unsafe fn register_erased_channel(
        &self,
        name: &CStr,
        index: Option<SdkIndex>,
        value_type: ValueType,
        flags: ChannelFlags,
        callback: sys::ScsTelemetryChannelCallback,
        context: *mut c_void,
    ) -> SdkResult {
        // SAFETY: The caller owns the name/context lifetime and callback ABI
        // invariants documented above.
        let result = unsafe {
            (self.table.register_for_channel)(
                name.as_ptr(),
                index.map_or(sys::SCS_U32_NIL, SdkIndex::raw),
                value_type.raw(),
                flags.bits(),
                callback,
                context,
            )
        };
        SdkError::from_code(result)
    }

    /// Unregisters one previously registered type-erased channel.
    ///
    /// # Safety
    ///
    /// This must run at an SDK-approved unregistration point on the game main
    /// thread. The callback context and channel name must remain alive until
    /// unregistration succeeds and any active callback has returned.
    ///
    /// # Errors
    ///
    /// Returns the exact result reported by the game's unregistration function.
    pub unsafe fn unregister_erased_channel(
        &self,
        name: &CStr,
        index: Option<SdkIndex>,
        value_type: ValueType,
    ) -> SdkResult {
        // SAFETY: The caller upholds the SDK call-site and storage-lifetime
        // restrictions documented above.
        let result = unsafe {
            (self.table.unregister_from_channel)(
                name.as_ptr(),
                index.map_or(sys::SCS_U32_NIL, SdkIndex::raw),
                value_type.raw(),
            )
        };
        SdkError::from_code(result)
    }
}

/// Complete typed channel catalog for SCS Telemetry SDK 1.14.
pub mod channels {
    use super::AnyChannel;
    use crate::{GameSchemaAvailability, game};

    // These constants encode the game-schema history documented by the ETS2
    // and ATS headers. They are deliberately named by both version domains:
    // an ETS2 minor must never be reused as though it were the equivalent ATS
    // minor merely because both games expose the same common C macro.
    const INITIAL: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_00), Some(game::ats::V1_00));
    const ETS2_1_01_ATS_1_00: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_01), Some(game::ats::V1_00));
    const ETS2_1_02_ATS_1_00: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_02), Some(game::ats::V1_00));
    const ETS2_1_04_ATS_1_00: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_04), Some(game::ats::V1_00));
    const ETS2_1_09_ATS_1_00: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_09), Some(game::ats::V1_00));
    const ETS2_1_10_ATS_1_00: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_10), Some(game::ats::V1_00));
    const ETS2_1_11_ATS_1_00: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_11), Some(game::ats::V1_00));
    const ETS2_1_12_ATS_1_00: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_12), Some(game::ats::V1_00));
    const ETS2_ONLY_1_12: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_12), None);
    const ETS2_1_14_ATS_1_01: GameSchemaAvailability = game::capabilities::MULTI_TRAILER;
    const ETS2_1_17_ATS_1_04: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_17), Some(game::ats::V1_04));
    const ETS2_1_18_ATS_1_05: GameSchemaAvailability =
        GameSchemaAvailability::new(Some(game::ets2::V1_18), Some(game::ats::V1_05));

    /// First schema which supports the numbered `trailer.[index].*` namespace.
    ///
    /// The static `trailer.*` descriptors predate this feature. Framework code
    /// must combine their individual availability with this capability when it
    /// constructs a numbered multi-trailer channel name.
    pub const MULTI_TRAILER_AVAILABILITY: GameSchemaAvailability =
        game::capabilities::MULTI_TRAILER;

    /// Channels declared by `scssdk_telemetry_common_channels.h`.
    pub mod common {
        use super::super::{AnyChannel, Channel};
        use super::{ETS2_1_09_ATS_1_00, ETS2_1_18_ATS_1_05, INITIAL};

        /// Number of typed descriptors in this catalog group.
        pub const COUNT: usize = 4;

        /// Scale applied to distance and time to compensate
        /// for the scale of the map (e.g. 1s of real time corresponds to `local_scale`
        /// seconds of simulated game time).
        ///
        /// Games which use real 1:1 maps will not provide this
        /// channel.
        ///
        /// Type: float
        pub const LOCAL_SCALE: Channel<f32> = Channel::new(c"local.scale", INITIAL);

        /// Absolute in-game time.
        ///
        /// Represented in number of in-game minutes since beginning (i.e. 00:00)
        /// of the first in-game day.
        ///
        /// Type: u32
        pub const GAME_TIME: Channel<u32> = Channel::new(c"game.time", ETS2_1_09_ATS_1_00);

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
        pub const MULTIPLAYER_TIME_OFFSET: Channel<i32> =
            Channel::new(c"multiplayer.time.offset", ETS2_1_18_ATS_1_05);

        /// Time until next rest stop.
        ///
        /// When the fatique simulation is disabled, the behavior of this channel
        /// is implementation dependent. The game might provide the value which would
        /// apply if it was enabled or provide no value at all.
        ///
        /// Represented in in-game minutes.
        ///
        /// Type: s32
        pub const NEXT_REST_STOP: Channel<i32> = Channel::new(c"rest.stop", ETS2_1_09_ATS_1_00);

        /// Every common channel in the order used by the SDK 1.14 header.
        ///
        /// The marker types are erased only so callers can enumerate mixed
        /// channel types. Each entry still retains its exact value type and
        /// indexed/scalar registration mode.
        pub const ALL: [AnyChannel; COUNT] = [
            LOCAL_SCALE.erase(),
            GAME_TIME.erase(),
            MULTIPLAYER_TIME_OFFSET.erase(),
            NEXT_REST_STOP.erase(),
        ];
    }

    /// Channels declared by `scssdk_telemetry_truck_common_channels.h`.
    pub mod truck {
        use super::super::{AnyChannel, Channel};
        use super::{
            ETS2_1_01_ATS_1_00, ETS2_1_02_ATS_1_00, ETS2_1_04_ATS_1_00, ETS2_1_10_ATS_1_00,
            ETS2_1_11_ATS_1_00, ETS2_1_12_ATS_1_00, ETS2_1_17_ATS_1_04, ETS2_ONLY_1_12, INITIAL,
        };

        /// Number of typed descriptors in this catalog group.
        pub const COUNT: usize = 84;

        /// Represents world space position and orientation of the truck.
        ///
        /// Type: dplacement
        pub const WORLD_PLACEMENT: Channel<crate::DPlacement> =
            Channel::new(c"truck.world.placement", INITIAL);

        /// Represents vehicle space linear velocity of the truck measured
        /// in meters per second.
        ///
        /// Type: fvector
        pub const LOCAL_LINEAR_VELOCITY: Channel<crate::FVector> =
            Channel::new(c"truck.local.velocity.linear", INITIAL);

        /// Represents vehicle space angular velocity of the truck measured
        /// in rotations per second.
        ///
        /// Type: fvector
        pub const LOCAL_ANGULAR_VELOCITY: Channel<crate::FVector> =
            Channel::new(c"truck.local.velocity.angular", INITIAL);

        /// Represents vehicle space linear acceleration of the truck measured
        /// in meters per second^2
        ///
        /// Type: fvector
        pub const LOCAL_LINEAR_ACCELERATION: Channel<crate::FVector> =
            Channel::new(c"truck.local.acceleration.linear", INITIAL);

        /// Represents vehicle space angular acceleration of the truck meassured
        /// in rotations per second^2
        ///
        /// Type: fvector
        pub const LOCAL_ANGULAR_ACCELERATION: Channel<crate::FVector> =
            Channel::new(c"truck.local.acceleration.angular", INITIAL);

        /// Represents a vehicle space position and orientation delta
        /// of the cabin from its default position.
        ///
        /// Type: fplacement
        pub const CABIN_OFFSET: Channel<crate::FPlacement> =
            Channel::new(c"truck.cabin.offset", ETS2_1_02_ATS_1_00);

        /// Represents cabin space angular velocity of the cabin measured
        /// in rotations per second.
        ///
        /// Type: fvector
        pub const CABIN_ANGULAR_VELOCITY: Channel<crate::FVector> =
            Channel::new(c"truck.cabin.velocity.angular", INITIAL);

        /// Represents cabin space angular acceleration of the cabin
        /// measured in rotations per second^2
        ///
        /// Type: fvector
        pub const CABIN_ANGULAR_ACCELERATION: Channel<crate::FVector> =
            Channel::new(c"truck.cabin.acceleration.angular", INITIAL);

        /// Represents a cabin space position and orientation delta
        /// of the driver head from its default position.
        ///
        /// Note that this value might change rapidly as result of
        /// the user switching between cameras or camera presets.
        ///
        /// Type: fplacement
        pub const HEAD_OFFSET: Channel<crate::FPlacement> =
            Channel::new(c"truck.head.offset", INITIAL);

        /// Speedometer speed in meters per second.
        ///
        /// Uses negative value to represent reverse movement.
        ///
        /// Type: float
        pub const SPEED: Channel<f32> = Channel::new(c"truck.speed", INITIAL);

        /// RPM of the engine.
        ///
        /// Type: float
        pub const ENGINE_RPM: Channel<f32> = Channel::new(c"truck.engine.rpm", INITIAL);

        /// Gear currently selected in the engine.
        ///
        /// - >0 - Forwad gears
        /// - 0 - Neutral
        /// - <0 - Reverse gears
        ///
        /// Type: s32
        pub const ENGINE_GEAR: Channel<i32> = Channel::new(c"truck.engine.gear", INITIAL);

        /// Gear currently displayed on dashboard.
        ///
        /// - >0 - Forwad gears
        /// - 0 - Neutral
        /// - <0 - Reverse gears
        ///
        /// Type: s32
        pub const DISPLAYED_GEAR: Channel<i32> =
            Channel::new(c"truck.displayed.gear", ETS2_1_11_ATS_1_00);

        /// Steering received from input <-1;1>.
        ///
        /// Note that it is interpreted counterclockwise.
        ///
        /// If the user presses the steer right button on digital input
        /// (e.g. keyboard) this value goes immediatelly to -1.0
        ///
        /// Type: float
        pub const INPUT_STEERING: Channel<f32> = Channel::new(c"truck.input.steering", INITIAL);

        /// Throttle received from input <0;1>
        ///
        /// If the user presses the forward button on digital input
        /// (e.g. keyboard) this value goes immediatelly to 1.0
        ///
        /// Type: float
        pub const INPUT_THROTTLE: Channel<f32> = Channel::new(c"truck.input.throttle", INITIAL);

        /// Brake received from input <0;1>
        ///
        /// If the user presses the brake button on digital input
        /// (e.g. keyboard) this value goes immediatelly to 1.0
        ///
        /// Type: float
        pub const INPUT_BRAKE: Channel<f32> = Channel::new(c"truck.input.brake", INITIAL);

        /// Clutch received from input <0;1>
        ///
        /// If the user presses the clutch button on digital input
        /// (e.g. keyboard) this value goes immediatelly to 1.0
        ///
        /// Type: float
        pub const INPUT_CLUTCH: Channel<f32> = Channel::new(c"truck.input.clutch", INITIAL);

        /// Steering as used by the simulation <-1;1>
        ///
        /// Note that it is interpreted counterclockwise.
        ///
        /// Accounts for interpolation speeds and simulated
        /// counterfoces for digital inputs.
        ///
        /// Type: float
        pub const EFFECTIVE_STEERING: Channel<f32> =
            Channel::new(c"truck.effective.steering", INITIAL);

        /// Throttle pedal input as used by the simulation <0;1>
        ///
        /// Accounts for the press attack curve for digital inputs
        /// or cruise-control input.
        ///
        /// Type: float
        pub const EFFECTIVE_THROTTLE: Channel<f32> =
            Channel::new(c"truck.effective.throttle", INITIAL);

        /// Brake pedal input as used by the simulation <0;1>
        ///
        /// Accounts for the press attack curve for digital inputs. Does
        /// not contain retarder, parking or engine brake.
        ///
        /// Type: float
        pub const EFFECTIVE_BRAKE: Channel<f32> = Channel::new(c"truck.effective.brake", INITIAL);

        /// Clutch pedal input as used by the simulation <0;1>
        ///
        /// Accounts for the automatic shifting or interpolation of
        /// player input.
        ///
        /// Type: float
        pub const EFFECTIVE_CLUTCH: Channel<f32> = Channel::new(c"truck.effective.clutch", INITIAL);

        /// Speed selected for the cruise control in m/s
        ///
        /// Is zero if cruise control is disabled.
        ///
        /// Type: float
        pub const CRUISE_CONTROL: Channel<f32> = Channel::new(c"truck.cruise_control", INITIAL);

        /// Gearbox slot the h-shifter handle is currently in.
        ///
        /// 0 means that no slot is selected.
        ///
        /// Type: u32
        pub const HSHIFTER_SLOT: Channel<u32> = Channel::new(c"truck.hshifter.slot", INITIAL);

        /// Enabled state of range/splitter selector toggles.
        ///
        /// Mapping between the range/splitter functionality and
        /// selector index is described by HSHIFTER configuration.
        ///
        /// Type: indexed bool
        pub const HSHIFTER_SELECTOR: Channel<bool> =
            Channel::indexed(c"truck.hshifter.select", INITIAL);

        /// Is the parking brake enabled?
        ///
        /// Type: bool
        pub const PARKING_BRAKE: Channel<bool> = Channel::new(c"truck.brake.parking", INITIAL);

        /// Is the engine brake enabled?
        ///
        /// Type: bool
        pub const MOTOR_BRAKE: Channel<bool> = Channel::new(c"truck.brake.motor", INITIAL);

        /// Current level of the retarder.
        ///
        /// <0;max> where 0 is disabled retarder and max is maximum
        /// value found in TRUCK configuration.
        ///
        /// Type: u32
        pub const RETARDER_LEVEL: Channel<u32> = Channel::new(c"truck.brake.retarder", INITIAL);

        /// Pressure in the brake air tank in psi
        ///
        /// Type: float
        pub const BRAKE_AIR_PRESSURE: Channel<f32> =
            Channel::new(c"truck.brake.air.pressure", INITIAL);

        /// Is the air pressure warning active?
        ///
        /// Type: bool
        pub const BRAKE_AIR_PRESSURE_WARNING: Channel<bool> =
            Channel::new(c"truck.brake.air.pressure.warning", INITIAL);

        /// Are the emergency brakes active as result of low air pressure?
        ///
        /// Type: bool
        pub const BRAKE_AIR_PRESSURE_EMERGENCY: Channel<bool> =
            Channel::new(c"truck.brake.air.pressure.emergency", ETS2_1_01_ATS_1_00);

        /// Temperature of the brakes in degrees celsius.
        ///
        /// Aproximated for entire truck, not at the wheel level.
        ///
        /// Type: float
        pub const BRAKE_TEMPERATURE: Channel<f32> =
            Channel::new(c"truck.brake.temperature", INITIAL);

        /// Amount of fuel in liters
        ///
        /// Type: float
        pub const FUEL: Channel<f32> = Channel::new(c"truck.fuel.amount", INITIAL);

        /// Is the low fuel warning active?
        ///
        /// Type: bool
        pub const FUEL_WARNING: Channel<bool> = Channel::new(c"truck.fuel.warning", INITIAL);

        /// Average consumption of the fuel in liters/km
        ///
        /// Type: float
        pub const FUEL_AVERAGE_CONSUMPTION: Channel<f32> =
            Channel::new(c"truck.fuel.consumption.average", INITIAL);

        /// Estimated range of truck with current amount of fuel in km
        ///
        /// Type: float
        pub const FUEL_RANGE: Channel<f32> = Channel::new(c"truck.fuel.range", ETS2_1_12_ATS_1_00);

        /// Amount of `AdBlue` in liters
        ///
        /// Type: float
        pub const ADBLUE: Channel<f32> = Channel::new(c"truck.adblue", ETS2_ONLY_1_12);

        /// Is the low adblue warning active?
        ///
        /// Type: bool
        pub const ADBLUE_WARNING: Channel<bool> =
            Channel::new(c"truck.adblue.warning", ETS2_ONLY_1_12);

        /// Average consumption of the adblue in liters/km
        ///
        /// Type: float
        pub const ADBLUE_AVERAGE_CONSUMPTION: Channel<f32> =
            Channel::new(c"truck.adblue.consumption.average", ETS2_ONLY_1_12);

        /// Pressure of the oil in psi
        ///
        /// Type: float
        pub const OIL_PRESSURE: Channel<f32> = Channel::new(c"truck.oil.pressure", INITIAL);

        /// Is the oil pressure warning active?
        ///
        /// Type: bool
        pub const OIL_PRESSURE_WARNING: Channel<bool> =
            Channel::new(c"truck.oil.pressure.warning", INITIAL);

        /// Temperature of the oil in degrees celsius.
        ///
        /// Type: float
        pub const OIL_TEMPERATURE: Channel<f32> = Channel::new(c"truck.oil.temperature", INITIAL);

        /// Temperature of the water in degrees celsius.
        ///
        /// Type: float
        pub const WATER_TEMPERATURE: Channel<f32> =
            Channel::new(c"truck.water.temperature", INITIAL);

        /// Is the water temperature warning active?
        ///
        /// Type: bool
        pub const WATER_TEMPERATURE_WARNING: Channel<bool> =
            Channel::new(c"truck.water.temperature.warning", INITIAL);

        /// Voltage of the battery in volts.
        ///
        /// Type: float
        pub const BATTERY_VOLTAGE: Channel<f32> = Channel::new(c"truck.battery.voltage", INITIAL);

        /// Is the battery voltage/not charging warning active?
        ///
        /// Type: bool
        pub const BATTERY_VOLTAGE_WARNING: Channel<bool> =
            Channel::new(c"truck.battery.voltage.warning", INITIAL);

        /// Is the electric enabled?
        ///
        /// Type: bool
        pub const ELECTRIC_ENABLED: Channel<bool> =
            Channel::new(c"truck.electric.enabled", INITIAL);

        /// Is the engine enabled?
        ///
        /// Type: bool
        pub const ENGINE_ENABLED: Channel<bool> = Channel::new(c"truck.engine.enabled", INITIAL);

        /// Is the left blinker enabled?
        ///
        /// This represents the logical enable state of the blinker. It
        /// it is true as long the blinker is enabled regardless of the
        /// physical enabled state of the light (i.e. it does not blink
        /// and ignores enable state of electric).
        ///
        /// Type: bool
        pub const LBLINKER: Channel<bool> = Channel::new(c"truck.lblinker", INITIAL);

        /// Is the right blinker enabled?
        ///
        /// This represents the logical enable state of the blinker. It
        /// it is true as long the blinker is enabled regardless of the
        /// physical enabled state of the light (i.e. it does not blink
        /// and ignores enable state of electric).
        ///
        /// Type: bool
        pub const RBLINKER: Channel<bool> = Channel::new(c"truck.rblinker", INITIAL);

        /// Are the hazard warning light enabled?
        ///
        /// This represents the logical enable state of the hazard warning.
        /// It it is true as long it is enabled regardless of the physical
        /// enabled state of the light (i.e. it does not blink).
        ///
        /// Type: bool
        pub const HAZARD_WARNING: Channel<bool> =
            Channel::new(c"truck.hazard.warning", ETS2_1_17_ATS_1_04);

        /// Is the light in the left blinker currently on?
        ///
        /// Type: bool
        pub const LIGHT_LBLINKER: Channel<bool> =
            Channel::new(c"truck.light.lblinker", ETS2_1_04_ATS_1_00);

        /// Is the light in the right blinker currently on?
        ///
        /// Type: bool
        pub const LIGHT_RBLINKER: Channel<bool> =
            Channel::new(c"truck.light.rblinker", ETS2_1_04_ATS_1_00);

        /// Are the parking lights enabled?
        ///
        /// Type: bool
        pub const LIGHT_PARKING: Channel<bool> = Channel::new(c"truck.light.parking", INITIAL);

        /// Are the low beam lights enabled?
        ///
        /// Type: bool
        pub const LIGHT_LOW_BEAM: Channel<bool> = Channel::new(c"truck.light.beam.low", INITIAL);

        /// Are the high beam lights enabled?
        ///
        /// Type: bool
        pub const LIGHT_HIGH_BEAM: Channel<bool> = Channel::new(c"truck.light.beam.high", INITIAL);

        /// Are the auxiliary front lights active?
        ///
        /// Those lights have several intensity levels:
        /// - 1 - dimmed state
        /// - 2 - full state
        ///
        /// Type: u32
        pub const LIGHT_AUX_FRONT: Channel<u32> = Channel::new(c"truck.light.aux.front", INITIAL);

        /// Are the auxiliary roof lights active?
        ///
        /// Those lights have several intensity levels:
        /// - 1 - dimmed state
        /// - 2 - full state
        ///
        /// Type: u32
        pub const LIGHT_AUX_ROOF: Channel<u32> = Channel::new(c"truck.light.aux.roof", INITIAL);

        /// Are the beacon lights enabled?
        ///
        /// Type: bool
        pub const LIGHT_BEACON: Channel<bool> = Channel::new(c"truck.light.beacon", INITIAL);

        /// Is the brake light active?
        ///
        /// Type: bool
        pub const LIGHT_BRAKE: Channel<bool> = Channel::new(c"truck.light.brake", INITIAL);

        /// Is the reverse light active?
        ///
        /// Type: bool
        pub const LIGHT_REVERSE: Channel<bool> = Channel::new(c"truck.light.reverse", INITIAL);

        /// Are the wipers enabled?
        ///
        /// Type: bool
        pub const WIPERS: Channel<bool> = Channel::new(c"truck.wipers", INITIAL);

        /// Intensity of the dashboard backlight as factor <0;1>
        ///
        /// Type: float
        pub const DASHBOARD_BACKLIGHT: Channel<f32> =
            Channel::new(c"truck.dashboard.backlight", INITIAL);

        /// Is the differential lock enabled?
        ///
        /// Type: bool
        pub const DIFFERENTIAL_LOCK: Channel<bool> =
            Channel::new(c"truck.differential_lock", ETS2_1_17_ATS_1_04);

        /// Is the lift axle control set to lifted state?
        ///
        /// Type: bool
        pub const LIFT_AXLE: Channel<bool> = Channel::new(c"truck.lift_axle", ETS2_1_17_ATS_1_04);

        /// Is the lift axle indicator lit?
        ///
        /// Type: bool
        pub const LIFT_AXLE_INDICATOR: Channel<bool> =
            Channel::new(c"truck.lift_axle.indicator", ETS2_1_17_ATS_1_04);

        /// Is the trailer lift axle control set to lifted state?
        ///
        /// Type: bool
        pub const TRAILER_LIFT_AXLE: Channel<bool> =
            Channel::new(c"truck.trailer.lift_axle", ETS2_1_17_ATS_1_04);

        /// Is the trailer lift axle indicator lit?
        ///
        /// Type: bool
        pub const TRAILER_LIFT_AXLE_INDICATOR: Channel<bool> =
            Channel::new(c"truck.trailer.lift_axle.indicator", ETS2_1_17_ATS_1_04);

        /// Wear of the engine accessory as <0;1>
        ///
        /// Type: float
        pub const WEAR_ENGINE: Channel<f32> = Channel::new(c"truck.wear.engine", INITIAL);

        /// Wear of the transmission accessory as <0;1>
        ///
        /// Type: float
        pub const WEAR_TRANSMISSION: Channel<f32> =
            Channel::new(c"truck.wear.transmission", INITIAL);

        /// Wear of the cabin accessory as <0;1>
        ///
        /// Type: float
        pub const WEAR_CABIN: Channel<f32> = Channel::new(c"truck.wear.cabin", INITIAL);

        /// Wear of the chassis accessory as <0;1>
        ///
        /// Type: float
        pub const WEAR_CHASSIS: Channel<f32> = Channel::new(c"truck.wear.chassis", INITIAL);

        /// Average wear across the wheel accessories as <0;1>
        ///
        /// Type: float
        pub const WEAR_WHEELS: Channel<f32> = Channel::new(c"truck.wear.wheels", INITIAL);

        /// The value of the odometer in km.
        ///
        /// Type: float
        pub const ODOMETER: Channel<f32> = Channel::new(c"truck.odometer", INITIAL);

        /// The value of truck's navigation distance (in meters).
        ///
        /// This is the value used by the advisor.
        ///
        /// Type: float
        pub const NAVIGATION_DISTANCE: Channel<f32> =
            Channel::new(c"truck.navigation.distance", ETS2_1_12_ATS_1_00);

        /// The value of truck's navigation eta (in second).
        ///
        /// This is the value used by the advisor.
        ///
        /// Type: float
        pub const NAVIGATION_TIME: Channel<f32> =
            Channel::new(c"truck.navigation.time", ETS2_1_12_ATS_1_00);

        /// The value of truck's navigation speed limit (in m/s).
        ///
        /// This is the value used by the advisor and respects the
        /// current state of the "Route Advisor speed limit" option.
        ///
        /// Type: float
        pub const NAVIGATION_SPEED_LIMIT: Channel<f32> =
            Channel::new(c"truck.navigation.speed.limit", ETS2_1_12_ATS_1_00);

        /// Vertical displacement of the wheel from its
        /// axis in meters.
        ///
        /// Type: indexed float
        pub const WHEEL_SUSP_DEFLECTION: Channel<f32> =
            Channel::indexed(c"truck.wheel.suspension.deflection", INITIAL);

        /// Is the wheel in contact with ground?
        ///
        /// Type: indexed bool
        pub const WHEEL_ON_GROUND: Channel<bool> =
            Channel::indexed(c"truck.wheel.on_ground", INITIAL);

        /// Substance below the whell.
        ///
        /// Index of substance as delivered trough SUBSTANCE config.
        ///
        /// Type: indexed u32
        pub const WHEEL_SUBSTANCE: Channel<u32> =
            Channel::indexed(c"truck.wheel.substance", INITIAL);

        /// Angular velocity of the wheel in rotations per
        /// second.
        ///
        /// Positive velocity corresponds to forward movement.
        ///
        /// Type: indexed float
        pub const WHEEL_VELOCITY: Channel<f32> =
            Channel::indexed(c"truck.wheel.angular_velocity", INITIAL);

        /// Steering rotation of the wheel in rotations.
        ///
        /// Value is from <-0.25,0.25> range in counterclockwise direction
        /// when looking from top (e.g. 0.25 corresponds to left and
        /// -0.25 corresponds to right).
        ///
        /// Set to zero for non-steered wheels.
        ///
        /// Type: indexed float
        pub const WHEEL_STEERING: Channel<f32> = Channel::indexed(c"truck.wheel.steering", INITIAL);

        /// Rolling rotation of the wheel in rotations.
        ///
        /// Value is from <0.0,1.0) range in which value
        /// increase corresponds to forward movement.
        ///
        /// Type: indexed float
        pub const WHEEL_ROTATION: Channel<f32> = Channel::indexed(c"truck.wheel.rotation", INITIAL);

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
        pub const WHEEL_LIFT: Channel<f32> =
            Channel::indexed(c"truck.wheel.lift", ETS2_1_10_ATS_1_00);

        /// Vertical displacement of the wheel axle
        /// from its normal position in meters as result of
        /// lifting.
        ///
        /// Might have non-linear relation to lift ratio.
        ///
        /// Set to zero or not provided for non-liftable axles.
        ///
        /// Type: indexed float
        pub const WHEEL_LIFT_OFFSET: Channel<f32> =
            Channel::indexed(c"truck.wheel.lift.offset", ETS2_1_10_ATS_1_00);

        /// Every truck channel in the order used by the SDK 1.14 header.
        ///
        /// Indexed wheel and H-shifter entries keep `is_indexed() == true`;
        /// the array itself never supplies or guesses a runtime index.
        pub const ALL: [AnyChannel; COUNT] = [
            WORLD_PLACEMENT.erase(),
            LOCAL_LINEAR_VELOCITY.erase(),
            LOCAL_ANGULAR_VELOCITY.erase(),
            LOCAL_LINEAR_ACCELERATION.erase(),
            LOCAL_ANGULAR_ACCELERATION.erase(),
            CABIN_OFFSET.erase(),
            CABIN_ANGULAR_VELOCITY.erase(),
            CABIN_ANGULAR_ACCELERATION.erase(),
            HEAD_OFFSET.erase(),
            SPEED.erase(),
            ENGINE_RPM.erase(),
            ENGINE_GEAR.erase(),
            DISPLAYED_GEAR.erase(),
            INPUT_STEERING.erase(),
            INPUT_THROTTLE.erase(),
            INPUT_BRAKE.erase(),
            INPUT_CLUTCH.erase(),
            EFFECTIVE_STEERING.erase(),
            EFFECTIVE_THROTTLE.erase(),
            EFFECTIVE_BRAKE.erase(),
            EFFECTIVE_CLUTCH.erase(),
            CRUISE_CONTROL.erase(),
            HSHIFTER_SLOT.erase(),
            HSHIFTER_SELECTOR.erase(),
            PARKING_BRAKE.erase(),
            MOTOR_BRAKE.erase(),
            RETARDER_LEVEL.erase(),
            BRAKE_AIR_PRESSURE.erase(),
            BRAKE_AIR_PRESSURE_WARNING.erase(),
            BRAKE_AIR_PRESSURE_EMERGENCY.erase(),
            BRAKE_TEMPERATURE.erase(),
            FUEL.erase(),
            FUEL_WARNING.erase(),
            FUEL_AVERAGE_CONSUMPTION.erase(),
            FUEL_RANGE.erase(),
            ADBLUE.erase(),
            ADBLUE_WARNING.erase(),
            ADBLUE_AVERAGE_CONSUMPTION.erase(),
            OIL_PRESSURE.erase(),
            OIL_PRESSURE_WARNING.erase(),
            OIL_TEMPERATURE.erase(),
            WATER_TEMPERATURE.erase(),
            WATER_TEMPERATURE_WARNING.erase(),
            BATTERY_VOLTAGE.erase(),
            BATTERY_VOLTAGE_WARNING.erase(),
            ELECTRIC_ENABLED.erase(),
            ENGINE_ENABLED.erase(),
            LBLINKER.erase(),
            RBLINKER.erase(),
            HAZARD_WARNING.erase(),
            LIGHT_LBLINKER.erase(),
            LIGHT_RBLINKER.erase(),
            LIGHT_PARKING.erase(),
            LIGHT_LOW_BEAM.erase(),
            LIGHT_HIGH_BEAM.erase(),
            LIGHT_AUX_FRONT.erase(),
            LIGHT_AUX_ROOF.erase(),
            LIGHT_BEACON.erase(),
            LIGHT_BRAKE.erase(),
            LIGHT_REVERSE.erase(),
            WIPERS.erase(),
            DASHBOARD_BACKLIGHT.erase(),
            DIFFERENTIAL_LOCK.erase(),
            LIFT_AXLE.erase(),
            LIFT_AXLE_INDICATOR.erase(),
            TRAILER_LIFT_AXLE.erase(),
            TRAILER_LIFT_AXLE_INDICATOR.erase(),
            WEAR_ENGINE.erase(),
            WEAR_TRANSMISSION.erase(),
            WEAR_CABIN.erase(),
            WEAR_CHASSIS.erase(),
            WEAR_WHEELS.erase(),
            ODOMETER.erase(),
            NAVIGATION_DISTANCE.erase(),
            NAVIGATION_TIME.erase(),
            NAVIGATION_SPEED_LIMIT.erase(),
            WHEEL_SUSP_DEFLECTION.erase(),
            WHEEL_ON_GROUND.erase(),
            WHEEL_SUBSTANCE.erase(),
            WHEEL_VELOCITY.erase(),
            WHEEL_STEERING.erase(),
            WHEEL_ROTATION.erase(),
            WHEEL_LIFT.erase(),
            WHEEL_LIFT_OFFSET.erase(),
        ];
    }

    /// Channels declared by `scssdk_telemetry_trailer_common_channels.h`.
    pub mod trailer {
        use super::super::{AnyChannel, Channel};
        use super::{ETS2_1_14_ATS_1_01, ETS2_1_18_ATS_1_05, INITIAL};

        /// Number of typed descriptors in this catalog group.
        pub const COUNT: usize = 18;

        /// Is the trailer connected to the truck?
        ///
        /// Type: bool
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const CONNECTED: Channel<bool> = Channel::new(c"trailer.connected", INITIAL);

        /// How much is the cargo damaged that is loaded to this trailer in <0.0, 1.0> range.
        ///
        /// Type: float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const CARGO_DAMAGE: Channel<f32> =
            Channel::new(c"trailer.cargo.damage", ETS2_1_14_ATS_1_01);

        /// world placement trailer telemetry.
        ///
        /// Type: dplacement
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WORLD_PLACEMENT: Channel<crate::DPlacement> =
            Channel::new(c"trailer.world.placement", INITIAL);

        /// local linear velocity trailer telemetry.
        ///
        /// Type: fvector
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const LOCAL_LINEAR_VELOCITY: Channel<crate::FVector> =
            Channel::new(c"trailer.velocity.linear", INITIAL);

        /// local angular velocity trailer telemetry.
        ///
        /// Type: fvector
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const LOCAL_ANGULAR_VELOCITY: Channel<crate::FVector> =
            Channel::new(c"trailer.velocity.angular", INITIAL);

        /// local linear acceleration trailer telemetry.
        ///
        /// Type: fvector
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const LOCAL_LINEAR_ACCELERATION: Channel<crate::FVector> =
            Channel::new(c"trailer.acceleration.linear", INITIAL);

        /// local angular acceleration trailer telemetry.
        ///
        /// Type: fvector
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const LOCAL_ANGULAR_ACCELERATION: Channel<crate::FVector> =
            Channel::new(c"trailer.acceleration.angular", INITIAL);

        /// wear body trailer telemetry.
        ///
        /// Type: float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WEAR_BODY: Channel<f32> = Channel::new(c"trailer.wear.body", ETS2_1_18_ATS_1_05);

        /// wear chassis trailer telemetry.
        ///
        /// Type: float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WEAR_CHASSIS: Channel<f32> = Channel::new(c"trailer.wear.chassis", INITIAL);

        /// wear wheels trailer telemetry.
        ///
        /// Type: float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WEAR_WHEELS: Channel<f32> =
            Channel::new(c"trailer.wear.wheels", ETS2_1_14_ATS_1_01);

        /// wheel susp deflection trailer telemetry.
        ///
        /// Type: indexed float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WHEEL_SUSP_DEFLECTION: Channel<f32> =
            Channel::indexed(c"trailer.wheel.suspension.deflection", INITIAL);

        /// wheel on ground trailer telemetry.
        ///
        /// Type: indexed bool
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WHEEL_ON_GROUND: Channel<bool> =
            Channel::indexed(c"trailer.wheel.on_ground", INITIAL);

        /// wheel substance trailer telemetry.
        ///
        /// Type: indexed u32
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WHEEL_SUBSTANCE: Channel<u32> =
            Channel::indexed(c"trailer.wheel.substance", INITIAL);

        /// wheel velocity trailer telemetry.
        ///
        /// Type: indexed float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WHEEL_VELOCITY: Channel<f32> =
            Channel::indexed(c"trailer.wheel.angular_velocity", INITIAL);

        /// wheel steering trailer telemetry.
        ///
        /// Type: indexed float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WHEEL_STEERING: Channel<f32> =
            Channel::indexed(c"trailer.wheel.steering", INITIAL);

        /// wheel rotation trailer telemetry.
        ///
        /// Type: indexed float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WHEEL_ROTATION: Channel<f32> =
            Channel::indexed(c"trailer.wheel.rotation", INITIAL);

        /// wheel lift trailer telemetry.
        ///
        /// Type: indexed float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WHEEL_LIFT: Channel<f32> =
            Channel::indexed(c"trailer.wheel.lift", ETS2_1_14_ATS_1_01);

        /// wheel lift offset trailer telemetry.
        ///
        /// Type: indexed float
        ///
        /// For trailer indices above zero, the plugin framework derives the
        /// `trailer.[index].*` channel name described by the official SDK.
        pub const WHEEL_LIFT_OFFSET: Channel<f32> =
            Channel::indexed(c"trailer.wheel.lift.offset", ETS2_1_14_ATS_1_01);

        /// Every first-trailer channel in SDK 1.14 header order.
        ///
        /// These canonical names use the backward-compatible `trailer.*`
        /// form. Multi-trailer subscription remains an explicit framework
        /// operation which derives `trailer.[index].*` for indices 0 through
        /// 9; catalog enumeration itself performs no such expansion.
        pub const ALL: [AnyChannel; COUNT] = [
            CONNECTED.erase(),
            CARGO_DAMAGE.erase(),
            WORLD_PLACEMENT.erase(),
            LOCAL_LINEAR_VELOCITY.erase(),
            LOCAL_ANGULAR_VELOCITY.erase(),
            LOCAL_LINEAR_ACCELERATION.erase(),
            LOCAL_ANGULAR_ACCELERATION.erase(),
            WEAR_BODY.erase(),
            WEAR_CHASSIS.erase(),
            WEAR_WHEELS.erase(),
            WHEEL_SUSP_DEFLECTION.erase(),
            WHEEL_ON_GROUND.erase(),
            WHEEL_SUBSTANCE.erase(),
            WHEEL_VELOCITY.erase(),
            WHEEL_STEERING.erase(),
            WHEEL_ROTATION.erase(),
            WHEEL_LIFT.erase(),
            WHEEL_LIFT_OFFSET.erase(),
        ];
    }

    /// Channels declared by `scssdk_telemetry_job_common_channels.h`.
    pub mod job {
        use super::super::{AnyChannel, Channel};
        use super::ETS2_1_14_ATS_1_01;

        /// Number of typed descriptors in this catalog group.
        pub const COUNT: usize = 1;

        /// The total damage of the cargo in range 0.0 to 1.0.
        ///
        /// Type: float
        pub const CARGO_DAMAGE: Channel<f32> =
            Channel::new(c"job.cargo.damage", ETS2_1_14_ATS_1_01);

        /// Every job channel in SDK 1.14 header order.
        pub const ALL: [AnyChannel; COUNT] = [CARGO_DAMAGE.erase()];
    }

    /// Total number of typed descriptors in the catalog.
    pub const COUNT: usize = common::COUNT + truck::COUNT + trailer::COUNT + job::COUNT;

    /// Every public telemetry channel from the SDK 1.14 header bundle.
    ///
    /// Entries follow header grouping and declaration order: common, truck,
    /// trailer, then job. The catalog is descriptive only. Merely iterating it
    /// never registers anything; plugin code must still call the appropriate
    /// explicit `PluginContext::subscribe*` method for every desired channel,
    /// index, trailer number, delivery flag, and requested representation.
    pub const ALL: [AnyChannel; COUNT] = [
        common::LOCAL_SCALE.erase(),
        common::GAME_TIME.erase(),
        common::MULTIPLAYER_TIME_OFFSET.erase(),
        common::NEXT_REST_STOP.erase(),
        truck::WORLD_PLACEMENT.erase(),
        truck::LOCAL_LINEAR_VELOCITY.erase(),
        truck::LOCAL_ANGULAR_VELOCITY.erase(),
        truck::LOCAL_LINEAR_ACCELERATION.erase(),
        truck::LOCAL_ANGULAR_ACCELERATION.erase(),
        truck::CABIN_OFFSET.erase(),
        truck::CABIN_ANGULAR_VELOCITY.erase(),
        truck::CABIN_ANGULAR_ACCELERATION.erase(),
        truck::HEAD_OFFSET.erase(),
        truck::SPEED.erase(),
        truck::ENGINE_RPM.erase(),
        truck::ENGINE_GEAR.erase(),
        truck::DISPLAYED_GEAR.erase(),
        truck::INPUT_STEERING.erase(),
        truck::INPUT_THROTTLE.erase(),
        truck::INPUT_BRAKE.erase(),
        truck::INPUT_CLUTCH.erase(),
        truck::EFFECTIVE_STEERING.erase(),
        truck::EFFECTIVE_THROTTLE.erase(),
        truck::EFFECTIVE_BRAKE.erase(),
        truck::EFFECTIVE_CLUTCH.erase(),
        truck::CRUISE_CONTROL.erase(),
        truck::HSHIFTER_SLOT.erase(),
        truck::HSHIFTER_SELECTOR.erase(),
        truck::PARKING_BRAKE.erase(),
        truck::MOTOR_BRAKE.erase(),
        truck::RETARDER_LEVEL.erase(),
        truck::BRAKE_AIR_PRESSURE.erase(),
        truck::BRAKE_AIR_PRESSURE_WARNING.erase(),
        truck::BRAKE_AIR_PRESSURE_EMERGENCY.erase(),
        truck::BRAKE_TEMPERATURE.erase(),
        truck::FUEL.erase(),
        truck::FUEL_WARNING.erase(),
        truck::FUEL_AVERAGE_CONSUMPTION.erase(),
        truck::FUEL_RANGE.erase(),
        truck::ADBLUE.erase(),
        truck::ADBLUE_WARNING.erase(),
        truck::ADBLUE_AVERAGE_CONSUMPTION.erase(),
        truck::OIL_PRESSURE.erase(),
        truck::OIL_PRESSURE_WARNING.erase(),
        truck::OIL_TEMPERATURE.erase(),
        truck::WATER_TEMPERATURE.erase(),
        truck::WATER_TEMPERATURE_WARNING.erase(),
        truck::BATTERY_VOLTAGE.erase(),
        truck::BATTERY_VOLTAGE_WARNING.erase(),
        truck::ELECTRIC_ENABLED.erase(),
        truck::ENGINE_ENABLED.erase(),
        truck::LBLINKER.erase(),
        truck::RBLINKER.erase(),
        truck::HAZARD_WARNING.erase(),
        truck::LIGHT_LBLINKER.erase(),
        truck::LIGHT_RBLINKER.erase(),
        truck::LIGHT_PARKING.erase(),
        truck::LIGHT_LOW_BEAM.erase(),
        truck::LIGHT_HIGH_BEAM.erase(),
        truck::LIGHT_AUX_FRONT.erase(),
        truck::LIGHT_AUX_ROOF.erase(),
        truck::LIGHT_BEACON.erase(),
        truck::LIGHT_BRAKE.erase(),
        truck::LIGHT_REVERSE.erase(),
        truck::WIPERS.erase(),
        truck::DASHBOARD_BACKLIGHT.erase(),
        truck::DIFFERENTIAL_LOCK.erase(),
        truck::LIFT_AXLE.erase(),
        truck::LIFT_AXLE_INDICATOR.erase(),
        truck::TRAILER_LIFT_AXLE.erase(),
        truck::TRAILER_LIFT_AXLE_INDICATOR.erase(),
        truck::WEAR_ENGINE.erase(),
        truck::WEAR_TRANSMISSION.erase(),
        truck::WEAR_CABIN.erase(),
        truck::WEAR_CHASSIS.erase(),
        truck::WEAR_WHEELS.erase(),
        truck::ODOMETER.erase(),
        truck::NAVIGATION_DISTANCE.erase(),
        truck::NAVIGATION_TIME.erase(),
        truck::NAVIGATION_SPEED_LIMIT.erase(),
        truck::WHEEL_SUSP_DEFLECTION.erase(),
        truck::WHEEL_ON_GROUND.erase(),
        truck::WHEEL_SUBSTANCE.erase(),
        truck::WHEEL_VELOCITY.erase(),
        truck::WHEEL_STEERING.erase(),
        truck::WHEEL_ROTATION.erase(),
        truck::WHEEL_LIFT.erase(),
        truck::WHEEL_LIFT_OFFSET.erase(),
        trailer::CONNECTED.erase(),
        trailer::CARGO_DAMAGE.erase(),
        trailer::WORLD_PLACEMENT.erase(),
        trailer::LOCAL_LINEAR_VELOCITY.erase(),
        trailer::LOCAL_ANGULAR_VELOCITY.erase(),
        trailer::LOCAL_LINEAR_ACCELERATION.erase(),
        trailer::LOCAL_ANGULAR_ACCELERATION.erase(),
        trailer::WEAR_BODY.erase(),
        trailer::WEAR_CHASSIS.erase(),
        trailer::WEAR_WHEELS.erase(),
        trailer::WHEEL_SUSP_DEFLECTION.erase(),
        trailer::WHEEL_ON_GROUND.erase(),
        trailer::WHEEL_SUBSTANCE.erase(),
        trailer::WHEEL_VELOCITY.erase(),
        trailer::WHEEL_STEERING.erase(),
        trailer::WHEEL_ROTATION.erase(),
        trailer::WHEEL_LIFT.erase(),
        trailer::WHEEL_LIFT_OFFSET.erase(),
        job::CARGO_DAMAGE.erase(),
    ];

    pub use common::{GAME_TIME, LOCAL_SCALE, MULTIPLAYER_TIME_OFFSET, NEXT_REST_STOP};
    pub use job::CARGO_DAMAGE as JOB_CARGO_DAMAGE;
    pub use truck::{
        ENGINE_GEAR as TRUCK_ENGINE_GEAR, ENGINE_RPM as TRUCK_ENGINE_RPM,
        NAVIGATION_DISTANCE as TRUCK_NAVIGATION_DISTANCE,
        NAVIGATION_SPEED_LIMIT as TRUCK_NAVIGATION_SPEED_LIMIT,
        NAVIGATION_TIME as TRUCK_NAVIGATION_TIME, SPEED as TRUCK_SPEED,
        WORLD_PLACEMENT as TRUCK_WORLD_PLACEMENT,
    };

    const _: [(); 107] = [(); COUNT];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameSchemaVersion;

    fn assert_since<T: ChannelValue>(
        channel: Channel<T>,
        ets2: Option<GameSchemaVersion>,
        ats: Option<GameSchemaVersion>,
    ) {
        assert_eq!(
            channel.availability().available_since_ets2(),
            ets2,
            "{:?}",
            channel.name()
        );
        assert_eq!(
            channel.availability().available_since_ats(),
            ats,
            "{:?}",
            channel.name()
        );
    }

    fn assert_catalog_matches_raw(typed: &[AnyChannel], raw: &[&[u8]]) {
        assert_eq!(typed.len(), raw.len());
        for (descriptor, raw_name) in typed.iter().zip(raw) {
            assert_eq!(descriptor.name().to_bytes_with_nul(), *raw_name);
        }
    }

    #[test]
    fn typed_channels_select_the_sdk_value_type() {
        assert_eq!(
            channels::TRUCK_SPEED.value_type(),
            sys::SCS_VALUE_TYPE_FLOAT
        );
        assert_eq!(
            channels::TRUCK_ENGINE_GEAR.value_type(),
            sys::SCS_VALUE_TYPE_S32
        );
        assert_eq!(
            channels::TRUCK_WORLD_PLACEMENT.value_type(),
            sys::SCS_VALUE_TYPE_DPLACEMENT
        );
    }

    #[test]
    fn channel_flags_match_the_header_bits() {
        assert_eq!(ChannelFlags::EACH_FRAME.bits(), 1);
        assert_eq!(ChannelFlags::NO_VALUE.bits(), 2);
        assert_eq!(
            (ChannelFlags::EACH_FRAME | ChannelFlags::NO_VALUE).bits(),
            3
        );
    }

    #[test]
    fn typed_catalog_counts_match_the_raw_sdk_catalog() {
        assert_eq!(channels::common::COUNT, sys::channels::common::COUNT);
        assert_eq!(channels::truck::COUNT, sys::channels::truck::COUNT);
        assert_eq!(channels::trailer::COUNT, sys::channels::trailer::COUNT);
        assert_eq!(channels::job::COUNT, sys::channels::job::COUNT);
        assert_eq!(channels::COUNT, sys::channels::COUNT);
        assert_eq!(channels::COUNT, 107);
    }

    #[test]
    fn every_typed_channel_matches_the_raw_header_catalog() {
        assert_catalog_matches_raw(&channels::common::ALL, &sys::channels::common::ALL);
        assert_catalog_matches_raw(&channels::truck::ALL, &sys::channels::truck::ALL);
        assert_catalog_matches_raw(&channels::trailer::ALL, &sys::channels::trailer::ALL);
        assert_catalog_matches_raw(&channels::job::ALL, &sys::channels::job::ALL);

        let groups = [
            channels::common::ALL.as_slice(),
            channels::truck::ALL.as_slice(),
            channels::trailer::ALL.as_slice(),
            channels::job::ALL.as_slice(),
        ];
        let mut offset = 0;
        for group in groups {
            let end = offset + group.len();
            assert_eq!(&channels::ALL[offset..end], group);
            offset = end;
        }
        assert_eq!(offset, channels::COUNT);

        for (position, descriptor) in channels::ALL.iter().enumerate() {
            assert!(
                channels::ALL[..position]
                    .iter()
                    .all(|earlier| earlier.name() != descriptor.name()),
                "duplicate channel descriptor: {:?}",
                descriptor.name()
            );
        }
        assert_eq!(
            channels::ALL
                .iter()
                .filter(|channel| channel.is_indexed())
                .count(),
            17
        );
    }

    #[test]
    fn descriptors_preserve_name_type_and_index_mode() {
        assert_eq!(channels::common::GAME_TIME.name(), c"game.time");
        assert_eq!(
            channels::truck::WORLD_PLACEMENT.erase(),
            AnyChannel {
                name: c"truck.world.placement",
                value_type: ValueType::DPlacement,
                indexed: false,
                availability: GameSchemaAvailability::new(
                    Some(crate::game::ets2::V1_00),
                    Some(crate::game::ats::V1_00),
                ),
            }
        );
        assert_eq!(
            channels::truck::WHEEL_ROTATION.erase(),
            AnyChannel {
                name: c"truck.wheel.rotation",
                value_type: ValueType::F32,
                indexed: true,
                availability: GameSchemaAvailability::new(
                    Some(crate::game::ets2::V1_00),
                    Some(crate::game::ats::V1_00),
                ),
            }
        );
        assert_eq!(
            channels::trailer::WHEEL_ON_GROUND.erase(),
            AnyChannel {
                name: c"trailer.wheel.on_ground",
                value_type: ValueType::Bool,
                indexed: true,
                availability: GameSchemaAvailability::new(
                    Some(crate::game::ets2::V1_00),
                    Some(crate::game::ats::V1_00),
                ),
            }
        );
        assert_eq!(channels::job::CARGO_DAMAGE.name(), c"job.cargo.damage");
    }

    #[test]
    fn channel_availability_follows_early_game_schema_history() {
        use channels::{common, truck};

        // Early ETS2 additions are preserved even though the oldest archived
        // downloadable bundle (SDK 1.0) already reports schema 1.05.
        assert_since(
            truck::BRAKE_AIR_PRESSURE_EMERGENCY,
            Some(crate::game::ets2::V1_01),
            Some(crate::game::ats::V1_00),
        );
        assert_since(
            truck::CABIN_OFFSET,
            Some(crate::game::ets2::V1_02),
            Some(crate::game::ats::V1_00),
        );
        for channel in [truck::LIGHT_LBLINKER, truck::LIGHT_RBLINKER] {
            assert_since(
                channel,
                Some(crate::game::ets2::V1_04),
                Some(crate::game::ats::V1_00),
            );
        }

        for channel in [
            common::GAME_TIME.requesting::<i32>(),
            common::NEXT_REST_STOP,
        ] {
            assert_since(
                channel,
                Some(crate::game::ets2::V1_09),
                Some(crate::game::ats::V1_00),
            );
        }
        for channel in [truck::WHEEL_LIFT, truck::WHEEL_LIFT_OFFSET] {
            assert_since(
                channel,
                Some(crate::game::ets2::V1_10),
                Some(crate::game::ats::V1_00),
            );
        }
        assert_since(
            truck::DISPLAYED_GEAR,
            Some(crate::game::ets2::V1_11),
            Some(crate::game::ats::V1_00),
        );
        for channel in [
            truck::FUEL_RANGE,
            truck::NAVIGATION_DISTANCE,
            truck::NAVIGATION_TIME,
            truck::NAVIGATION_SPEED_LIMIT,
        ] {
            assert_since(
                channel,
                Some(crate::game::ets2::V1_12),
                Some(crate::game::ats::V1_00),
            );
        }
        for channel in [
            truck::ADBLUE,
            truck::ADBLUE_WARNING.requesting::<f32>(),
            truck::ADBLUE_AVERAGE_CONSUMPTION,
        ] {
            assert_since(channel, Some(crate::game::ets2::V1_12), None);
        }
    }

    #[test]
    fn channel_availability_follows_later_game_schema_history() {
        use channels::{common, job, trailer, truck};

        for channel in [
            trailer::CARGO_DAMAGE,
            trailer::WEAR_WHEELS,
            trailer::WHEEL_LIFT,
            trailer::WHEEL_LIFT_OFFSET,
            job::CARGO_DAMAGE,
        ] {
            assert_since(
                channel,
                Some(crate::game::ets2::V1_14),
                Some(crate::game::ats::V1_01),
            );
        }
        for channel in [
            truck::HAZARD_WARNING,
            truck::DIFFERENTIAL_LOCK,
            truck::LIFT_AXLE,
            truck::LIFT_AXLE_INDICATOR,
            truck::TRAILER_LIFT_AXLE,
            truck::TRAILER_LIFT_AXLE_INDICATOR,
        ] {
            assert_since(
                channel,
                Some(crate::game::ets2::V1_17),
                Some(crate::game::ats::V1_04),
            );
        }
        assert_since(
            common::MULTIPLAYER_TIME_OFFSET,
            Some(crate::game::ets2::V1_18),
            Some(crate::game::ats::V1_05),
        );
        assert_since(
            trailer::WEAR_BODY,
            Some(crate::game::ets2::V1_18),
            Some(crate::game::ats::V1_05),
        );

        assert_eq!(
            channels::MULTI_TRAILER_AVAILABILITY,
            GameSchemaAvailability::new(
                Some(crate::game::ets2::V1_14),
                Some(crate::game::ats::V1_01),
            )
        );
        assert!(
            channels::ALL
                .iter()
                .all(|channel| channel.availability().available_since_ets2().is_some())
        );
        assert_eq!(
            channels::ALL
                .iter()
                .filter(|channel| channel.availability().available_since_ats().is_none())
                .count(),
            3,
            "only the three AdBlue channels are explicitly unsupported by ATS",
        );
    }
}
