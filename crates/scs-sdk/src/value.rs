use core::ffi::CStr;
use core::fmt;

use crate::{TelemetryApiVersion, sys};

mod sealed {
    pub trait Sealed {}
}

/// A three-dimensional single-precision vector reported by the SDK.
///
/// In vehicle-local space, positive X points right, positive Y points up, and
/// positive Z points backwards. In world space, positive X points east,
/// positive Y points up, and positive Z points south.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A three-dimensional double-precision vector reported by the SDK.
///
/// World positions use double precision because ETS2 and ATS maps are large
/// enough for single-precision coordinates to lose visible accuracy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DVector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Normalized object orientation used by the SCS SDK.
///
/// `heading` uses `[0.0, 1.0)` for `[0, 360)` degrees and increases
/// counter-clockwise when viewed from above. `pitch` normally uses
/// `[-0.25, 0.25]` for `[-90, 90]` degrees. `roll` normally uses
/// `[-0.5, 0.5]` for `[-180, 180]` degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Euler {
    pub heading: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// Single-precision position and orientation pair.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FPlacement {
    pub position: FVector,
    pub orientation: Euler,
}

/// Double-precision position with single-precision normalized orientation.
///
/// Unlike the ABI structure, this high-level type has no explicit alignment
/// padding. It is safe to copy, compare, serialize, and retain after the SDK
/// callback returns.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DPlacement {
    pub position: DVector,
    pub orientation: Euler,
}

/// Marker used by typed descriptors whose decoded value is a borrowed C
/// string. SCS strings are UTF-8 and remain valid only for the current callback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StringValue;

/// Error returned when a string is not in one documented SDK value catalog.
///
/// The original text remains owned or borrowed by the caller, so this compact
/// error carries no allocation. Callers which need forward-compatible logging
/// can report their input unchanged after a failed `FromStr` conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnknownStringValue;

impl fmt::Display for UnknownStringValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown SDK string value")
    }
}

impl From<sys::ScsFVector> for FVector {
    fn from(value: sys::ScsFVector) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl From<sys::ScsDVector> for DVector {
    fn from(value: sys::ScsDVector) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl From<sys::ScsEuler> for Euler {
    fn from(value: sys::ScsEuler) -> Self {
        Self {
            heading: value.heading,
            pitch: value.pitch,
            roll: value.roll,
        }
    }
}

impl From<sys::ScsFPlacement> for FPlacement {
    fn from(value: sys::ScsFPlacement) -> Self {
        Self {
            position: value.position.into(),
            orientation: value.orientation.into(),
        }
    }
}

impl From<sys::ScsDPlacement> for DPlacement {
    fn from(value: sys::ScsDPlacement) -> Self {
        Self {
            position: value.position.into(),
            orientation: value.orientation.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ValueType {
    Bool = sys::SCS_VALUE_TYPE_BOOL,
    I32 = sys::SCS_VALUE_TYPE_S32,
    U32 = sys::SCS_VALUE_TYPE_U32,
    U64 = sys::SCS_VALUE_TYPE_U64,
    F32 = sys::SCS_VALUE_TYPE_FLOAT,
    F64 = sys::SCS_VALUE_TYPE_DOUBLE,
    FVector = sys::SCS_VALUE_TYPE_FVECTOR,
    DVector = sys::SCS_VALUE_TYPE_DVECTOR,
    Euler = sys::SCS_VALUE_TYPE_EULER,
    FPlacement = sys::SCS_VALUE_TYPE_FPLACEMENT,
    DPlacement = sys::SCS_VALUE_TYPE_DPLACEMENT,
    String = sys::SCS_VALUE_TYPE_STRING,
    I64 = sys::SCS_VALUE_TYPE_S64,
}

impl ValueType {
    #[must_use]
    pub const fn from_raw(value: sys::ScsValueType) -> Option<Self> {
        match value {
            sys::SCS_VALUE_TYPE_BOOL => Some(Self::Bool),
            sys::SCS_VALUE_TYPE_S32 => Some(Self::I32),
            sys::SCS_VALUE_TYPE_U32 => Some(Self::U32),
            sys::SCS_VALUE_TYPE_U64 => Some(Self::U64),
            sys::SCS_VALUE_TYPE_FLOAT => Some(Self::F32),
            sys::SCS_VALUE_TYPE_DOUBLE => Some(Self::F64),
            sys::SCS_VALUE_TYPE_FVECTOR => Some(Self::FVector),
            sys::SCS_VALUE_TYPE_DVECTOR => Some(Self::DVector),
            sys::SCS_VALUE_TYPE_EULER => Some(Self::Euler),
            sys::SCS_VALUE_TYPE_FPLACEMENT => Some(Self::FPlacement),
            sys::SCS_VALUE_TYPE_DPLACEMENT => Some(Self::DPlacement),
            sys::SCS_VALUE_TYPE_STRING => Some(Self::String),
            sys::SCS_VALUE_TYPE_S64 => Some(Self::I64),
            _ => None,
        }
    }

    /// Returns the numeric discriminator used by the C ABI.
    ///
    /// This is primarily useful to framework crates which must retain an SDK
    /// value type after erasing the Rust marker type from a descriptor. Normal
    /// plugin code should prefer the typed decoding methods on [`crate::Channel`]
    /// and [`crate::Attribute`].
    #[must_use]
    pub const fn raw(self) -> sys::ScsValueType {
        self as sys::ScsValueType
    }

    /// Oldest Telemetry API which defines this tagged-union representation.
    ///
    /// The Telemetry API 1.01 changelog explicitly adds signed 64-bit values.
    /// Every other value tag exposed by SDK 1.14 belongs to API 1.00. This is
    /// representation availability, not a promise that every channel supports
    /// every representation: SCS still performs the channel-specific
    /// conversion check during registration.
    #[must_use]
    pub const fn minimum_api_version(self) -> TelemetryApiVersion {
        match self {
            Self::I64 => TelemetryApiVersion::V1_01,
            Self::Bool
            | Self::I32
            | Self::U32
            | Self::U64
            | Self::F32
            | Self::F64
            | Self::FVector
            | Self::DVector
            | Self::Euler
            | Self::FPlacement
            | Self::DPlacement
            | Self::String => TelemetryApiVersion::V1_00,
        }
    }
}

/// Associates a Rust type with one SCS tagged-union member and its safe
/// callback-time decoded representation.
///
/// Primitive and geometry values decode by value. [`StringValue`] decodes to a
/// borrowed [`CStr`] whose lifetime cannot outlive the SDK callback.
pub trait SdkValue: sealed::Sealed {
    type Decoded<'a>;

    /// High-level discriminator retained by type-erased framework descriptors.
    const VALUE_TYPE: ValueType;

    /// Numeric discriminator passed across the C ABI.
    const TYPE: sys::ScsValueType;

    fn decode(value: ValueRef<'_>) -> Option<Self::Decoded<'_>>;
}

macro_rules! scalar_sdk_value {
    ($type:ty, $value_type:expr, $tag:expr, $getter:ident) => {
        impl sealed::Sealed for $type {}

        impl SdkValue for $type {
            type Decoded<'a> = $type;

            const VALUE_TYPE: ValueType = $value_type;
            const TYPE: sys::ScsValueType = $tag;

            fn decode(value: ValueRef<'_>) -> Option<Self::Decoded<'_>> {
                value.$getter()
            }
        }
    };
}

scalar_sdk_value!(bool, ValueType::Bool, sys::SCS_VALUE_TYPE_BOOL, as_bool);
scalar_sdk_value!(i32, ValueType::I32, sys::SCS_VALUE_TYPE_S32, as_i32);
scalar_sdk_value!(u32, ValueType::U32, sys::SCS_VALUE_TYPE_U32, as_u32);
scalar_sdk_value!(u64, ValueType::U64, sys::SCS_VALUE_TYPE_U64, as_u64);
scalar_sdk_value!(i64, ValueType::I64, sys::SCS_VALUE_TYPE_S64, as_i64);
scalar_sdk_value!(f32, ValueType::F32, sys::SCS_VALUE_TYPE_FLOAT, as_f32);
scalar_sdk_value!(f64, ValueType::F64, sys::SCS_VALUE_TYPE_DOUBLE, as_f64);
scalar_sdk_value!(
    FVector,
    ValueType::FVector,
    sys::SCS_VALUE_TYPE_FVECTOR,
    as_fvector
);
scalar_sdk_value!(
    DVector,
    ValueType::DVector,
    sys::SCS_VALUE_TYPE_DVECTOR,
    as_dvector
);
scalar_sdk_value!(Euler, ValueType::Euler, sys::SCS_VALUE_TYPE_EULER, as_euler);
scalar_sdk_value!(
    FPlacement,
    ValueType::FPlacement,
    sys::SCS_VALUE_TYPE_FPLACEMENT,
    as_fplacement
);
scalar_sdk_value!(
    DPlacement,
    ValueType::DPlacement,
    sys::SCS_VALUE_TYPE_DPLACEMENT,
    as_dplacement
);

impl sealed::Sealed for StringValue {}

impl SdkValue for StringValue {
    type Decoded<'a> = &'a CStr;

    const VALUE_TYPE: ValueType = ValueType::String;
    const TYPE: sys::ScsValueType = sys::SCS_VALUE_TYPE_STRING;

    fn decode(value: ValueRef<'_>) -> Option<Self::Decoded<'_>> {
        value.as_c_str()
    }
}

#[derive(Clone, Copy)]
pub struct ValueRef<'a> {
    raw: &'a sys::ScsValue,
}

impl<'a> ValueRef<'a> {
    pub(crate) const fn from_ref(raw: &'a sys::ScsValue) -> Self {
        Self { raw }
    }

    /// Borrows a tagged SDK value for the duration of its callback.
    ///
    /// # Safety
    ///
    /// `value` must either be null or point to a correctly aligned, initialized
    /// [`sys::ScsValue`] that remains alive for `'a`. For a known type tag, the
    /// corresponding union member must be initialized. Unknown tags are
    /// preserved without reading the union. String members must be non-null,
    /// NUL-terminated, and remain alive for the same lifetime.
    #[must_use]
    pub unsafe fn from_ptr(value: *const sys::ScsValue) -> Option<Self> {
        // SAFETY: The caller guarantees that a non-null pointer is aligned,
        // initialized as `ScsValue`, and valid for the returned lifetime.
        // `as_ref` additionally maps a null pointer to `None`.
        unsafe { value.as_ref() }.map(|raw| Self { raw })
    }

    #[must_use]
    pub const fn raw_type(self) -> sys::ScsValueType {
        self.raw.type_
    }

    #[must_use]
    pub const fn value_type(self) -> Option<ValueType> {
        ValueType::from_raw(self.raw.type_)
    }

    #[must_use]
    pub fn as_bool(self) -> Option<bool> {
        if self.value_type() == Some(ValueType::Bool) {
            // SAFETY: The checked tag selects the initialized bool member.
            Some(unsafe { self.raw.value.value_bool.value != 0 })
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_i32(self) -> Option<i32> {
        if self.value_type() == Some(ValueType::I32) {
            // SAFETY: The checked tag selects the initialized signed-32 member.
            Some(unsafe { self.raw.value.value_s32.value })
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_u32(self) -> Option<u32> {
        if self.value_type() == Some(ValueType::U32) {
            // SAFETY: The checked tag selects the initialized unsigned-32 member.
            Some(unsafe { self.raw.value.value_u32.value })
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_u64(self) -> Option<u64> {
        if self.value_type() == Some(ValueType::U64) {
            // SAFETY: The checked tag selects the initialized unsigned-64 member.
            Some(unsafe { self.raw.value.value_u64.value })
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_i64(self) -> Option<i64> {
        if self.value_type() == Some(ValueType::I64) {
            // SAFETY: The checked tag selects the initialized signed-64 member.
            Some(unsafe { self.raw.value.value_s64.value })
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_f32(self) -> Option<f32> {
        if self.value_type() == Some(ValueType::F32) {
            // SAFETY: The checked tag selects the initialized float member.
            Some(unsafe { self.raw.value.value_float.value })
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_f64(self) -> Option<f64> {
        if self.value_type() == Some(ValueType::F64) {
            // SAFETY: The checked tag selects the initialized double member.
            Some(unsafe { self.raw.value.value_double.value })
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_fvector(self) -> Option<FVector> {
        if self.value_type() == Some(ValueType::FVector) {
            // SAFETY: The checked tag selects the initialized float-vector member.
            Some(unsafe { self.raw.value.value_fvector }.into())
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_dvector(self) -> Option<DVector> {
        if self.value_type() == Some(ValueType::DVector) {
            // SAFETY: The checked tag selects the initialized double-vector member.
            Some(unsafe { self.raw.value.value_dvector }.into())
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_euler(self) -> Option<Euler> {
        if self.value_type() == Some(ValueType::Euler) {
            // SAFETY: The checked tag selects the initialized Euler member.
            Some(unsafe { self.raw.value.value_euler }.into())
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_fplacement(self) -> Option<FPlacement> {
        if self.value_type() == Some(ValueType::FPlacement) {
            // SAFETY: The checked tag selects the initialized float-placement member.
            Some(unsafe { self.raw.value.value_fplacement }.into())
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_dplacement(self) -> Option<DPlacement> {
        if self.value_type() == Some(ValueType::DPlacement) {
            // SAFETY: The checked tag selects the initialized double-placement member.
            Some(unsafe { self.raw.value.value_dplacement }.into())
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_c_str(self) -> Option<&'a CStr> {
        if self.value_type() != Some(ValueType::String) {
            return None;
        }

        // SAFETY: The checked tag selects the initialized string member. Its
        // pointer is validated separately before constructing a `CStr`.
        let pointer = unsafe { self.raw.value.value_string.value };
        if pointer.is_null() {
            return None;
        }

        // SAFETY: String values are guaranteed to be non-null, NUL-terminated,
        // and valid for the duration of the current SDK callback.
        Some(unsafe { CStr::from_ptr(pointer) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_ref(raw: &sys::ScsValue) -> ValueRef<'_> {
        // SAFETY: A shared Rust reference proves alignment, initialization of
        // the outer `ScsValue`, and validity for the returned borrow. Each test
        // fixture initializes the union member selected by its known tag; an
        // unknown tag is tested only through operations that do not read it.
        unsafe { ValueRef::from_ptr(raw) }.expect("value reference should be present")
    }

    #[test]
    fn signed_64_bit_values_are_the_only_v101_representation() {
        for value_type in [
            ValueType::Bool,
            ValueType::I32,
            ValueType::U32,
            ValueType::U64,
            ValueType::F32,
            ValueType::F64,
            ValueType::FVector,
            ValueType::DVector,
            ValueType::Euler,
            ValueType::FPlacement,
            ValueType::DPlacement,
            ValueType::String,
        ] {
            assert_eq!(value_type.minimum_api_version(), TelemetryApiVersion::V1_00);
        }
        assert_eq!(
            ValueType::I64.minimum_api_version(),
            TelemetryApiVersion::V1_01
        );
    }

    #[test]
    fn decodes_only_the_tagged_union_member() {
        let raw = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_FLOAT,
            padding: sys::ScsPadding::new(0),
            value: sys::ScsValueData {
                value_float: sys::ScsValueFloat { value: 12.5 },
            },
        };
        let value = value_ref(&raw);

        assert_eq!(value.as_f32(), Some(12.5));
        assert_eq!(value.as_i32(), None);
        assert_eq!(value.value_type(), Some(ValueType::F32));
    }

    #[test]
    fn preserves_unknown_value_tags() {
        let raw = sys::ScsValue {
            type_: 99,
            padding: sys::ScsPadding::new(0),
            value: sys::ScsValueData {
                value_u64: sys::ScsValueU64 { value: 0 },
            },
        };
        let value = value_ref(&raw);

        assert_eq!(value.raw_type(), 99);
        assert_eq!(value.value_type(), None);
    }

    #[test]
    fn mismatched_getters_do_not_decode_an_inactive_union_member() {
        let raw = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_BOOL,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_bool: sys::ScsValueBool { value: 1 },
            },
        };
        let value = value_ref(&raw);

        assert_eq!(value.as_bool(), Some(true));
        assert_eq!(value.as_i32(), None);
        assert_eq!(value.as_u32(), None);
        assert_eq!(value.as_i64(), None);
        assert_eq!(value.as_u64(), None);
        assert_eq!(value.as_f32(), None);
        assert_eq!(value.as_f64(), None);
        assert!(value.as_dplacement().is_none());
        assert_eq!(value.as_c_str(), None);
    }

    #[test]
    fn decodes_every_scalar_sdk_value_type() {
        let signed_32 = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_S32,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_s32: sys::ScsValueS32 { value: -32 },
            },
        };
        let unsigned_32 = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_U32,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_u32: sys::ScsValueU32 { value: 32 },
            },
        };
        let signed_64 = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_S64,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_s64: sys::ScsValueS64 { value: -64 },
            },
        };
        let unsigned_64 = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_U64,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_u64: sys::ScsValueU64 { value: 64 },
            },
        };
        let double = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_DOUBLE,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_double: sys::ScsValueDouble { value: 12.25 },
            },
        };
        let string = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_STRING,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_string: sys::ScsValueString {
                    value: c"telemetry".as_ptr(),
                },
            },
        };

        let signed_32 = value_ref(&signed_32);
        let unsigned_32 = value_ref(&unsigned_32);
        let signed_64 = value_ref(&signed_64);
        let unsigned_64 = value_ref(&unsigned_64);
        let double = value_ref(&double);
        let string = value_ref(&string);

        assert_eq!(signed_32.as_i32(), Some(-32));
        assert_eq!(unsigned_32.as_u32(), Some(32));
        assert_eq!(signed_64.as_i64(), Some(-64));
        assert_eq!(unsigned_64.as_u64(), Some(64));
        assert_eq!(double.as_f64(), Some(12.25));
        assert_eq!(string.as_c_str(), Some(c"telemetry"));
    }

    #[test]
    fn decodes_vector_and_euler_geometry_types() {
        let fvector = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_FVECTOR,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_fvector: sys::ScsFVector {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
            },
        };
        let dvector = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_DVECTOR,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_dvector: sys::ScsDVector {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                },
            },
        };
        let euler = sys::ScsEuler {
            heading: 0.25,
            pitch: -0.125,
            roll: 0.5,
        };
        let raw_euler = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_EULER,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData { value_euler: euler },
        };
        let fvector = value_ref(&fvector);
        let dvector = value_ref(&dvector);
        let raw_euler = value_ref(&raw_euler);

        assert_eq!(
            fvector.as_fvector(),
            Some(FVector {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );
        assert_eq!(
            dvector.as_dvector(),
            Some(DVector {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            })
        );
        assert_eq!(
            raw_euler.as_euler(),
            Some(Euler {
                heading: 0.25,
                pitch: -0.125,
                roll: 0.5,
            })
        );
    }

    #[test]
    fn decodes_placements_without_reading_uninitialized_abi_padding() {
        let euler = sys::ScsEuler {
            heading: 0.25,
            pitch: -0.125,
            roll: 0.5,
        };
        let fplacement = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_FPLACEMENT,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_fplacement: sys::ScsFPlacement {
                    position: sys::ScsFVector {
                        x: 7.0,
                        y: 8.0,
                        z: 9.0,
                    },
                    orientation: euler,
                },
            },
        };
        let dplacement = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_DPLACEMENT,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_dplacement: sys::ScsDPlacement {
                    position: sys::ScsDVector {
                        x: 10.0,
                        y: 11.0,
                        z: 12.0,
                    },
                    orientation: euler,
                    padding: sys::ScsPadding::uninit(),
                },
            },
        };

        let fplacement = value_ref(&fplacement);
        let dplacement = value_ref(&dplacement);
        assert_eq!(
            fplacement.as_fplacement(),
            Some(FPlacement {
                position: FVector {
                    x: 7.0,
                    y: 8.0,
                    z: 9.0,
                },
                orientation: Euler {
                    heading: 0.25,
                    pitch: -0.125,
                    roll: 0.5,
                },
            })
        );
        assert_eq!(
            dplacement.as_dplacement(),
            Some(DPlacement {
                position: DVector {
                    x: 10.0,
                    y: 11.0,
                    z: 12.0,
                },
                orientation: Euler {
                    heading: 0.25,
                    pitch: -0.125,
                    roll: 0.5,
                },
            })
        );
    }
}
