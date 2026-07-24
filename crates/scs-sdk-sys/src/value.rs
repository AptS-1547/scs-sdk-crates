use core::ffi::{c_char, c_void};
use core::mem::{MaybeUninit, offset_of, size_of};

pub type ScsU8 = u8;
pub type ScsU16 = u16;
pub type ScsU32 = u32;
pub type ScsU64 = u64;
pub type ScsS32 = i32;
pub type ScsS64 = i64;
pub type ScsFloat = f32;
pub type ScsDouble = f64;
pub type ScsString = *const c_char;
pub type ScsContext = *mut c_void;
pub type ScsTimestamp = ScsU64;
pub type ScsResult = ScsS32;
pub type ScsLogType = ScsS32;
pub type ScsValueType = ScsU32;
pub type ScsEvent = ScsU32;
pub type ScsPadding = MaybeUninit<ScsU32>;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScsValueBool {
    pub value: ScsU8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScsValueS32 {
    pub value: ScsS32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScsValueU32 {
    pub value: ScsU32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScsValueU64 {
    pub value: ScsU64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScsValueS64 {
    pub value: ScsS64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScsValueFloat {
    pub value: ScsFloat,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScsValueDouble {
    pub value: ScsDouble,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScsValueString {
    pub value: ScsString,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScsFVector {
    pub x: ScsFloat,
    pub y: ScsFloat,
    pub z: ScsFloat,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScsDVector {
    pub x: ScsDouble,
    pub y: ScsDouble,
    pub z: ScsDouble,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScsEuler {
    pub heading: ScsFloat,
    pub pitch: ScsFloat,
    pub roll: ScsFloat,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScsFPlacement {
    pub position: ScsFVector,
    pub orientation: ScsEuler,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ScsDPlacement {
    pub position: ScsDVector,
    pub orientation: ScsEuler,
    pub padding: ScsPadding,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ScsValueData {
    pub value_bool: ScsValueBool,
    pub value_s32: ScsValueS32,
    pub value_u32: ScsValueU32,
    pub value_u64: ScsValueU64,
    pub value_s64: ScsValueS64,
    pub value_float: ScsValueFloat,
    pub value_double: ScsValueDouble,
    pub value_fvector: ScsFVector,
    pub value_dvector: ScsDVector,
    pub value_euler: ScsEuler,
    pub value_fplacement: ScsFPlacement,
    pub value_dplacement: ScsDPlacement,
    pub value_string: ScsValueString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ScsValue {
    pub type_: ScsValueType,
    pub padding: ScsPadding,
    pub value: ScsValueData,
}

#[repr(C)]
pub struct ScsNamedValue {
    pub name: ScsString,
    pub index: ScsU32,
    pub padding: ScsPadding,
    pub value: ScsValue,
}

const _: [(); 1] = [(); size_of::<ScsValueBool>()];
const _: [(); 4] = [(); size_of::<ScsValueS32>()];
const _: [(); 4] = [(); size_of::<ScsValueU32>()];
const _: [(); 8] = [(); size_of::<ScsValueU64>()];
const _: [(); 8] = [(); size_of::<ScsValueS64>()];
const _: [(); 4] = [(); size_of::<ScsValueFloat>()];
const _: [(); 8] = [(); size_of::<ScsValueDouble>()];
const _: [(); 8] = [(); size_of::<ScsValueString>()];
const _: [(); 12] = [(); size_of::<ScsFVector>()];
const _: [(); 24] = [(); size_of::<ScsDVector>()];
const _: [(); 12] = [(); size_of::<ScsEuler>()];
const _: [(); 24] = [(); size_of::<ScsFPlacement>()];
const _: [(); 40] = [(); size_of::<ScsDPlacement>()];
const _: [(); 40] = [(); size_of::<ScsValueData>()];
const _: [(); 48] = [(); size_of::<ScsValue>()];
const _: [(); 64] = [(); size_of::<ScsNamedValue>()];

const _: [(); 24] = [(); offset_of!(ScsDPlacement, orientation)];
const _: [(); 36] = [(); offset_of!(ScsDPlacement, padding)];
const _: [(); 8] = [(); offset_of!(ScsValue, value)];
const _: [(); 16] = [(); offset_of!(ScsNamedValue, value)];
