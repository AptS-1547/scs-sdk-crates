use crate::{ScsLogType, ScsResult, ScsU32, ScsValueType};

#[must_use]
pub const fn make_version(major: ScsU32, minor: ScsU32) -> ScsU32 {
    (major << 16) | minor
}

#[must_use]
pub const fn version_major(version: ScsU32) -> ScsU32 {
    (version >> 16) & 0xffff
}

#[must_use]
pub const fn version_minor(version: ScsU32) -> ScsU32 {
    version & 0xffff
}

pub const SCS_U32_NIL: ScsU32 = ScsU32::MAX;

pub const SCS_RESULT_OK: ScsResult = 0;
pub const SCS_RESULT_UNSUPPORTED: ScsResult = -1;
pub const SCS_RESULT_INVALID_PARAMETER: ScsResult = -2;
pub const SCS_RESULT_ALREADY_REGISTERED: ScsResult = -3;
pub const SCS_RESULT_NOT_FOUND: ScsResult = -4;
pub const SCS_RESULT_UNSUPPORTED_TYPE: ScsResult = -5;
pub const SCS_RESULT_NOT_NOW: ScsResult = -6;
pub const SCS_RESULT_GENERIC_ERROR: ScsResult = -7;

pub const SCS_LOG_TYPE_MESSAGE: ScsLogType = 0;
pub const SCS_LOG_TYPE_WARNING: ScsLogType = 1;
pub const SCS_LOG_TYPE_ERROR: ScsLogType = 2;

pub const SCS_VALUE_TYPE_INVALID: ScsValueType = 0;
pub const SCS_VALUE_TYPE_BOOL: ScsValueType = 1;
pub const SCS_VALUE_TYPE_S32: ScsValueType = 2;
pub const SCS_VALUE_TYPE_U32: ScsValueType = 3;
pub const SCS_VALUE_TYPE_U64: ScsValueType = 4;
pub const SCS_VALUE_TYPE_FLOAT: ScsValueType = 5;
pub const SCS_VALUE_TYPE_DOUBLE: ScsValueType = 6;
pub const SCS_VALUE_TYPE_FVECTOR: ScsValueType = 7;
pub const SCS_VALUE_TYPE_DVECTOR: ScsValueType = 8;
pub const SCS_VALUE_TYPE_EULER: ScsValueType = 9;
pub const SCS_VALUE_TYPE_FPLACEMENT: ScsValueType = 10;
pub const SCS_VALUE_TYPE_DPLACEMENT: ScsValueType = 11;
pub const SCS_VALUE_TYPE_STRING: ScsValueType = 12;
pub const SCS_VALUE_TYPE_S64: ScsValueType = 13;
pub const SCS_VALUE_TYPE_LAST: ScsValueType = SCS_VALUE_TYPE_S64;

pub const SCS_TELEMETRY_VERSION_1_00: ScsU32 = make_version(1, 0);
pub const SCS_TELEMETRY_VERSION_1_01: ScsU32 = make_version(1, 1);
pub const SCS_TELEMETRY_VERSION_CURRENT: ScsU32 = SCS_TELEMETRY_VERSION_1_01;

pub const SCS_TELEMETRY_EVENT_INVALID: ScsU32 = 0;
pub const SCS_TELEMETRY_EVENT_FRAME_START: ScsU32 = 1;
pub const SCS_TELEMETRY_EVENT_FRAME_END: ScsU32 = 2;
pub const SCS_TELEMETRY_EVENT_PAUSED: ScsU32 = 3;
pub const SCS_TELEMETRY_EVENT_STARTED: ScsU32 = 4;
pub const SCS_TELEMETRY_EVENT_CONFIGURATION: ScsU32 = 5;
pub const SCS_TELEMETRY_EVENT_GAMEPLAY: ScsU32 = 6;

pub const SCS_TELEMETRY_FRAME_START_FLAG_TIMER_RESTART: ScsU32 = 0x0000_0001;

pub const SCS_TELEMETRY_CHANNEL_FLAG_NONE: ScsU32 = 0x0000_0000;
pub const SCS_TELEMETRY_CHANNEL_FLAG_EACH_FRAME: ScsU32 = 0x0000_0001;
pub const SCS_TELEMETRY_CHANNEL_FLAG_NO_VALUE: ScsU32 = 0x0000_0002;

pub const SCS_GAME_ID_EUT2: &[u8] = b"eut2\0";
pub const SCS_GAME_ID_ATS: &[u8] = b"ats\0";
