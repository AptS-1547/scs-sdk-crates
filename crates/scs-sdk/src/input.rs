//! Safe typed interpretation of the SCS Input SDK 1.00 interface.
//!
//! Input devices are registered only during `scs_input_init`. Once active, SCS
//! repeatedly calls each device's event callback on the game main thread until
//! the callback reports `NotFound` for the current frame. This module keeps the
//! registration capability scoped to initialization and represents the only
//! supported event value types—bool and float—without exposing the raw union.

use core::ffi::{CStr, c_void};
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use crate::{InputApiVersion, InputGameVersion, ScopedLogger, SdkError, SdkResult, sys};

/// Input-device class declared by the official SDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum InputDeviceType {
    /// Generic device whose inputs can be bound in the game UI.
    Generic = sys::SCS_INPUT_DEVICE_TYPE_GENERIC,
    /// Device whose input names map directly to supported game mixes.
    Semantical = sys::SCS_INPUT_DEVICE_TYPE_SEMANTICAL,
}

impl InputDeviceType {
    #[must_use]
    pub const fn raw(self) -> sys::ScsInputDeviceType {
        self as sys::ScsInputDeviceType
    }
}

/// Value representation accepted for an input-device entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputValueType {
    Bool,
    /// Normalized finite axis represented by [`InputAxisValue`].
    Float,
}

impl InputValueType {
    #[must_use]
    pub const fn raw(self) -> sys::ScsValueType {
        match self {
            Self::Bool => sys::SCS_VALUE_TYPE_BOOL,
            Self::Float => sys::SCS_VALUE_TYPE_FLOAT,
        }
    }
}

/// A finite normalized value for one float input axis.
///
/// The raw Input API describes this representation only as a `float`. The game
/// consumes bindable axes in the inclusive `-1.0..=1.0` interval, where zero is
/// the center and the endpoints are the two maximum directions. Real ETS2
/// validation showed that a value below `-1.0` still crosses the ABI boundary
/// successfully but is interpreted by the input UI as the neutral center.
/// Keeping the normalized domain in this type prevents safe plugins from
/// emitting those semantically invalid values.
///
/// Construction rejects NaN and infinities as well as finite out-of-range
/// values. Values are never silently clamped because that would hide a device
/// conversion error and would not match the observed game behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputAxisValue(f32);

impl InputAxisValue {
    /// Maximum movement in the negative direction.
    pub const MIN: Self = Self(-1.0);
    /// Neutral centered position.
    pub const CENTER: Self = Self(0.0);
    /// Maximum movement in the positive direction.
    pub const MAX: Self = Self(1.0);

    /// Validates one normalized axis value.
    ///
    /// Both positive and negative zero are accepted and preserved exactly.
    ///
    /// # Errors
    ///
    /// Returns [`InputAxisValueError::NotFinite`] for NaN or either infinity,
    /// and [`InputAxisValueError::OutOfRange`] for a finite value outside the
    /// inclusive `-1.0..=1.0` interval.
    pub fn new(value: f32) -> Result<Self, InputAxisValueError> {
        if !value.is_finite() {
            return Err(InputAxisValueError::NotFinite);
        }
        if !(-1.0..=1.0).contains(&value) {
            return Err(InputAxisValueError::OutOfRange);
        }
        Ok(Self(value))
    }

    /// Returns the validated SDK float representation.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for InputAxisValue {
    type Error = InputAxisValueError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<InputAxisValue> for f32 {
    fn from(value: InputAxisValue) -> Self {
        value.get()
    }
}

/// Why a raw float could not become a normalized [`InputAxisValue`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAxisValueError {
    /// The value was NaN, positive infinity, or negative infinity.
    NotFinite,
    /// The finite value was below `-1.0` or above `1.0`.
    OutOfRange,
}

impl fmt::Display for InputAxisValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("input axis value is not finite"),
            Self::OutOfRange => {
                formatter.write_str("input axis value is outside the inclusive -1.0 to 1.0 range")
            }
        }
    }
}

/// Strong zero-based input index limited by the official per-device maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputIndex(u32);

impl InputIndex {
    pub const MAX_COUNT: u32 = sys::SCS_INPUT_MAX_INPUT_COUNT;

    #[must_use]
    pub const fn new(raw: u32) -> Option<Self> {
        if raw < Self::MAX_COUNT {
            Some(Self(raw))
        } else {
            None
        }
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Flags supplied when SCS requests the next event from a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputEventFlags(u32);

impl InputEventFlags {
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn first_in_frame(self) -> bool {
        self.0 & sys::SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_IN_FRAME != 0
    }

    #[must_use]
    pub const fn first_after_activation(self) -> bool {
        self.0 & sys::SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_AFTER_ACTIVATION != 0
    }
}

/// Safe event value returned by an application input provider.
///
/// Float events require a validated [`InputAxisValue`]; an arbitrary raw float
/// cannot cross the safe application boundary.
///
/// ```compile_fail
/// use scs_sdk::input::InputValue;
///
/// let _invalid = InputValue::Float(-2.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputValue {
    Bool(bool),
    /// One normalized float axis. Arbitrary `f32` values must first pass
    /// [`InputAxisValue::new`], so every safe event is finite and in range.
    Float(InputAxisValue),
}

impl InputValue {
    #[must_use]
    pub const fn value_type(self) -> InputValueType {
        match self {
            Self::Bool(_) => InputValueType::Bool,
            Self::Float(_) => InputValueType::Float,
        }
    }
}

/// One safe input event to write into the buffer supplied by SCS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputEvent {
    index: InputIndex,
    value: InputValue,
}

impl InputEvent {
    #[must_use]
    pub const fn new(index: InputIndex, value: InputValue) -> Self {
        Self { index, value }
    }

    #[must_use]
    pub const fn index(self) -> InputIndex {
        self.index
    }

    #[must_use]
    pub const fn value(self) -> InputValue {
        self.value
    }

    /// Writes this event into a live SDK output buffer after type validation.
    ///
    /// The official callback contract gives the plugin a reusable output
    /// buffer and requires it to populate the index plus the union member that
    /// matches the registered input type. This method deliberately leaves the
    /// inactive union tail untouched instead of replacing the entire raw
    /// structure. Besides matching the official sample, that avoids copying
    /// bytes for inactive or future-extension storage whose contents have no
    /// meaning for the current value.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::InvalidParameter`] when `output` is null or when the
    /// event value does not match the value type registered for that input.
    ///
    /// # Safety
    ///
    /// `output` must identify the live `scs_input_event_t` supplied to the
    /// current input callback. The callback must be executing directly on the
    /// SCS game main thread.
    pub unsafe fn write_to(
        self,
        output: *mut sys::ScsInputEvent,
        expected_type: InputValueType,
    ) -> SdkResult {
        if output.is_null() || self.value.value_type() != expected_type {
            return Err(SdkError::InvalidParameter);
        }

        // SAFETY: The null and registered-type checks above succeeded, and the
        // caller guarantees that `output` points to the live writable event
        // buffer for this direct main-thread callback. `addr_of_mut!` performs
        // raw field projection without creating a reference to the possibly
        // partially initialized structure or union. We initialize the index
        // and exactly the active member selected by the validated Rust value;
        // no inactive member or future-extension byte is read or written.
        unsafe {
            core::ptr::addr_of_mut!((*output).input_index).write(self.index.raw());
            match self.value {
                InputValue::Bool(value) => {
                    core::ptr::addr_of_mut!((*output).value.value_bool).write(sys::ScsValueBool {
                        value: u8::from(value),
                    });
                }
                InputValue::Float(value) => {
                    core::ptr::addr_of_mut!((*output).value.value_float)
                        .write(sys::ScsValueFloat { value: value.get() });
                }
            }
        }
        Ok(())
    }
}

/// Header-shaped description of one input, with C-string lifetimes retained.
#[repr(transparent)]
pub struct InputDeviceInput<'a> {
    raw: sys::ScsInputDeviceInput,
    lifetime: PhantomData<(&'a CStr, &'a CStr)>,
}

impl<'a> InputDeviceInput<'a> {
    #[must_use]
    pub const fn new(name: &'a CStr, display_name: &'a CStr, value_type: InputValueType) -> Self {
        Self {
            raw: sys::ScsInputDeviceInput {
                name: name.as_ptr(),
                display_name: display_name.as_ptr(),
                value_type: value_type.raw(),
                padding: MaybeUninit::uninit(),
            },
            lifetime: PhantomData,
        }
    }

    #[must_use]
    pub const fn value_type(&self) -> Option<InputValueType> {
        match self.raw.value_type {
            sys::SCS_VALUE_TYPE_BOOL => Some(InputValueType::Bool),
            sys::SCS_VALUE_TYPE_FLOAT => Some(InputValueType::Float),
            _ => None,
        }
    }
}

/// Fully described raw registration whose callback invariants were established
/// by an audited upper layer.
pub struct InputDeviceRegistration<'a> {
    raw: sys::ScsInputDevice,
    lifetime: PhantomData<&'a [InputDeviceInput<'a>]>,
}

impl<'a> InputDeviceRegistration<'a> {
    /// Creates a device registration for one initialization-scoped SDK call.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::InvalidParameter`] when the input slice is empty,
    /// exceeds [`InputIndex::MAX_COUNT`], or cannot be represented by the raw
    /// SDK count field.
    ///
    /// # Safety
    ///
    /// `callback_context` must remain valid for every callback until SCS has
    /// automatically unregistered the device before `scs_input_shutdown`.
    /// Both callbacks must contain panics, obey the game-main-thread contract,
    /// and validate every foreign pointer before dereferencing it.
    pub unsafe fn new(
        name: &'a CStr,
        display_name: &'a CStr,
        device_type: InputDeviceType,
        inputs: &'a [InputDeviceInput<'a>],
        callback_context: *mut c_void,
        active_callback: Option<sys::ScsInputActiveCallback>,
        event_callback: sys::ScsInputEventCallback,
    ) -> SdkResult<Self> {
        if inputs.is_empty() || inputs.len() > sys::SCS_INPUT_MAX_INPUT_COUNT as usize {
            return Err(SdkError::InvalidParameter);
        }
        let input_count = u32::try_from(inputs.len()).map_err(|_| SdkError::InvalidParameter)?;
        Ok(Self {
            raw: sys::ScsInputDevice {
                name: name.as_ptr(),
                display_name: display_name.as_ptr(),
                type_: device_type.raw(),
                input_count,
                inputs: inputs.as_ptr().cast::<sys::ScsInputDeviceInput>(),
                callback_context,
                input_active_callback: active_callback,
                input_event_callback: event_callback,
            },
            lifetime: PhantomData,
        })
    }
}

#[derive(Clone, Copy)]
struct InputSessionTable {
    version: InputApiVersion,
    logger: sys::ScsLog,
}

/// Typed view over input initialization parameters supplied by SCS.
pub struct InputApi<'a> {
    raw: &'a sys::ScsInputInitParamsV100,
    version: InputApiVersion,
    not_send_sync: PhantomData<*mut ()>,
}

/// Inert handle retained for direct input callbacks and shutdown logging.
#[derive(Clone, Copy)]
pub struct InputSession {
    table: InputSessionTable,
}

/// Capabilities valid during one direct input callback or shutdown call.
///
/// The raw SDK permits calls back into the game only while SCS is directly
/// invoking plugin code on the game main thread. This token therefore carries
/// an invariant lifetime and a raw-pointer marker so it cannot be stored, sent
/// to another thread, or used as a global capability.
///
/// ```compile_fail
/// use scs_sdk::input::InputCall;
///
/// fn require_send<T: Send>() {}
///
/// require_send::<InputCall<'static>>();
/// ```
pub struct InputCall<'scope> {
    table: InputSessionTable,
    scope: PhantomData<&'scope mut ()>,
    not_send_sync: PhantomData<*mut ()>,
}

/// Initialization-only input capability which can register devices.
pub struct InputInitCall<'scope> {
    call: InputCall<'scope>,
    register_device: sys::ScsInputRegisterDevice,
}

impl<'a> InputApi<'a> {
    pub const SUPPORTED_VERSIONS: &'static [InputApiVersion] = &[InputApiVersion::V1_00];

    #[must_use]
    pub const fn supports_version(version: InputApiVersion) -> bool {
        version.raw() == InputApiVersion::V1_00.raw()
    }

    /// Creates a typed view over the input initialization parameters.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::Unsupported`] for any input API version other than
    /// the audited 1.00 layout, or [`SdkError::InvalidParameter`] when `params`
    /// is null.
    ///
    /// # Safety
    ///
    /// For input API 1.00, `params` must point to the live matching structure
    /// supplied by SCS for the duration of this main-thread initialization.
    pub unsafe fn from_raw(
        version: InputApiVersion,
        params: *const sys::ScsInputInitParams,
    ) -> SdkResult<Self> {
        if !Self::supports_version(version) {
            return Err(SdkError::Unsupported);
        }
        // SAFETY: The caller guarantees the matching v1.00 layout after the
        // exact supported-version check above.
        let raw = unsafe { params.cast::<sys::ScsInputInitParamsV100>().as_ref() }
            .ok_or(SdkError::InvalidParameter)?;
        Ok(Self {
            raw,
            version,
            not_send_sync: PhantomData,
        })
    }

    #[must_use]
    pub const fn version(&self) -> InputApiVersion {
        self.version
    }

    #[must_use]
    pub fn game_name(&self) -> &'a CStr {
        // SAFETY: SCS documents this pointer as non-null and NUL-terminated for
        // the complete initialization call.
        unsafe { CStr::from_ptr(self.raw.common.game_name) }
    }

    #[must_use]
    pub fn game_id(&self) -> &'a CStr {
        // SAFETY: SCS documents this pointer as non-null and NUL-terminated for
        // the complete initialization call.
        unsafe { CStr::from_ptr(self.raw.common.game_id) }
    }

    #[must_use]
    pub const fn game_version(&self) -> InputGameVersion {
        InputGameVersion::from_raw(self.raw.common.game_version)
    }

    #[must_use]
    pub const fn session(&self) -> InputSession {
        InputSession {
            table: InputSessionTable {
                version: self.version,
                logger: self.raw.common.log,
            },
        }
    }

    pub fn with_init_call<R>(
        &self,
        operation: impl for<'scope> FnOnce(&InputInitCall<'scope>) -> R,
    ) -> R {
        let call = InputInitCall {
            call: InputCall {
                table: self.session().table,
                scope: PhantomData,
                not_send_sync: PhantomData,
            },
            register_device: self.raw.register_device,
        };
        operation(&call)
    }
}

impl InputSession {
    /// Creates a callback-scoped capability during a direct call from SCS.
    ///
    /// The higher-ranked callback keeps the created [`InputCall`] inside the
    /// direct SDK callback scope:
    ///
    /// ```compile_fail
    /// use scs_sdk::input::{InputCall, InputSession};
    ///
    /// fn leak(session: InputSession) -> &'static InputCall<'static> {
    ///     unsafe { session.with_call(|call| call) }
    /// }
    /// ```
    ///
    /// # Safety
    ///
    /// The caller must be executing synchronously in an input callback or
    /// shutdown entry point on the game main thread while this session is live.
    pub unsafe fn with_call<R>(
        self,
        operation: impl for<'scope> FnOnce(&InputCall<'scope>) -> R,
    ) -> R {
        let call = InputCall {
            table: self.table,
            scope: PhantomData,
            not_send_sync: PhantomData,
        };
        operation(&call)
    }
}

impl InputCall<'_> {
    #[must_use]
    pub const fn input_api_version(&self) -> InputApiVersion {
        self.table.version
    }

    #[must_use]
    pub const fn logger(&self) -> ScopedLogger<'_> {
        ScopedLogger::from_raw(self.table.logger)
    }
}

impl InputInitCall<'_> {
    #[must_use]
    pub const fn input_api_version(&self) -> InputApiVersion {
        self.call.input_api_version()
    }

    #[must_use]
    pub const fn logger(&self) -> ScopedLogger<'_> {
        self.call.logger()
    }

    /// Registers one input device during `scs_input_init`.
    ///
    /// # Errors
    ///
    /// Returns the SDK error reported by SCS when the device name, input list,
    /// callbacks, or current lifecycle state are rejected by the game.
    pub fn register_device(&self, device: &InputDeviceRegistration<'_>) -> SdkResult {
        // SAFETY: `InputDeviceRegistration::new` established the callback and
        // context invariants. This type is constructible only during the
        // higher-ranked initialization scope, and SCS fully consumes the
        // descriptor arrays before returning from this call.
        let result = unsafe { (self.register_device)(&raw const device.raw) };
        SdkError::from_code(result)
    }
}

/// Input API game-version constants declared by the SDK 1.14 headers.
pub mod game {
    use crate::{InputGameVersion, sys};

    pub mod ets2 {
        use super::{InputGameVersion, sys};

        pub const V1_00: InputGameVersion =
            InputGameVersion::from_raw(sys::SCS_INPUT_EUT2_GAME_VERSION_1_00);
        pub const CURRENT: InputGameVersion = V1_00;
    }

    pub mod ats {
        use super::{InputGameVersion, sys};

        pub const V1_00: InputGameVersion =
            InputGameVersion::from_raw(sys::SCS_INPUT_ATS_GAME_VERSION_1_00);
        pub const CURRENT: InputGameVersion = V1_00;
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::ffi::c_void;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::vec::Vec;

    use super::*;

    static REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "system" fn fake_log(_level: sys::ScsLogType, _message: sys::ScsString) {}

    unsafe extern "system" fn fake_event(
        _event: *mut sys::ScsInputEvent,
        _flags: u32,
        _context: *mut c_void,
    ) -> sys::ScsResult {
        sys::SCS_RESULT_NOT_FOUND
    }

    unsafe extern "system" fn fake_register(_device: *const sys::ScsInputDevice) -> sys::ScsResult {
        REGISTRATIONS.fetch_add(1, Ordering::Relaxed);
        sys::SCS_RESULT_OK
    }

    fn raw_api() -> sys::ScsInputInitParamsV100 {
        sys::ScsInputInitParamsV100 {
            common: sys::ScsSdkInitParamsV100 {
                game_name: c"Game".as_ptr(),
                game_id: c"eut2".as_ptr(),
                game_version: sys::SCS_INPUT_EUT2_GAME_VERSION_1_00,
                padding: MaybeUninit::uninit(),
                log: fake_log,
            },
            register_device: fake_register,
        }
    }

    #[test]
    fn input_api_accepts_only_the_audited_v100_layout() {
        let raw = raw_api();
        let unsupported =
            unsafe { InputApi::from_raw(InputApiVersion::new(1, 1), (&raw const raw).cast()) };
        assert_eq!(unsupported.err(), Some(SdkError::Unsupported));

        let api = unsafe { InputApi::from_raw(InputApiVersion::V1_00, (&raw const raw).cast()) }
            .expect("v1.00 should be supported");
        assert_eq!(api.game_version(), game::ets2::V1_00);
    }

    #[test]
    fn registration_and_event_writing_preserve_types() {
        REGISTRATIONS.store(0, Ordering::Relaxed);
        let raw = raw_api();
        let api = unsafe { InputApi::from_raw(InputApiVersion::V1_00, (&raw const raw).cast()) }
            .expect("v1.00 should be supported");
        let inputs = [InputDeviceInput::new(
            c"button",
            c"Button",
            InputValueType::Bool,
        )];
        let device = unsafe {
            InputDeviceRegistration::new(
                c"device",
                c"Device",
                InputDeviceType::Generic,
                &inputs,
                core::ptr::null_mut(),
                None,
                fake_event,
            )
        }
        .expect("device should be valid");
        api.with_init_call(|call| call.register_device(&device))
            .expect("registration should succeed");
        assert_eq!(REGISTRATIONS.load(Ordering::Relaxed), 1);

        let mut output = MaybeUninit::<sys::ScsInputEvent>::zeroed();
        let event = InputEvent::new(
            InputIndex::new(0).expect("zero is valid"),
            InputValue::Bool(true),
        );
        unsafe { event.write_to(output.as_mut_ptr(), InputValueType::Bool) }
            .expect("matching value type should write");
        // SAFETY: The buffer was fully initialized with zeroes before
        // `write_to`, which then initialized the index and active bool member.
        let output = unsafe { output.assume_init() };
        assert_eq!(output.input_index, 0);
        // SAFETY: This event was registered and written as a bool above.
        let value = unsafe { output.value.value_bool.value };
        assert_eq!(value, 1);
    }

    #[test]
    fn event_writing_rejects_null_and_value_type_mismatch() {
        let index = InputIndex::new(0).expect("zero is valid");
        let bool_event = InputEvent::new(index, InputValue::Bool(true));
        let mut output = MaybeUninit::<sys::ScsInputEvent>::uninit();

        let null_result =
            unsafe { bool_event.write_to(core::ptr::null_mut(), InputValueType::Bool) };
        assert_eq!(null_result, Err(SdkError::InvalidParameter));

        let mismatch = unsafe { bool_event.write_to(output.as_mut_ptr(), InputValueType::Float) };
        assert_eq!(mismatch, Err(SdkError::InvalidParameter));
    }

    #[test]
    fn event_writing_initializes_only_the_active_union_storage() {
        const SENTINEL: u8 = 0xA5;

        let mut float_output = MaybeUninit::<sys::ScsInputEvent>::uninit();
        // SAFETY: `float_output` provides aligned storage for exactly one raw
        // event, and writing bytes is valid even before the typed value is
        // initialized. The resulting sentinel bit pattern is valid opaque
        // storage for the raw C union and lets this test observe which bytes
        // `write_to` changes without assuming the whole event afterward.
        unsafe {
            core::ptr::write_bytes(
                float_output.as_mut_ptr().cast::<u8>(),
                SENTINEL,
                core::mem::size_of::<sys::ScsInputEvent>(),
            );
        }
        let event = InputEvent::new(
            InputIndex::new(3).expect("three is valid"),
            InputValue::Float(InputAxisValue::new(-0.625).expect("value is normalized")),
        );

        unsafe { event.write_to(float_output.as_mut_ptr(), InputValueType::Float) }
            .expect("matching float value should write");

        // SAFETY: Every byte in the backing storage was initialized to the
        // sentinel before the field writes. Reading those initialized bytes as
        // a byte slice neither selects nor reads a typed union member.
        let float_bytes = unsafe {
            core::slice::from_raw_parts(
                float_output.as_ptr().cast::<u8>(),
                core::mem::size_of::<sys::ScsInputEvent>(),
            )
        };
        assert_eq!(&float_bytes[..4], &3_u32.to_ne_bytes());
        assert_eq!(&float_bytes[4..8], &(-0.625_f32).to_ne_bytes());
        assert!(float_bytes[8..].iter().all(|byte| *byte == SENTINEL));

        let mut bool_output = MaybeUninit::<sys::ScsInputEvent>::uninit();
        // SAFETY: The same aligned, in-bounds byte initialization argument as
        // for `float_output` applies to this independent event buffer.
        unsafe {
            core::ptr::write_bytes(
                bool_output.as_mut_ptr().cast::<u8>(),
                SENTINEL,
                core::mem::size_of::<sys::ScsInputEvent>(),
            );
        }
        let event = InputEvent::new(
            InputIndex::new(7).expect("seven is valid"),
            InputValue::Bool(true),
        );

        unsafe { event.write_to(bool_output.as_mut_ptr(), InputValueType::Bool) }
            .expect("matching bool value should write");

        // SAFETY: The full backing allocation was initialized to the sentinel
        // before `write_to` changed the index and one bool byte.
        let bool_bytes = unsafe {
            core::slice::from_raw_parts(
                bool_output.as_ptr().cast::<u8>(),
                core::mem::size_of::<sys::ScsInputEvent>(),
            )
        };
        assert_eq!(&bool_bytes[..4], &7_u32.to_ne_bytes());
        assert_eq!(bool_bytes[4], 1);
        assert!(bool_bytes[5..].iter().all(|byte| *byte == SENTINEL));
    }

    #[test]
    fn input_axis_value_accepts_only_the_finite_normalized_domain() {
        for value in [-1.0, -0.625, -0.0, 0.0, 0.625, 1.0] {
            let normalized = InputAxisValue::new(value).expect("value should be normalized");
            assert_eq!(normalized.get().to_bits(), value.to_bits());
            assert_eq!(f32::from(normalized).to_bits(), value.to_bits());
            assert_eq!(InputAxisValue::try_from(value), Ok(normalized));
        }

        for value in [-1.000_000_1, -2.0, 1.000_000_1, 2.0] {
            assert_eq!(
                InputAxisValue::new(value),
                Err(InputAxisValueError::OutOfRange)
            );
        }

        for value in [f32::NAN, f32::NEG_INFINITY, f32::INFINITY] {
            assert_eq!(
                InputAxisValue::new(value),
                Err(InputAxisValueError::NotFinite)
            );
        }

        assert_eq!(InputAxisValue::MIN.get().to_bits(), (-1.0_f32).to_bits());
        assert_eq!(InputAxisValue::CENTER.get().to_bits(), 0.0_f32.to_bits());
        assert_eq!(InputAxisValue::MAX.get().to_bits(), 1.0_f32.to_bits());
    }

    #[test]
    fn device_registration_rejects_empty_and_too_many_inputs() {
        let empty = unsafe {
            InputDeviceRegistration::new(
                c"device",
                c"Device",
                InputDeviceType::Generic,
                &[],
                core::ptr::null_mut(),
                None,
                fake_event,
            )
        };
        assert_eq!(empty.err(), Some(SdkError::InvalidParameter));

        let inputs: Vec<_> = (0..=InputIndex::MAX_COUNT)
            .map(|_| InputDeviceInput::new(c"button", c"Button", InputValueType::Bool))
            .collect();
        let too_many = unsafe {
            InputDeviceRegistration::new(
                c"device",
                c"Device",
                InputDeviceType::Generic,
                &inputs,
                core::ptr::null_mut(),
                None,
                fake_event,
            )
        };
        assert_eq!(too_many.err(), Some(SdkError::InvalidParameter));
    }

    #[test]
    fn input_event_flags_decode_known_bits_and_preserve_unknown_bits() {
        let flags = InputEventFlags::from_raw(
            sys::SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_IN_FRAME
                | sys::SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_AFTER_ACTIVATION
                | 0x8000_0000,
        );

        assert!(flags.first_in_frame());
        assert!(flags.first_after_activation());
        assert_eq!(
            flags.raw(),
            sys::SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_IN_FRAME
                | sys::SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_AFTER_ACTIVATION
                | 0x8000_0000
        );
    }

    #[test]
    fn input_device_value_type_does_not_treat_unknown_as_float() {
        let mut input = InputDeviceInput::new(c"axis", c"Axis", InputValueType::Float);
        assert_eq!(input.value_type(), Some(InputValueType::Float));

        input.raw.value_type = sys::SCS_VALUE_TYPE_INVALID;
        assert_eq!(input.value_type(), None);
    }
}
