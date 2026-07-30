use core::ffi::{CStr, c_void};
use core::marker::PhantomData;

use crate::{
    Attribute, ConfigurationId, GameSchemaAvailability, GameplayEventId, SdkIndex, SdkValue,
    TelemetryApiVersion, TrailerConfigurationId, TrailerIndex, ValueRef, game, sys,
};

/// Telemetry event which may be registered with the SCS SDK.
///
/// This type is the single typed representation of an SDK event identifier.
/// Higher framework layers may re-export it under an application-facing name,
/// but must not mirror its variants in a second enum: doing so would create a
/// second event catalog which could drift when SCS adds another event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Event {
    FrameStart = sys::SCS_TELEMETRY_EVENT_FRAME_START,
    FrameEnd = sys::SCS_TELEMETRY_EVENT_FRAME_END,
    Paused = sys::SCS_TELEMETRY_EVENT_PAUSED,
    Started = sys::SCS_TELEMETRY_EVENT_STARTED,
    Configuration = sys::SCS_TELEMETRY_EVENT_CONFIGURATION,
    Gameplay = sys::SCS_TELEMETRY_EVENT_GAMEPLAY,
}

impl Event {
    /// Raw event discriminator passed to SCS registration functions and back
    /// to the registered callback.
    #[must_use]
    pub const fn raw(self) -> sys::ScsEvent {
        self as sys::ScsEvent
    }

    /// Oldest Telemetry API which defines this event identifier.
    ///
    /// SCS SDK 1.14 documents gameplay events as an API 1.01 addition. The
    /// other identifiers belong to the original API 1.00 event set. Keeping
    /// this capability metadata beside the canonical event enum ensures that
    /// registration policy and the raw numeric identifier cannot acquire
    /// separate, independently maintained event inventories.
    #[must_use]
    pub const fn minimum_api_version(self) -> TelemetryApiVersion {
        match self {
            Self::Gameplay => TelemetryApiVersion::V1_01,
            Self::FrameStart
            | Self::FrameEnd
            | Self::Paused
            | Self::Started
            | Self::Configuration => TelemetryApiVersion::V1_00,
        }
    }

    /// Oldest per-game schema which can emit this SDK event kind.
    ///
    /// Gameplay callbacks require both Telemetry API 1.01 and the game schema
    /// which introduced gameplay events. Keeping those checks separate avoids
    /// comparing unrelated version domains. All original lifecycle and
    /// configuration event kinds exist from the first published game schemas.
    #[must_use]
    pub const fn availability(self) -> GameSchemaAvailability {
        match self {
            Self::Gameplay => game::capabilities::GAMEPLAY_EVENTS,
            Self::FrameStart
            | Self::FrameEnd
            | Self::Paused
            | Self::Started
            | Self::Configuration => {
                GameSchemaAvailability::new(Some(game::ets2::V1_00), Some(game::ats::V1_00))
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct FrameStartRef<'a> {
    raw: &'a sys::ScsTelemetryFrameStart,
}

impl FrameStartRef<'_> {
    /// Borrows frame-start data supplied by SCS.
    ///
    /// # Safety
    ///
    /// `event_info` must be the non-null pointer supplied for a frame-start
    /// event. It must be correctly aligned, point to an initialized
    /// [`sys::ScsTelemetryFrameStart`], and remain valid for the returned view.
    #[must_use]
    pub unsafe fn from_event_info(event_info: *const c_void) -> Option<Self> {
        // SAFETY: The caller guarantees that a non-null pointer is aligned,
        // initialized as `ScsTelemetryFrameStart`, and valid for the view's
        // lifetime. `as_ref` additionally maps a null pointer to `None`.
        unsafe { event_info.cast::<sys::ScsTelemetryFrameStart>().as_ref() }.map(|raw| Self { raw })
    }

    #[must_use]
    pub const fn flags(self) -> sys::ScsU32 {
        self.raw.flags
    }

    #[must_use]
    pub const fn render_time(self) -> sys::ScsTimestamp {
        self.raw.render_time
    }

    #[must_use]
    pub const fn simulation_time(self) -> sys::ScsTimestamp {
        self.raw.simulation_time
    }

    #[must_use]
    pub const fn paused_simulation_time(self) -> sys::ScsTimestamp {
        self.raw.paused_simulation_time
    }

    #[must_use]
    pub const fn timer_restarted(self) -> bool {
        self.raw.flags & sys::SCS_TELEMETRY_FRAME_START_FLAG_TIMER_RESTART != 0
    }
}

#[derive(Clone, Copy)]
pub struct NamedValueRef<'a> {
    raw: &'a sys::ScsNamedValue,
}

impl<'a> NamedValueRef<'a> {
    #[must_use]
    pub fn name(self) -> &'a CStr {
        // SAFETY: A non-terminal named value always has a valid name pointer.
        unsafe { CStr::from_ptr(self.raw.name) }
    }

    #[must_use]
    pub fn index(self) -> Option<SdkIndex> {
        if self.raw.index == sys::SCS_U32_NIL {
            None
        } else {
            SdkIndex::new(self.raw.index)
        }
    }

    #[must_use]
    pub fn value(self) -> ValueRef<'a> {
        ValueRef::from_ref(&self.raw.value)
    }
}

#[derive(Clone)]
pub struct NamedValues<'a> {
    current: *const sys::ScsNamedValue,
    marker: PhantomData<&'a sys::ScsNamedValue>,
}

impl<'a> NamedValues<'a> {
    /// Creates an iterator over a null-name-terminated SCS attribute array.
    ///
    /// # Safety
    ///
    /// `attributes` must point to an initialized, contiguous SDK array terminated
    /// by an entry with a null `name`. Every preceding name must point to a valid
    /// NUL-terminated string, every value tag must match its initialized union
    /// member, and the entire array and its referenced data must remain alive for
    /// `'a`.
    #[must_use]
    pub unsafe fn from_ptr(attributes: *const sys::ScsNamedValue) -> Self {
        Self {
            current: attributes,
            marker: PhantomData,
        }
    }

    #[must_use]
    pub fn find(self, name: &CStr) -> Option<NamedValueRef<'a>> {
        self.filter(|attribute| attribute.index().is_none())
            .find(|attribute| attribute.name() == name)
    }

    /// Finds one indexed attribute by its name and zero-based SDK index.
    #[must_use]
    pub fn find_at(self, name: &CStr, index: SdkIndex) -> Option<NamedValueRef<'a>> {
        self.filter(|attribute| attribute.index() == Some(index))
            .find(|attribute| attribute.name() == name)
    }

    /// Decodes a scalar attribute using its typed descriptor.
    #[must_use]
    pub fn get<T: SdkValue>(self, attribute: Attribute<T>) -> Option<T::Decoded<'a>> {
        if attribute.is_indexed() {
            return None;
        }
        attribute.decode(self.find(attribute.name())?.value())
    }

    /// Decodes one member of an indexed attribute using its typed descriptor.
    #[must_use]
    pub fn get_at<T: SdkValue>(
        self,
        attribute: Attribute<T>,
        index: SdkIndex,
    ) -> Option<T::Decoded<'a>> {
        if !attribute.is_indexed() {
            return None;
        }
        attribute.decode(self.find_at(attribute.name(), index)?.value())
    }
}

impl<'a> Iterator for NamedValues<'a> {
    type Item = NamedValueRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: `from_ptr` requires a contiguous array that remains valid
        // through its null-name terminator. `current` starts at its first entry
        // and advances only after observing a non-terminal entry.
        let raw = unsafe { self.current.as_ref() }?;
        if raw.name.is_null() {
            return None;
        }

        // SAFETY: A non-null name identifies a non-terminal entry, so the
        // constructor contract guarantees that the next contiguous entry
        // exists, possibly as the required terminator.
        self.current = unsafe { self.current.add(1) };
        Some(NamedValueRef { raw })
    }
}

#[derive(Clone, Copy)]
pub struct ConfigurationRef<'a> {
    raw: &'a sys::ScsTelemetryConfiguration,
}

impl<'a> ConfigurationRef<'a> {
    /// Borrows configuration event data supplied by SCS.
    ///
    /// # Safety
    ///
    /// `event_info` must be the non-null pointer supplied for a configuration
    /// event. The structure must be correctly aligned and initialized; its ID
    /// must be a valid NUL-terminated string, and its attributes must satisfy
    /// [`NamedValues::from_ptr`]. All referenced data must remain alive for `'a`.
    #[must_use]
    pub unsafe fn from_event_info(event_info: *const c_void) -> Option<Self> {
        // SAFETY: The caller guarantees that a non-null pointer is aligned,
        // initialized as `ScsTelemetryConfiguration`, and valid together with
        // all referenced strings and attributes for the returned lifetime.
        unsafe { event_info.cast::<sys::ScsTelemetryConfiguration>().as_ref() }
            .map(|raw| Self { raw })
    }

    #[must_use]
    pub fn id(self) -> &'a CStr {
        // SAFETY: The SDK guarantees a valid ID string for configuration events.
        unsafe { CStr::from_ptr(self.raw.id) }
    }

    /// Tests this event against a typed configuration identifier.
    #[must_use]
    pub fn is(self, id: ConfigurationId) -> bool {
        self.id() == id.name()
    }

    /// Classifies an unnumbered or numbered trailer configuration ID.
    ///
    /// Canonical numbered IDs use an unsigned decimal index without leading
    /// zeroes. Malformed names, custom configuration IDs, and values outside
    /// the SDK's `0..10` trailer range return `None` rather than being confused
    /// with the legacy compatibility alias.
    #[must_use]
    pub fn trailer(self) -> Option<TrailerConfigurationId> {
        let id = self.id();
        if id == crate::configuration::ids::TRAILER.name() {
            return Some(TrailerConfigurationId::Legacy);
        }
        let digits = id.to_bytes().strip_prefix(b"trailer.")?;
        if digits.is_empty() || (digits.len() > 1 && digits[0] == b'0') {
            return None;
        }

        let mut raw = 0_u32;
        for digit in digits {
            if !digit.is_ascii_digit() {
                return None;
            }
            raw = raw.checked_mul(10)?.checked_add(u32::from(*digit - b'0'))?;
        }
        TrailerIndex::new(raw).map(TrailerConfigurationId::Numbered)
    }

    /// Returns the numbered trailer index, excluding the legacy `trailer` ID.
    #[must_use]
    pub fn trailer_index(self) -> Option<TrailerIndex> {
        self.trailer()?.index()
    }

    /// Whether this event uses the legacy unnumbered `trailer` identity.
    #[must_use]
    pub fn is_legacy_trailer(self) -> bool {
        self.trailer()
            .is_some_and(TrailerConfigurationId::is_legacy)
    }

    #[must_use]
    pub fn attributes(self) -> NamedValues<'a> {
        // SAFETY: The SDK guarantees a terminated attribute array.
        unsafe { NamedValues::from_ptr(self.raw.attributes) }
    }
}

#[derive(Clone, Copy)]
pub struct GameplayEventRef<'a> {
    raw: &'a sys::ScsTelemetryGameplayEvent,
}

impl<'a> GameplayEventRef<'a> {
    /// Borrows gameplay event data supplied by SCS.
    ///
    /// # Safety
    ///
    /// `event_info` must be the non-null pointer supplied for a gameplay event.
    /// The structure must be correctly aligned and initialized; its ID must be a
    /// valid NUL-terminated string, and its attributes must satisfy
    /// [`NamedValues::from_ptr`]. All referenced data must remain alive for `'a`.
    #[must_use]
    pub unsafe fn from_event_info(event_info: *const c_void) -> Option<Self> {
        // SAFETY: The caller guarantees that a non-null pointer is aligned,
        // initialized as `ScsTelemetryGameplayEvent`, and valid together with
        // all referenced strings and attributes for the returned lifetime.
        unsafe { event_info.cast::<sys::ScsTelemetryGameplayEvent>().as_ref() }
            .map(|raw| Self { raw })
    }

    #[must_use]
    pub fn id(self) -> &'a CStr {
        // SAFETY: The SDK guarantees a valid ID string for gameplay events.
        unsafe { CStr::from_ptr(self.raw.id) }
    }

    /// Tests this event against a typed gameplay event identifier.
    #[must_use]
    pub fn is(self, id: GameplayEventId) -> bool {
        self.id() == id.name()
    }

    #[must_use]
    pub fn attributes(self) -> NamedValues<'a> {
        // SAFETY: The SDK guarantees a terminated attribute array.
        unsafe { NamedValues::from_ptr(self.raw.attributes) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_capabilities_follow_the_official_api_history() {
        assert_eq!(
            Event::FrameStart.raw(),
            sys::SCS_TELEMETRY_EVENT_FRAME_START
        );
        assert_eq!(Event::FrameEnd.raw(), sys::SCS_TELEMETRY_EVENT_FRAME_END);
        assert_eq!(Event::Paused.raw(), sys::SCS_TELEMETRY_EVENT_PAUSED);
        assert_eq!(Event::Started.raw(), sys::SCS_TELEMETRY_EVENT_STARTED);
        assert_eq!(
            Event::Configuration.raw(),
            sys::SCS_TELEMETRY_EVENT_CONFIGURATION
        );
        assert_eq!(Event::Gameplay.raw(), sys::SCS_TELEMETRY_EVENT_GAMEPLAY);

        assert_eq!(
            Event::FrameStart.minimum_api_version(),
            TelemetryApiVersion::V1_00
        );
        assert_eq!(
            Event::FrameEnd.minimum_api_version(),
            TelemetryApiVersion::V1_00
        );
        assert_eq!(
            Event::Paused.minimum_api_version(),
            TelemetryApiVersion::V1_00
        );
        assert_eq!(
            Event::Started.minimum_api_version(),
            TelemetryApiVersion::V1_00
        );
        assert_eq!(
            Event::Configuration.minimum_api_version(),
            TelemetryApiVersion::V1_00
        );
        assert_eq!(
            Event::Gameplay.minimum_api_version(),
            TelemetryApiVersion::V1_01
        );

        for event in [
            Event::FrameStart,
            Event::FrameEnd,
            Event::Paused,
            Event::Started,
            Event::Configuration,
        ] {
            assert_eq!(
                event.availability().available_since_ets2(),
                Some(game::ets2::V1_00)
            );
            assert_eq!(
                event.availability().available_since_ats(),
                Some(game::ats::V1_00)
            );
        }
        assert_eq!(
            Event::Gameplay.availability().available_since_ets2(),
            Some(game::ets2::V1_14)
        );
        assert_eq!(
            Event::Gameplay.availability().available_since_ats(),
            Some(game::ats::V1_01)
        );
    }

    #[test]
    fn iterates_and_finds_unindexed_attributes() {
        let cargo_name = c"cargo";
        let cargo_value = c"盆栽花朵";
        let attributes = [
            sys::ScsNamedValue {
                name: cargo_name.as_ptr(),
                index: sys::SCS_U32_NIL,
                padding: sys::ScsPadding::uninit(),
                value: sys::ScsValue {
                    type_: sys::SCS_VALUE_TYPE_STRING,
                    padding: sys::ScsPadding::uninit(),
                    value: sys::ScsValueData {
                        value_string: sys::ScsValueString {
                            value: cargo_value.as_ptr(),
                        },
                    },
                },
            },
            sys::ScsNamedValue {
                name: core::ptr::null(),
                index: 0,
                padding: sys::ScsPadding::uninit(),
                value: sys::ScsValue {
                    type_: sys::SCS_VALUE_TYPE_INVALID,
                    padding: sys::ScsPadding::uninit(),
                    value: sys::ScsValueData {
                        value_u64: sys::ScsValueU64 { value: 0 },
                    },
                },
            },
        ];

        // SAFETY: `attributes` is contiguous, ends with a null-name entry, and
        // its name, string value, and storage outlive the iterator.
        let values = unsafe { NamedValues::from_ptr(attributes.as_ptr()) };
        let cargo = values
            .find(cargo_name)
            .expect("cargo attribute should exist");

        assert_eq!(cargo.value().as_c_str(), Some(cargo_value));
        assert_eq!(cargo.index(), None);
    }

    #[test]
    fn indexed_attribute_lookup_uses_the_strong_sdk_index_domain() {
        let slot = c"slot.gear";
        let attributes = [
            sys::ScsNamedValue {
                name: slot.as_ptr(),
                index: 2,
                padding: sys::ScsPadding::uninit(),
                value: sys::ScsValue {
                    type_: sys::SCS_VALUE_TYPE_S32,
                    padding: sys::ScsPadding::uninit(),
                    value: sys::ScsValueData {
                        value_s32: sys::ScsValueS32 { value: 7 },
                    },
                },
            },
            sys::ScsNamedValue {
                name: core::ptr::null(),
                index: 0,
                padding: sys::ScsPadding::uninit(),
                value: sys::ScsValue {
                    type_: sys::SCS_VALUE_TYPE_INVALID,
                    padding: sys::ScsPadding::uninit(),
                    value: sys::ScsValueData {
                        value_u64: sys::ScsValueU64 { value: 0 },
                    },
                },
            },
        ];

        // SAFETY: `attributes` is contiguous, ends with a null-name entry, and
        // its initialized signed-32 value remains live during iteration.
        let values = unsafe { NamedValues::from_ptr(attributes.as_ptr()) };
        let index = SdkIndex::new(2).expect("ordinary array index");
        assert_eq!(
            values
                .clone()
                .get_at(crate::configuration::attributes::SLOT_GEAR, index),
            Some(7)
        );
        assert!(values.find(slot).is_none());
        assert_eq!(SdkIndex::new(sys::SCS_U32_NIL), None);
    }

    #[test]
    fn trailer_configuration_ids_require_the_canonical_numbered_form() {
        let terminator = [sys::ScsNamedValue {
            name: core::ptr::null(),
            index: 0,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValue {
                type_: sys::SCS_VALUE_TYPE_INVALID,
                padding: sys::ScsPadding::uninit(),
                value: sys::ScsValueData {
                    value_u64: sys::ScsValueU64 { value: 0 },
                },
            },
        }];

        for (id, expected) in [
            (c"trailer", Some(TrailerConfigurationId::Legacy)),
            (
                c"trailer.0",
                Some(TrailerConfigurationId::Numbered(TrailerIndex::ZERO)),
            ),
            (
                c"trailer.9",
                Some(TrailerConfigurationId::Numbered(
                    TrailerIndex::new(9).expect("last trailer index"),
                )),
            ),
            (c"trailer.00", None),
            (c"trailer.01", None),
            (c"trailer.10", None),
            (c"trailer.-1", None),
            (c"trailer.foo", None),
            (c"trailer.", None),
            (c"truck", None),
        ] {
            let raw = sys::ScsTelemetryConfiguration {
                id: id.as_ptr(),
                attributes: terminator.as_ptr(),
            };
            let event_info = (&raw const raw).cast::<c_void>();
            // SAFETY: `raw` is aligned and initialized for this iteration; its
            // ID and terminated attribute array outlive the borrowed event.
            let event = unsafe { ConfigurationRef::from_event_info(event_info) }
                .expect("configuration fixture");
            assert_eq!(event.trailer(), expected, "configuration id {id:?}");
            assert_eq!(
                event.trailer_index(),
                expected.and_then(TrailerConfigurationId::index)
            );
            assert_eq!(
                event.is_legacy_trailer(),
                expected.is_some_and(TrailerConfigurationId::is_legacy)
            );
        }
    }

    #[test]
    fn frame_start_ignores_uninitialized_alignment_storage() {
        let frame = sys::ScsTelemetryFrameStart {
            flags: sys::SCS_TELEMETRY_FRAME_START_FLAG_TIMER_RESTART,
            padding: sys::ScsPadding::uninit(),
            render_time: 11,
            simulation_time: 12,
            paused_simulation_time: 13,
        };
        let pointer = (&raw const frame).cast::<c_void>();
        // SAFETY: `pointer` addresses the live, aligned frame fixture. The
        // accessors read only initialized fields and deliberately skip padding.
        let frame = unsafe { FrameStartRef::from_event_info(pointer) }.expect("frame start");

        assert!(frame.timer_restarted());
        assert_eq!(frame.render_time(), 11);
        assert_eq!(frame.simulation_time(), 12);
        assert_eq!(frame.paused_simulation_time(), 13);
    }
}
