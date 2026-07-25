//! Raw ABI declarations for the SCS Input SDK 1.00 interface.
//!
//! These definitions mirror `scssdk_input.h`, `scssdk_input_device.h`,
//! `scssdk_input_event.h`, and the ETS2/ATS input-version headers from the
//! official SDK 1.14 distribution. The input API was introduced by SDK 1.14;
//! none of the official SDK 1.0 through 1.13 archives contain these headers.

use core::mem::{offset_of, size_of};

use crate::{
    ScsContext, ScsPadding, ScsResult, ScsSdkInitParamsV100, ScsString, ScsU8, ScsU32,
    ScsValueBool, ScsValueFloat, ScsValueType, make_version,
};

/// Initial and current input API version declared by SDK 1.14.
pub const SCS_INPUT_VERSION_1_00: ScsU32 = make_version(1, 0);
/// Latest input API version declared by SDK 1.14.
pub const SCS_INPUT_VERSION_CURRENT: ScsU32 = SCS_INPUT_VERSION_1_00;

/// Initial and current ETS2 game version for the input API.
pub const SCS_INPUT_EUT2_GAME_VERSION_1_00: ScsU32 = make_version(1, 0);
/// Latest ETS2 input game version declared by SDK 1.14.
pub const SCS_INPUT_EUT2_GAME_VERSION_CURRENT: ScsU32 = SCS_INPUT_EUT2_GAME_VERSION_1_00;

/// Initial and current ATS game version for the input API.
pub const SCS_INPUT_ATS_GAME_VERSION_1_00: ScsU32 = make_version(1, 0);
/// Latest ATS input game version declared by SDK 1.14.
pub const SCS_INPUT_ATS_GAME_VERSION_CURRENT: ScsU32 = SCS_INPUT_ATS_GAME_VERSION_1_00;

/// Raw discriminator used by `scs_input_device_t::type`.
pub type ScsInputDeviceType = ScsU32;

pub const SCS_INPUT_DEVICE_TYPE_INVALID: ScsInputDeviceType = 0;
pub const SCS_INPUT_DEVICE_TYPE_GENERIC: ScsInputDeviceType = 1;
pub const SCS_INPUT_DEVICE_TYPE_SEMANTICAL: ScsInputDeviceType = 2;

/// Maximum number of inputs accepted for one registered device.
pub const SCS_INPUT_MAX_INPUT_COUNT: ScsU32 = 400;

pub const SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_IN_FRAME: ScsU32 = 0x0000_0001;
pub const SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_AFTER_ACTIVATION: ScsU32 = 0x0000_0002;

/// Opaque common ancestor used by the loader-facing input init function.
#[repr(C)]
pub struct ScsInputInitParams {
    _private: [u8; 0],
}

/// Information about one bool or float input exposed by a device.
#[repr(C)]
pub struct ScsInputDeviceInput {
    pub name: ScsString,
    pub display_name: ScsString,
    pub value_type: ScsValueType,
    pub padding: ScsPadding,
}

/// Callback used by SCS to notify a device that it became active or inactive.
pub type ScsInputActiveCallback = unsafe extern "system" fn(active: ScsU8, context: ScsContext);

/// Storage written by the plugin when SCS asks for the next input event.
#[repr(C)]
#[derive(Clone, Copy)]
pub union ScsInputEventValue {
    pub value_bool: ScsValueBool,
    pub value_float: ScsValueFloat,
    pub sizing_for_future_extensions: [f32; 6],
}

/// One event returned by an input device callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ScsInputEvent {
    pub input_index: ScsU32,
    pub value: ScsInputEventValue,
}

/// Callback repeatedly invoked by SCS until it returns `SCS_RESULT_NOT_FOUND`.
pub type ScsInputEventCallback = unsafe extern "system" fn(
    event_info: *mut ScsInputEvent,
    flags: ScsU32,
    context: ScsContext,
) -> ScsResult;

/// Raw input-device registration structure.
#[repr(C)]
pub struct ScsInputDevice {
    pub name: ScsString,
    pub display_name: ScsString,
    pub type_: ScsInputDeviceType,
    pub input_count: ScsU32,
    pub inputs: *const ScsInputDeviceInput,
    pub callback_context: ScsContext,
    pub input_active_callback: Option<ScsInputActiveCallback>,
    pub input_event_callback: ScsInputEventCallback,
}

/// Function supplied by SCS for registering one input device during init.
pub type ScsInputRegisterDevice =
    unsafe extern "system" fn(device_info: *const ScsInputDevice) -> ScsResult;

/// Initialization parameters for input API 1.00.
#[repr(C)]
pub struct ScsInputInitParamsV100 {
    pub common: ScsSdkInitParamsV100,
    pub register_device: ScsInputRegisterDevice,
}

const _: [(); 24] = [(); size_of::<ScsInputDeviceInput>()];
const _: [(); 24] = [(); size_of::<ScsInputEventValue>()];
const _: [(); 28] = [(); size_of::<ScsInputEvent>()];
const _: [(); 56] = [(); size_of::<ScsInputDevice>()];
const _: [(); 40] = [(); size_of::<ScsInputInitParamsV100>()];

const _: [(); 0] = [(); offset_of!(ScsInputDeviceInput, name)];
const _: [(); 8] = [(); offset_of!(ScsInputDeviceInput, display_name)];
const _: [(); 16] = [(); offset_of!(ScsInputDeviceInput, value_type)];
const _: [(); 20] = [(); offset_of!(ScsInputDeviceInput, padding)];
const _: [(); 4] = [(); offset_of!(ScsInputEvent, value)];
const _: [(); 0] = [(); offset_of!(ScsInputDevice, name)];
const _: [(); 8] = [(); offset_of!(ScsInputDevice, display_name)];
const _: [(); 16] = [(); offset_of!(ScsInputDevice, type_)];
const _: [(); 20] = [(); offset_of!(ScsInputDevice, input_count)];
const _: [(); 24] = [(); offset_of!(ScsInputDevice, inputs)];
const _: [(); 32] = [(); offset_of!(ScsInputDevice, callback_context)];
const _: [(); 40] = [(); offset_of!(ScsInputDevice, input_active_callback)];
const _: [(); 48] = [(); offset_of!(ScsInputDevice, input_event_callback)];
const _: [(); 0] = [(); offset_of!(ScsInputInitParamsV100, common)];
const _: [(); 32] = [(); offset_of!(ScsInputInitParamsV100, register_device)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_constants_match_the_sdk_1_14_headers() {
        assert_eq!(SCS_INPUT_VERSION_CURRENT, make_version(1, 0));
        assert_eq!(SCS_INPUT_EUT2_GAME_VERSION_CURRENT, make_version(1, 0));
        assert_eq!(SCS_INPUT_ATS_GAME_VERSION_CURRENT, make_version(1, 0));
        assert_eq!(SCS_INPUT_MAX_INPUT_COUNT, 400);
        assert_eq!(SCS_INPUT_DEVICE_TYPE_GENERIC, 1);
        assert_eq!(SCS_INPUT_DEVICE_TYPE_SEMANTICAL, 2);
    }
}
