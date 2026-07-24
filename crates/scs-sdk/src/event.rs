use core::ffi::{CStr, c_void};
use core::marker::PhantomData;

use crate::{Attribute, ConfigurationId, GameplayEventId, SdkValue, ValueRef, sys};

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
    pub const fn index(self) -> Option<u32> {
        if self.raw.index == sys::SCS_U32_NIL {
            None
        } else {
            Some(self.raw.index)
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
    pub fn find_at(self, name: &CStr, index: u32) -> Option<NamedValueRef<'a>> {
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
        index: u32,
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
        let raw = unsafe { self.current.as_ref() }?;
        if raw.name.is_null() {
            return None;
        }

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

        let values = unsafe { NamedValues::from_ptr(attributes.as_ptr()) };
        let cargo = values
            .find(cargo_name)
            .expect("cargo attribute should exist");

        assert_eq!(cargo.value().as_c_str(), Some(cargo_value));
        assert_eq!(cargo.index(), None);
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
        let frame = unsafe { FrameStartRef::from_event_info(pointer) }.expect("frame start");

        assert!(frame.timer_restarted());
        assert_eq!(frame.render_time(), 11);
        assert_eq!(frame.simulation_time(), 12);
        assert_eq!(frame.paused_simulation_time(), 13);
    }
}
