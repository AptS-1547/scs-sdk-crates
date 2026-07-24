use core::ffi::CStr;
use core::marker::PhantomData;

use crate::{SdkValue, ValueRef, ValueType, sys};

/// Identifies one configuration group delivered by an SCS configuration event.
///
/// Examples include truck, trailer, active job, controls, and H-shifter
/// configuration. The identifier is a stable ASCII subset of UTF-8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigurationId {
    name: &'static CStr,
}

impl ConfigurationId {
    #[must_use]
    pub const fn new(name: &'static CStr) -> Self {
        Self { name }
    }

    #[must_use]
    pub const fn name(self) -> &'static CStr {
        self.name
    }
}

/// Identifies one gameplay event delivered by the Telemetry API.
///
/// The descriptor only identifies the event. Its typed attributes are declared
/// separately because several events share names such as payment amount,
/// source ID, and target ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameplayEventId {
    name: &'static CStr,
}

impl GameplayEventId {
    #[must_use]
    pub const fn new(name: &'static CStr) -> Self {
        Self { name }
    }

    #[must_use]
    pub const fn name(self) -> &'static CStr {
        self.name
    }
}

/// A typed configuration or gameplay-event attribute descriptor.
///
/// `T` selects the expected SCS tagged-union member and the decoded Rust value.
/// Indexed descriptors represent arrays such as wheel positions or forward
/// gear ratios; callers must select an explicit zero-based index for them.
#[derive(Debug, PartialEq, Eq)]
pub struct Attribute<T: SdkValue> {
    name: &'static CStr,
    indexed: bool,
    marker: PhantomData<fn() -> T>,
}

/// A configuration or gameplay attribute after erasing its Rust marker type.
///
/// Catalogs need one homogeneous representation even though attributes decode
/// to different Rust types. Erasure deliberately retains all metadata needed
/// to audit the descriptor against the SDK header: the canonical name, tagged
/// union discriminator, and whether lookup requires a zero-based index.
///
/// Normal callback code should keep using [`Attribute<T>`](Attribute), because
/// that preserves typed decoding. `AnyAttribute` is intended for enumeration,
/// diagnostics, schema generation, and coverage tests rather than replacing
/// the typed API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnyAttribute {
    name: &'static CStr,
    value_type: ValueType,
    indexed: bool,
}

impl AnyAttribute {
    /// Returns the canonical, NUL-terminated attribute name from the SDK
    /// header.
    #[must_use]
    pub const fn name(self) -> &'static CStr {
        self.name
    }

    /// Returns the SCS tagged-union member expected for this attribute.
    #[must_use]
    pub const fn value_type(self) -> ValueType {
        self.value_type
    }

    /// Returns whether lookup requires an explicit zero-based SDK index.
    ///
    /// Indexed attributes include wheel properties, transmission ratios, and
    /// H-shifter slots. Scalar attributes must instead be looked up using the
    /// SDK sentinel index `SCS_U32_NIL`.
    #[must_use]
    pub const fn is_indexed(self) -> bool {
        self.indexed
    }
}

impl<T: SdkValue> Clone for Attribute<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: SdkValue> Copy for Attribute<T> {}

impl<T: SdkValue> Attribute<T> {
    /// Creates a scalar attribute descriptor whose SDK index is `SCS_U32_NIL`.
    #[must_use]
    pub const fn new(name: &'static CStr) -> Self {
        Self {
            name,
            indexed: false,
            marker: PhantomData,
        }
    }

    /// Creates an attribute descriptor selected by a zero-based SDK index.
    #[must_use]
    pub const fn indexed(name: &'static CStr) -> Self {
        Self {
            name,
            indexed: true,
            marker: PhantomData,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static CStr {
        self.name
    }

    #[must_use]
    pub const fn is_indexed(self) -> bool {
        self.indexed
    }

    #[must_use]
    pub const fn value_type(self) -> sys::ScsValueType {
        T::TYPE
    }

    /// Erases the Rust marker type while preserving the complete descriptor
    /// metadata required for catalog enumeration.
    ///
    /// This does not decode values and does not weaken [`Attribute::decode`].
    /// The returned descriptor remains explicit about both the SCS value type
    /// and indexed/scalar lookup mode.
    #[must_use]
    pub const fn erase(self) -> AnyAttribute {
        AnyAttribute {
            name: self.name,
            value_type: T::VALUE_TYPE,
            indexed: self.indexed,
        }
    }

    /// Decodes one attribute value after verifying its SCS type tag.
    #[must_use]
    pub fn decode(self, value: ValueRef<'_>) -> Option<T::Decoded<'_>> {
        T::decode(value)
    }
}
