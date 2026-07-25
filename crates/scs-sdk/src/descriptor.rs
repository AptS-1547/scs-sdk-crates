use core::ffi::CStr;
use core::marker::PhantomData;

use crate::{GameSchemaVersion, SdkValue, ValueRef, ValueType, sys};

/// Per-game schema versions in which an official telemetry descriptor exists.
///
/// SCS versions three different contracts independently: the downloadable SDK
/// archive, the Telemetry API negotiated by `scs_telemetry_init`, and the game
/// telemetry schema supplied in the initialization parameters. Channel,
/// configuration, and gameplay descriptor additions belong to the last of
/// those domains. Consequently this value stores one minimum schema for ETS2
/// and another for ATS instead of pretending that an archive number or a
/// Telemetry API version answers the same question.
///
/// A `None` entry is meaningful: the current official game header explicitly
/// documents that descriptor as unavailable for that game. It is not an
/// unknown-version wildcard. The SDK 1.0 through 1.14 header history is the
/// source for the metadata attached to the built-in catalogs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameSchemaAvailability {
    ets2: Option<GameSchemaVersion>,
    ats: Option<GameSchemaVersion>,
}

impl GameSchemaAvailability {
    /// Describes the first schema which exposes a descriptor in each game.
    ///
    /// Use `None` only when the official headers explicitly exclude the
    /// descriptor for that game. Custom descriptors should still state their
    /// evidence explicitly instead of inheriting an implicit "all versions"
    /// policy.
    #[must_use]
    pub const fn new(ets2: Option<GameSchemaVersion>, ats: Option<GameSchemaVersion>) -> Self {
        Self { ets2, ats }
    }

    /// Oldest ETS2 telemetry schema which exposes the descriptor.
    #[must_use]
    pub const fn available_since_ets2(self) -> Option<GameSchemaVersion> {
        self.ets2
    }

    /// Oldest ATS telemetry schema which exposes the descriptor.
    #[must_use]
    pub const fn available_since_ats(self) -> Option<GameSchemaVersion> {
        self.ats
    }
}

/// Identifies one configuration group delivered by an SCS configuration event.
///
/// Examples include truck, trailer, active job, controls, and H-shifter
/// configuration. The identifier is a stable ASCII subset of UTF-8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigurationId {
    name: &'static CStr,
    availability: GameSchemaAvailability,
}

impl ConfigurationId {
    #[must_use]
    pub const fn new(name: &'static CStr, availability: GameSchemaAvailability) -> Self {
        Self { name, availability }
    }

    #[must_use]
    pub const fn name(self) -> &'static CStr {
        self.name
    }

    /// Official per-game schema history for this configuration identifier.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        self.availability
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
    availability: GameSchemaAvailability,
}

impl GameplayEventId {
    #[must_use]
    pub const fn new(name: &'static CStr, availability: GameSchemaAvailability) -> Self {
        Self { name, availability }
    }

    #[must_use]
    pub const fn name(self) -> &'static CStr {
        self.name
    }

    /// Official per-game schema history for this gameplay event identifier.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        self.availability
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
    availability: GameSchemaAvailability,
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
    availability: GameSchemaAvailability,
}

/// One configuration-group to attribute relationship from the SDK schema.
///
/// Attribute descriptors answer which name and value representation SCS uses;
/// they do not answer which configuration group carries that value. This
/// distinction matters for shared names such as `id`, `brand`, and
/// `wheel.position`, and for attributes added to an existing group later than
/// the attribute name itself first appeared elsewhere.
///
/// The relationship therefore owns separate per-game availability metadata.
/// Schema generators and diagnostics can enumerate these values without
/// weakening the typed [`Attribute<T>`] API used to decode callback data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigurationAttributeAssociation {
    configuration: ConfigurationId,
    attribute: AnyAttribute,
    availability: GameSchemaAvailability,
}

impl ConfigurationAttributeAssociation {
    /// Declares that one typed attribute belongs to a configuration group.
    ///
    /// `availability` must describe the relationship, not merely repeat the
    /// earliest schema of either descriptor. For example, `brand` existed on
    /// the truck configuration before it was added to trailer configuration.
    #[must_use]
    pub const fn new<T: SdkValue>(
        configuration: ConfigurationId,
        attribute: Attribute<T>,
        availability: GameSchemaAvailability,
    ) -> Self {
        Self {
            configuration,
            attribute: attribute.erase(),
            availability,
        }
    }

    /// Configuration group which carries the attribute.
    #[must_use]
    pub const fn configuration(self) -> ConfigurationId {
        self.configuration
    }

    /// Type-erased attribute metadata for enumeration and diagnostics.
    #[must_use]
    pub const fn attribute(self) -> AnyAttribute {
        self.attribute
    }

    /// First ETS2 and ATS schemas which expose this relationship.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        self.availability
    }
}

/// One gameplay-event to attribute relationship from the SDK schema.
///
/// Several gameplay events reuse attribute descriptors such as `pay.amount`,
/// `source.id`, and `target.id`. Keeping event membership outside
/// [`AnyAttribute`] preserves that many-to-many shape and gives schema tooling
/// an enumerable association catalog without duplicating typed descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameplayAttributeAssociation {
    event: GameplayEventId,
    attribute: AnyAttribute,
    availability: GameSchemaAvailability,
}

impl GameplayAttributeAssociation {
    /// Declares that one typed attribute is carried by a gameplay event.
    #[must_use]
    pub const fn new<T: SdkValue>(
        event: GameplayEventId,
        attribute: Attribute<T>,
        availability: GameSchemaAvailability,
    ) -> Self {
        Self {
            event,
            attribute: attribute.erase(),
            availability,
        }
    }

    /// Gameplay event which carries the attribute.
    #[must_use]
    pub const fn event(self) -> GameplayEventId {
        self.event
    }

    /// Type-erased attribute metadata for enumeration and diagnostics.
    #[must_use]
    pub const fn attribute(self) -> AnyAttribute {
        self.attribute
    }

    /// First ETS2 and ATS schemas which expose this relationship.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        self.availability
    }
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

    /// Official per-game schema history retained during type erasure.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        self.availability
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
    pub const fn new(name: &'static CStr, availability: GameSchemaAvailability) -> Self {
        Self {
            name,
            indexed: false,
            availability,
            marker: PhantomData,
        }
    }

    /// Creates an attribute descriptor selected by a zero-based SDK index.
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
    pub const fn is_indexed(self) -> bool {
        self.indexed
    }

    /// Official per-game schema history for this attribute.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        self.availability
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
            availability: self.availability,
        }
    }

    /// Decodes one attribute value after verifying its SCS type tag.
    #[must_use]
    pub fn decode(self, value: ValueRef<'_>) -> Option<T::Decoded<'_>> {
        T::decode(value)
    }
}
