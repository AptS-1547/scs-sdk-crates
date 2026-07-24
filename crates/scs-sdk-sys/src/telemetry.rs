use core::ffi::c_void;
use core::mem::{offset_of, size_of};

use crate::{
    ScsContext, ScsEvent, ScsLogType, ScsNamedValue, ScsResult, ScsString, ScsTimestamp, ScsU32,
    ScsValue, ScsValueType,
};

pub type ScsLog = unsafe extern "system" fn(log_type: ScsLogType, message: ScsString);

pub type ScsTelemetryEventCallback =
    unsafe extern "system" fn(event: ScsEvent, event_info: *const c_void, context: ScsContext);

pub type ScsTelemetryChannelCallback = unsafe extern "system" fn(
    name: ScsString,
    index: ScsU32,
    value: *const ScsValue,
    context: ScsContext,
);

pub type ScsTelemetryRegisterForEvent = unsafe extern "system" fn(
    event: ScsEvent,
    callback: ScsTelemetryEventCallback,
    context: ScsContext,
) -> ScsResult;

pub type ScsTelemetryUnregisterFromEvent = unsafe extern "system" fn(event: ScsEvent) -> ScsResult;

pub type ScsTelemetryRegisterForChannel = unsafe extern "system" fn(
    name: ScsString,
    index: ScsU32,
    type_: ScsValueType,
    flags: ScsU32,
    callback: ScsTelemetryChannelCallback,
    context: ScsContext,
) -> ScsResult;

pub type ScsTelemetryUnregisterFromChannel =
    unsafe extern "system" fn(name: ScsString, index: ScsU32, type_: ScsValueType) -> ScsResult;

#[repr(C)]
pub struct ScsSdkInitParamsV100 {
    pub game_name: ScsString,
    pub game_id: ScsString,
    pub game_version: ScsU32,
    pub padding: crate::ScsPadding,
    pub log: ScsLog,
}

#[repr(C)]
pub struct ScsTelemetryInitParams {
    _private: [u8; 0],
}

// The C++ base class is empty and uses the empty-base optimization, so the common
// fields begin at offset zero in the concrete v1.00/v1.01 parameter structure.
#[repr(C)]
pub struct ScsTelemetryInitParamsV100 {
    pub common: ScsSdkInitParamsV100,
    pub register_for_event: ScsTelemetryRegisterForEvent,
    pub unregister_from_event: ScsTelemetryUnregisterFromEvent,
    pub register_for_channel: ScsTelemetryRegisterForChannel,
    pub unregister_from_channel: ScsTelemetryUnregisterFromChannel,
}

pub type ScsTelemetryInitParamsV101 = ScsTelemetryInitParamsV100;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ScsTelemetryFrameStart {
    pub flags: ScsU32,
    pub padding: crate::ScsPadding,
    pub render_time: ScsTimestamp,
    pub simulation_time: ScsTimestamp,
    pub paused_simulation_time: ScsTimestamp,
}

#[repr(C)]
pub struct ScsTelemetryConfiguration {
    pub id: ScsString,
    pub attributes: *const ScsNamedValue,
}

#[repr(C)]
pub struct ScsTelemetryGameplayEvent {
    pub id: ScsString,
    pub attributes: *const ScsNamedValue,
}

const _: [(); 32] = [(); size_of::<ScsSdkInitParamsV100>()];
const _: [(); 64] = [(); size_of::<ScsTelemetryInitParamsV100>()];
const _: [(); 32] = [(); size_of::<ScsTelemetryFrameStart>()];
const _: [(); 16] = [(); size_of::<ScsTelemetryConfiguration>()];
const _: [(); 16] = [(); size_of::<ScsTelemetryGameplayEvent>()];

const _: [(); 0] = [(); offset_of!(ScsSdkInitParamsV100, game_name)];
const _: [(); 8] = [(); offset_of!(ScsSdkInitParamsV100, game_id)];
const _: [(); 16] = [(); offset_of!(ScsSdkInitParamsV100, game_version)];
const _: [(); 24] = [(); offset_of!(ScsSdkInitParamsV100, log)];
const _: [(); 0] = [(); offset_of!(ScsTelemetryInitParamsV100, common)];
const _: [(); 32] = [(); offset_of!(ScsTelemetryInitParamsV100, register_for_event)];
const _: [(); 40] = [(); offset_of!(ScsTelemetryInitParamsV100, unregister_from_event)];
const _: [(); 48] = [(); offset_of!(ScsTelemetryInitParamsV100, register_for_channel)];
const _: [(); 56] = [(); offset_of!(ScsTelemetryInitParamsV100, unregister_from_channel)];
