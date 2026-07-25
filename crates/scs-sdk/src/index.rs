//! Strong index domains used by telemetry descriptors and callbacks.
//!
//! SCS uses the same raw `scs_u32_t` representation for two unrelated ideas:
//! an SDK array index supplied beside a channel or named value, and the trailer
//! number embedded in a `trailer.[index].*` name. Keeping both as `u32` lets
//! otherwise safe application code silently swap them. These newtypes preserve
//! the distinct domains while remaining zero-cost at the FFI boundary.

use core::fmt;

use crate::sys;

/// Zero-based index passed through the SDK's explicit `index` field.
///
/// This selects one member of an indexed channel or configuration attribute,
/// such as a wheel, H-shifter slot, or substance. The raw value
/// [`sys::SCS_U32_NIL`] is reserved by SCS to represent a scalar descriptor and
/// therefore has no `SdkIndex` representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SdkIndex(u32);

impl SdkIndex {
    /// First zero-based SDK array element.
    pub const ZERO: Self = Self(0);

    /// Creates an SDK index unless `raw` is the scalar sentinel.
    #[must_use]
    pub const fn new(raw: u32) -> Option<Self> {
        if raw == sys::SCS_U32_NIL {
            None
        } else {
            Some(Self(raw))
        }
    }

    /// Returns the raw non-sentinel value used by the SCS ABI.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SdkIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Zero-based trailer number embedded in a multi-trailer name.
///
/// SCS SDK 1.14 defines ten numbered trailer configurations and channel
/// namespaces, `trailer.0` through `trailer.9`. Construction validates that
/// fixed header boundary once, so framework APIs accepting this type do not
/// repeat range checks or expose an invalid trailer identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TrailerIndex(u32);

impl TrailerIndex {
    /// Number of trailer slots declared by the official SDK header.
    pub const COUNT: usize = sys::configuration::MAX_TRAILERS;

    /// First trailer in a chain.
    pub const ZERO: Self = Self(0);

    /// Every valid trailer index in canonical numeric order.
    pub const ALL: [Self; Self::COUNT] = [
        Self(0),
        Self(1),
        Self(2),
        Self(3),
        Self(4),
        Self(5),
        Self(6),
        Self(7),
        Self(8),
        Self(9),
    ];

    /// Creates a trailer index when `raw` is within the SDK 1.14 range.
    #[must_use]
    pub fn new(raw: u32) -> Option<Self> {
        // The official count is small and fixed by the vendored header. Using
        // the raw ABI constant as the comparison source prevents the wrapper
        // and sys inventories from acquiring separate trailer limits.
        match usize::try_from(raw) {
            Ok(index) if index < Self::COUNT => Some(Self(raw)),
            Ok(_) | Err(_) => None,
        }
    }

    /// Returns the number embedded after the `trailer.` prefix.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Display for TrailerIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Identity of a trailer configuration callback.
///
/// SCS emits both the legacy unnumbered `trailer` configuration and the newer
/// numbered `trailer.0` through `trailer.9` configurations. They describe
/// related payloads but are not interchangeable: the numbered namespace has a
/// later game-schema requirement and `trailer` remains a compatibility alias
/// for the first trailer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrailerConfigurationId {
    /// Backward-compatible unnumbered `trailer` configuration.
    Legacy,
    /// One canonical numbered `trailer.[index]` configuration.
    Numbered(TrailerIndex),
}

impl TrailerConfigurationId {
    /// Returns the numbered trailer index, or `None` for the legacy alias.
    #[must_use]
    pub const fn index(self) -> Option<TrailerIndex> {
        match self {
            Self::Legacy => None,
            Self::Numbered(index) => Some(index),
        }
    }

    /// Whether this identity is the legacy unnumbered compatibility alias.
    #[must_use]
    pub const fn is_legacy(self) -> bool {
        matches!(self, Self::Legacy)
    }
}

const _: [(); TrailerIndex::COUNT] = [(); sys::configuration::MAX_TRAILERS];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_index_excludes_only_the_scalar_sentinel() {
        assert_eq!(SdkIndex::new(0), Some(SdkIndex::ZERO));
        assert_eq!(SdkIndex::new(42).map(SdkIndex::raw), Some(42));
        assert_eq!(SdkIndex::new(sys::SCS_U32_NIL), None);
    }

    #[test]
    fn trailer_indices_match_the_official_header_range() {
        assert_eq!(TrailerIndex::COUNT, 10);
        assert_eq!(TrailerIndex::COUNT, sys::configuration::MAX_TRAILERS);
        for (expected, index) in (0_u32..10).zip(TrailerIndex::ALL) {
            assert_eq!(index.raw(), expected);
            assert_eq!(TrailerIndex::new(expected), Some(index));
        }
        assert_eq!(TrailerIndex::new(10), None);
        assert_eq!(TrailerIndex::new(u32::MAX), None);
    }

    #[test]
    fn trailer_configuration_identity_keeps_legacy_separate() {
        assert!(TrailerConfigurationId::Legacy.is_legacy());
        assert_eq!(TrailerConfigurationId::Legacy.index(), None);
        assert_eq!(
            TrailerConfigurationId::Numbered(TrailerIndex::ZERO).index(),
            Some(TrailerIndex::ZERO)
        );
        assert!(!TrailerConfigurationId::Numbered(TrailerIndex::ZERO).is_legacy());
    }
}
