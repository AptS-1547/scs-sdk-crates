//! Safe application framework for SCS Telemetry SDK plugins.
//!
//! This crate owns the parts of an SCS plugin which inherently require unsafe
//! Rust: exported ABI symbols, raw callback trampolines, stable opaque context
//! addresses, tagged-union borrowing, lifecycle sequencing, registration
//! rollback, and panic containment. Application crates implement
//! [`TelemetryPlugin`] and export it with [`export_plugin!`] using ordinary safe
//! Rust only.
//!
//! # Layering
//!
//! - `scs-sdk-sys` mirrors the C ABI and header constants;
//! - `scs-sdk` supplies typed descriptors and callback-time borrowed views;
//! - this crate turns those pieces into a safe plugin lifecycle;
//! - `scs-sdk-plugin-macros` emits the two symbols discovered by the game.
//!
//! # Threading model
//!
//! The official SDK invokes initialization, telemetry callbacks, and shutdown
//! on the game main thread. The runtime nevertheless serializes its global
//! state with a mutex so Rust's process-wide statics remain sound and poisoned
//! locks can be recovered deterministically. Plugin implementations are
//! required to be [`Send`] because they are stored behind that synchronized
//! boundary; the framework does not itself move callbacks onto worker threads.

#![deny(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::unwrap_used
    )
)]

mod input;
mod input_runtime;
mod runtime;

use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::fmt;

use scs_sdk::{
    AnyChannel, Attribute, Channel, ChannelFlags, ChannelValue, ConfigurationId, ConfigurationRef,
    FrameStartRef, GameSchemaAvailability, GameSchemaVersion, GameplayEventId, GameplayEventRef,
    LogLevel, SdkCall, SdkError, SdkIndex, SdkValue, StringValue, TelemetryApiVersion,
    TrailerConfigurationId, TrailerIndex, ValueRef,
};

pub use input::{
    InputAxisValue, InputAxisValueError, InputDeviceId, InputDeviceSpec, InputDeviceType,
    InputEvent, InputEventFlags, InputEventRequest, InputGameCompatibility, InputGameInfo,
    InputIndex, InputPlugin, InputPluginCompatibility, InputPluginContext, InputSpec, InputValue,
    InputValueType,
};
/// Typed descriptor and value layer used when implementing plugin hooks.
///
/// Re-exporting the middle layer keeps application manifests dependent on the
/// framework crate alone while preserving its normal module organization.
pub use scs_sdk as sdk;
pub use scs_sdk_plugin_macros::{export_input_plugin, export_plugin};

/// Application-facing name for the canonical [`sdk::Event`] descriptor.
///
/// This is a re-export rather than a framework-owned mirror enum. Event
/// identifiers and their minimum Telemetry API versions belong to the typed
/// SDK layer; the plugin framework only records explicit subscriptions and
/// turns the corresponding callbacks into [`TelemetryEvent`] payloads.
pub use scs_sdk::Event as TelemetryEventKind;

/// Result type returned by plugin initialization and subscription helpers.
pub type PluginResult<T = ()> = Result<T, PluginError>;

/// Tests one negotiated Telemetry API against a minimum within the same major
/// compatibility family.
///
/// This policy belongs to the framework rather than the version newtype: the
/// typed SDK preserves raw future versions, while the plugin runtime decides
/// when a product or requested capability may use them.
pub(crate) const fn telemetry_api_satisfies(
    actual: TelemetryApiVersion,
    minimum: TelemetryApiVersion,
) -> bool {
    actual.major() == minimum.major() && actual.raw() >= minimum.raw()
}

/// Tests an actual game schema against a descriptor minimum within one major.
pub(crate) const fn game_schema_satisfies(
    actual: GameSchemaVersion,
    minimum: GameSchemaVersion,
) -> bool {
    actual.major() == minimum.major() && actual.raw() >= minimum.raw()
}

/// An initialization failure with both an SDK result code and human-readable
/// context suitable for the game log.
///
/// The SDK ABI only returns a numeric `scs_result_t`. Retaining a message here
/// lets the runtime log the actual failing subscription or plugin invariant
/// before converting the error back to that numeric code.
#[derive(Debug)]
pub struct PluginError {
    result: SdkError,
    message: String,
}

impl PluginError {
    /// Creates a framework error which maps to `result` at the ABI boundary.
    #[must_use]
    pub fn new(result: SdkError, message: impl Into<String>) -> Self {
        Self {
            result,
            message: message.into(),
        }
    }

    /// Returns the SDK error ultimately reported to the game.
    #[must_use]
    pub const fn result(&self) -> SdkError {
        self.result
    }

    /// Returns the explanatory text logged before initialization is rolled back.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<SdkError> for PluginError {
    fn from(result: SdkError) -> Self {
        Self::new(result, result.to_string())
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PluginError {}

/// Recognized games which implement the SCS Telemetry SDK.
///
/// Unknown identifiers are retained in [`GameInfo::id`] even when this enum is
/// [`Game::Other`], allowing future SCS titles and third-party fixtures to be
/// diagnosed without exposing their C-string representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    EuroTruckSimulator2,
    AmericanTruckSimulator,
    Other,
}

pub(crate) fn classify_game_id(id: &CStr) -> Game {
    // Compare the exact NUL-terminated identifiers declared by the raw SDK
    // layer. Keeping the official bytes there avoids independent handwritten
    // `eut2`/`ats` catalogs in telemetry and input framework code.
    let id_bytes = id.to_bytes_with_nul();
    if id_bytes == scs_sdk::sys::SCS_GAME_ID_EUT2 {
        Game::EuroTruckSimulator2
    } else if id_bytes == scs_sdk::sys::SCS_GAME_ID_ATS {
        Game::AmericanTruckSimulator
    } else {
        Game::Other
    }
}

/// Owned identity of the game which loaded the plugin.
///
/// SCS only promises that the original C strings remain live during the
/// initialization call. The framework copies them so every later callback can
/// inspect the same metadata without extending a foreign pointer's lifetime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameInfo {
    name: String,
    id: String,
    kind: Game,
    schema_version: GameSchemaVersion,
}

impl GameInfo {
    pub(crate) fn new(name: &CStr, id: &CStr, schema_version: GameSchemaVersion) -> Self {
        Self {
            name: name.to_string_lossy().into_owned(),
            id: id.to_string_lossy().into_owned(),
            kind: classify_game_id(id),
            schema_version,
        }
    }

    /// Human-facing game name supplied by the SDK as Rust text.
    ///
    /// Invalid UTF-8 is replaced during initialization because the foreign
    /// allocation ceases to be valid when that call returns.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Stable textual identifier such as `eut2` or `ats`.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Typed classification of the game identifier.
    #[must_use]
    pub const fn kind(&self) -> Game {
        self.kind
    }

    /// Game-specific telemetry schema supplied by the SDK.
    ///
    /// This does not represent the telemetry API ABI or the public game patch.
    #[must_use]
    pub const fn schema_version(&self) -> GameSchemaVersion {
        self.schema_version
    }

    /// Resolves one SDK descriptor, association, capability, or value history
    /// entry for the detected game.
    ///
    /// Unknown game IDs return `None`: ETS2 and ATS use independent schema
    /// histories, so applying either known game's minimum to a third game would
    /// silently invent compatibility evidence.
    #[must_use]
    pub const fn minimum_schema_for(
        &self,
        availability: GameSchemaAvailability,
    ) -> Option<GameSchemaVersion> {
        match self.kind {
            Game::EuroTruckSimulator2 => availability.available_since_ets2(),
            Game::AmericanTruckSimulator => availability.available_since_ats(),
            Game::Other => None,
        }
    }

    /// Whether the detected game schema satisfies one official availability
    /// record within the same schema-major family.
    ///
    /// This is the same policy used by runtime registration preflight. It is
    /// public so application diagnostics and schema-driven UI can query channel,
    /// association, and enum-value history without reimplementing version or
    /// game-kind matching.
    #[must_use]
    pub const fn supports(&self, availability: GameSchemaAvailability) -> bool {
        match self.minimum_schema_for(availability) {
            Some(minimum) => game_schema_satisfies(self.schema_version, minimum),
            None => false,
        }
    }
}

/// Minimum telemetry schema required for one supported SCS game.
///
/// A schema major identifies the compatibility family. Minor versions are
/// additive according to the SDK contract, so a plugin accepts the declared
/// minimum and later minor versions within the same major. A different major
/// is rejected until the plugin explicitly updates this declaration after
/// reviewing the changed semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameCompatibility {
    game: Game,
    minimum_schema: GameSchemaVersion,
}

impl GameCompatibility {
    /// Declares support for one recognized game and its minimum schema.
    ///
    /// Use one entry per game. The runtime rejects duplicate entries and the
    /// broad `Game::Other` classification because neither would identify one
    /// unambiguous compatibility policy.
    #[must_use]
    pub const fn new(game: Game, minimum_schema: GameSchemaVersion) -> Self {
        Self {
            game,
            minimum_schema,
        }
    }

    /// Recognized game governed by this declaration.
    #[must_use]
    pub const fn game(self) -> Game {
        self.game
    }

    /// Oldest telemetry schema accepted for this game.
    #[must_use]
    pub const fn minimum_schema(self) -> GameSchemaVersion {
        self.minimum_schema
    }
}

/// Explicit runtime requirements declared by one product plugin.
///
/// The framework and product answer different compatibility questions:
///
/// - scs-sdk decides which foreign API layouts it can decode soundly;
/// - this declaration states which decoded API and game capabilities the
///   product actually needs;
/// - SCS chooses the concrete API version by calling the exported initializer
///   from newest to oldest.
///
/// Ordinary users therefore install one plugin binary and never select an SDK
/// distribution or ABI version manually.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginCompatibility {
    minimum_telemetry_api: TelemetryApiVersion,
    games: &'static [GameCompatibility],
}

impl PluginCompatibility {
    /// Creates an explicit product compatibility declaration.
    ///
    /// The games slice must contain at least one recognized game, must not
    /// contain duplicate game entries, and must not use `Game::Other`. These
    /// invariants are checked before product initialization so a malformed
    /// declaration never reaches SDK registration.
    #[must_use]
    pub const fn new(
        minimum_telemetry_api: TelemetryApiVersion,
        games: &'static [GameCompatibility],
    ) -> Self {
        Self {
            minimum_telemetry_api,
            games,
        }
    }

    /// Oldest Telemetry API whose capabilities the product requires.
    #[must_use]
    pub const fn minimum_telemetry_api(self) -> TelemetryApiVersion {
        self.minimum_telemetry_api
    }

    /// Per-game schema requirements accepted by the product.
    #[must_use]
    pub const fn games(self) -> &'static [GameCompatibility] {
        self.games
    }
}

/// Whether absence of one channel invalidates the complete plugin transaction.
///
/// This remains internal because application code expresses the policy through
/// the explicit `subscribe*` and `subscribe_optional*` method families rather
/// than constructing an abstract options object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SubscriptionRequirement {
    Required,
    Optional,
}

impl SubscriptionRequirement {
    pub(crate) const fn tolerates_channel_registration_error(self, error: SdkError) -> bool {
        matches!(self, Self::Optional)
            && matches!(error, SdkError::NotFound | SdkError::UnsupportedType)
    }

    pub(crate) const fn tolerates_event_registration_error(self, error: SdkError) -> bool {
        matches!(self, Self::Optional)
            && matches!(error, SdkError::Unsupported | SdkError::NotFound)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EventSubscriptionSpec {
    pub(crate) event: TelemetryEventKind,
    pub(crate) requirement: SubscriptionRequirement,
}

#[derive(Clone, Debug)]
pub(crate) struct SubscriptionSpec {
    pub(crate) channel: AnyChannel,
    pub(crate) registered_name: CString,
    pub(crate) sdk_index: Option<SdkIndex>,
    pub(crate) trailer_index: Option<TrailerIndex>,
    pub(crate) flags: ChannelFlags,
    pub(crate) requirement: SubscriptionRequirement,
}

/// Safe capabilities available while the framework calls plugin code.
///
/// Logging is valid in every hook. Channel registration is intentionally open
/// only during [`TelemetryPlugin::initialize`]; calling a subscription method
/// from another hook returns [`SdkError::NotNow`] without touching the SDK.
pub struct PluginContext<'scope> {
    call: &'scope SdkCall<'scope>,
    game: GameInfo,
    events: Option<&'scope mut Vec<EventSubscriptionSpec>>,
    subscriptions: Option<&'scope mut Vec<SubscriptionSpec>>,
}

impl<'scope> PluginContext<'scope> {
    pub(crate) fn initializing(
        call: &'scope SdkCall<'scope>,
        game: GameInfo,
        events: &'scope mut Vec<EventSubscriptionSpec>,
        subscriptions: &'scope mut Vec<SubscriptionSpec>,
    ) -> Self {
        Self {
            call,
            game,
            events: Some(events),
            subscriptions: Some(subscriptions),
        }
    }

    pub(crate) fn callback(call: &'scope SdkCall<'scope>, game: GameInfo) -> Self {
        Self {
            call,
            game,
            events: None,
            subscriptions: None,
        }
    }

    /// Metadata copied from the game's initialization parameters.
    #[must_use]
    pub const fn game(&self) -> &GameInfo {
        &self.game
    }

    /// Telemetry API version selected by the SCS loader for this session.
    ///
    /// This is the negotiated runtime ABI, not the SDK archive version and not
    /// the game-specific telemetry schema returned by `PluginContext::game`.
    /// Product code normally relies on `TelemetryPlugin::compatibility` for
    /// mandatory requirements and reads this value only for an explicit
    /// optional capability branch.
    #[must_use]
    pub const fn telemetry_api_version(&self) -> TelemetryApiVersion {
        self.call.telemetry_api_version()
    }

    /// Formats and writes one message to the game log.
    ///
    /// Interior NUL characters have no representation in a C string. They are
    /// replaced with spaces so malformed external text cannot suppress the
    /// entire log entry or force application code to handle an FFI detail.
    pub fn log(&self, level: LogLevel, arguments: fmt::Arguments<'_>) {
        let rendered = format!("{arguments}").replace('\0', " ");
        if let Ok(message) = CString::new(rendered) {
            self.call.logger().log(level, &message);
        }
    }

    /// Logs an informational message.
    pub fn message(&self, arguments: fmt::Arguments<'_>) {
        self.log(LogLevel::Message, arguments);
    }

    /// Logs a warning message.
    pub fn warning(&self, arguments: fmt::Arguments<'_>) {
        self.log(LogLevel::Warning, arguments);
    }

    /// Logs an error message.
    pub fn error(&self, arguments: fmt::Arguments<'_>) {
        self.log(LogLevel::Error, arguments);
    }

    /// Requests delivery of one telemetry event kind.
    ///
    /// Event registration is explicit: the framework never infers it from the
    /// presence of [`TelemetryPlugin::event`] and does not subscribe to unused
    /// frame, configuration, or gameplay callbacks. Registration with SCS is
    /// deferred until [`TelemetryPlugin::initialize`] returns successfully so
    /// the complete set can be rolled back transactionally.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::AlreadyRegistered`] when the same event kind was
    /// requested twice, or [`SdkError::NotNow`] when called from a callback or
    /// shutdown hook instead of initialization. A required event introduced by
    /// a newer game schema returns [`SdkError::Unsupported`] before foreign
    /// registration, independently from its Telemetry API requirement.
    pub fn subscribe_event(&mut self, event: TelemetryEventKind) -> PluginResult {
        self.subscribe_event_with_requirement(event, SubscriptionRequirement::Required)
    }

    /// Requests delivery of an event when the negotiated API and loading game
    /// provide it.
    ///
    /// An event introduced by a newer API is skipped locally. If SCS reports
    /// `Unsupported` or `NotFound` while registering it, the remaining required
    /// transaction continues. Duplicate declarations, wrong lifecycle phase,
    /// and other SDK failures remain errors.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::AlreadyRegistered`] for a duplicate event or
    /// [`SdkError::NotNow`] outside product initialization. A non-capability
    /// SDK registration error later aborts the complete transaction.
    pub fn subscribe_event_optional(&mut self, event: TelemetryEventKind) -> PluginResult {
        self.subscribe_event_with_requirement(event, SubscriptionRequirement::Optional)
    }

    fn subscribe_event_with_requirement(
        &mut self,
        event: TelemetryEventKind,
        requirement: SubscriptionRequirement,
    ) -> PluginResult {
        let api_version = self.telemetry_api_version();
        let Some(events) = self.events.as_deref_mut() else {
            return Err(PluginError::new(
                SdkError::NotNow,
                "events may only be subscribed during plugin initialization",
            ));
        };
        let minimum_api = event.minimum_api_version();
        if !telemetry_api_satisfies(api_version, minimum_api)
            && requirement == SubscriptionRequirement::Required
        {
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!(
                    "event {event:?} requires telemetry API {minimum_api}, negotiated {api_version}"
                ),
            ));
        }
        let minimum_schema = self.game.minimum_schema_for(event.availability());
        let schema_supported = self.game.supports(event.availability());
        if !schema_supported && requirement == SubscriptionRequirement::Required {
            let detail = minimum_schema.map_or_else(
                || format!("is not available for {:?}", self.game.kind()),
                |minimum| {
                    format!(
                        "requires {:?} telemetry schema {minimum}; detected {}",
                        self.game.kind(),
                        self.game.schema_version(),
                    )
                },
            );
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!("event {event:?} {detail}"),
            ));
        }
        if events.iter().any(|candidate| candidate.event == event) {
            return Err(PluginError::new(
                SdkError::AlreadyRegistered,
                format!("duplicate event subscription for {event:?}"),
            ));
        }
        events.push(EventSubscriptionSpec { event, requirement });
        Ok(())
    }

    /// Subscribes to one scalar channel using its canonical SDK name.
    ///
    /// Values are delivered only when they change unless `flags` includes
    /// [`ChannelFlags::EACH_FRAME`]. For an indexed channel, use
    /// [`PluginContext::subscribe_at`].
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::InvalidParameter`] when `channel` is indexed,
    /// [`SdkError::AlreadyRegistered`] for a duplicate subscription, or
    /// [`SdkError::NotNow`] outside initialization. A requested value
    /// representation introduced after the negotiated Telemetry API returns
    /// [`SdkError::Unsupported`] before the SDK registration transaction begins.
    /// The same result is returned when the built-in descriptor postdates the
    /// loading game's telemetry schema.
    pub fn subscribe<T: ChannelValue>(&mut self, channel: Channel<T>) -> PluginResult {
        self.subscribe_with_flags(channel, ChannelFlags::NONE)
    }

    /// Subscribes to one scalar channel with explicit SDK delivery flags.
    ///
    /// # Errors
    ///
    /// Uses the same validation as [`PluginContext::subscribe`].
    pub fn subscribe_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        flags: ChannelFlags,
    ) -> PluginResult {
        self.subscribe_scalar_with_flags(channel, flags, SubscriptionRequirement::Required)
    }

    /// Requests a scalar channel when available in the loading game.
    ///
    /// Unlike [`PluginContext::subscribe`], absence of the channel or an
    /// unsupported channel-specific conversion does not abort initialization.
    /// No callback is delivered for a skipped subscription, so product state
    /// must retain an explicit default or unavailable representation.
    ///
    /// API- and game-schema-level capability checks are also optional: for
    /// example, an `i64` request is skipped under Telemetry API 1.00, and a
    /// navigation channel is skipped before ETS2 schema 1.12.
    /// Invalid descriptor shape, duplicate declarations, and lifecycle misuse
    /// remain errors because they indicate a malformed plugin declaration.
    ///
    /// # Errors
    ///
    /// Returns the same descriptor-shape, duplicate, and lifecycle declaration
    /// errors as [`PluginContext::subscribe`]. Non-capability registration
    /// failures still abort the complete transaction.
    pub fn subscribe_optional<T: ChannelValue>(&mut self, channel: Channel<T>) -> PluginResult {
        self.subscribe_optional_with_flags(channel, ChannelFlags::NONE)
    }

    /// Requests an optional scalar channel with explicit delivery flags.
    ///
    /// # Errors
    ///
    /// Uses the declaration validation documented by
    /// [`PluginContext::subscribe_optional`].
    pub fn subscribe_optional_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        flags: ChannelFlags,
    ) -> PluginResult {
        self.subscribe_scalar_with_flags(channel, flags, SubscriptionRequirement::Optional)
    }

    fn subscribe_scalar_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        flags: ChannelFlags,
        requirement: SubscriptionRequirement,
    ) -> PluginResult {
        if channel.is_indexed() {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                format!("indexed channel {:?} requires subscribe_at", channel.name()),
            ));
        }
        self.push_subscription(
            channel.erase(),
            channel.name().to_owned(),
            None,
            None,
            flags,
            requirement,
        )
    }

    /// Subscribes to one member of an SDK-indexed channel.
    ///
    /// The index selects an element such as one truck wheel. It is independent
    /// from the trailer number embedded by [`PluginContext::subscribe_trailer_at`].
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::InvalidParameter`] for a scalar descriptor and the
    /// normal phase/duplicate errors documented by [`PluginContext::subscribe`].
    pub fn subscribe_at<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        index: SdkIndex,
    ) -> PluginResult {
        self.subscribe_at_with_flags(channel, index, ChannelFlags::NONE)
    }

    /// Subscribes to an indexed channel with explicit SDK delivery flags.
    ///
    /// # Errors
    ///
    /// Uses the same validation as [`PluginContext::subscribe_at`].
    pub fn subscribe_at_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        index: SdkIndex,
        flags: ChannelFlags,
    ) -> PluginResult {
        self.subscribe_at_with_flags_and_requirement(
            channel,
            index,
            flags,
            SubscriptionRequirement::Required,
        )
    }

    /// Requests one indexed channel member when available.
    ///
    /// `NotFound` and `UnsupportedType` from SCS skip this member without
    /// affecting required registrations. Descriptor-shape, duplicate, index,
    /// and lifecycle validation remain strict.
    ///
    /// # Errors
    ///
    /// Returns the same declaration errors as [`PluginContext::subscribe_at`].
    /// Non-capability registration failures still abort the transaction.
    pub fn subscribe_at_optional<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        index: SdkIndex,
    ) -> PluginResult {
        self.subscribe_at_optional_with_flags(channel, index, ChannelFlags::NONE)
    }

    /// Requests an optional indexed channel member with delivery flags.
    ///
    /// # Errors
    ///
    /// Uses the declaration validation documented by
    /// [`PluginContext::subscribe_at_optional`].
    pub fn subscribe_at_optional_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        index: SdkIndex,
        flags: ChannelFlags,
    ) -> PluginResult {
        self.subscribe_at_with_flags_and_requirement(
            channel,
            index,
            flags,
            SubscriptionRequirement::Optional,
        )
    }

    fn subscribe_at_with_flags_and_requirement<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        index: SdkIndex,
        flags: ChannelFlags,
        requirement: SubscriptionRequirement,
    ) -> PluginResult {
        if !channel.is_indexed() {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                format!(
                    "scalar channel {:?} does not accept an SDK index",
                    channel.name()
                ),
            ));
        }
        self.push_subscription(
            channel.erase(),
            channel.name().to_owned(),
            Some(index),
            None,
            flags,
            requirement,
        )
    }

    /// Subscribes to one scalar channel for an explicit trailer in a chain.
    ///
    /// The official static macros use `trailer.*`, while multi-trailer
    /// telemetry is named `trailer.0.*`, `trailer.1.*`, and so on. This helper
    /// performs that transformation and retains the generated C string for the
    /// complete SDK registration lifetime.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::InvalidParameter`] when the descriptor is not a
    /// scalar `trailer.*` channel. [`TrailerIndex`] construction already
    /// enforces the official `0..10` range. The numbered namespace itself
    /// requires ETS2 schema 1.14 or ATS schema 1.01, even when the underlying
    /// backward-compatible `trailer.*` descriptor is older.
    pub fn subscribe_trailer<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
    ) -> PluginResult {
        self.subscribe_trailer_with_flags(channel, trailer_index, ChannelFlags::NONE)
    }

    /// Subscribes to a scalar multi-trailer channel with explicit delivery flags.
    ///
    /// # Errors
    ///
    /// Uses the same validation as [`PluginContext::subscribe_trailer`].
    pub fn subscribe_trailer_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
        flags: ChannelFlags,
    ) -> PluginResult {
        self.subscribe_trailer_with_flags_and_requirement(
            channel,
            trailer_index,
            flags,
            SubscriptionRequirement::Required,
        )
    }

    /// Requests one scalar trailer channel when available.
    ///
    /// The trailer name is still validated eagerly; the strong index is valid
    /// by construction. Only SCS `NotFound` and `UnsupportedType` registration
    /// results, or a value representation newer than the negotiated API, are
    /// treated as expected absence.
    ///
    /// # Errors
    ///
    /// Returns the same descriptor, duplicate, and lifecycle declaration errors
    /// as [`PluginContext::subscribe_trailer`].
    pub fn subscribe_trailer_optional<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
    ) -> PluginResult {
        self.subscribe_trailer_optional_with_flags(channel, trailer_index, ChannelFlags::NONE)
    }

    /// Requests an optional scalar trailer channel with delivery flags.
    ///
    /// # Errors
    ///
    /// Uses the declaration validation documented by
    /// [`PluginContext::subscribe_trailer_optional`].
    pub fn subscribe_trailer_optional_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
        flags: ChannelFlags,
    ) -> PluginResult {
        self.subscribe_trailer_with_flags_and_requirement(
            channel,
            trailer_index,
            flags,
            SubscriptionRequirement::Optional,
        )
    }

    fn subscribe_trailer_with_flags_and_requirement<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
        flags: ChannelFlags,
        requirement: SubscriptionRequirement,
    ) -> PluginResult {
        if channel.is_indexed() {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                "indexed trailer channels require subscribe_trailer_at",
            ));
        }
        let name = trailer_channel_name(channel, trailer_index)?;
        self.push_subscription(
            channel.erase(),
            name,
            None,
            Some(trailer_index),
            flags,
            requirement,
        )
    }

    /// Subscribes to one SDK-indexed member of an explicit trailer channel.
    ///
    /// For example, trailer index 1 and SDK index 2 select wheel 2 of the second
    /// trailer. Both strong index domains are zero-based.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::InvalidParameter`] unless `channel` is an indexed
    /// `trailer.*` descriptor. The trailer range and scalar sentinel are
    /// excluded while constructing [`TrailerIndex`] and [`SdkIndex`].
    pub fn subscribe_trailer_at<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
        index: SdkIndex,
    ) -> PluginResult {
        self.subscribe_trailer_at_with_flags(channel, trailer_index, index, ChannelFlags::NONE)
    }

    /// Subscribes to an indexed multi-trailer channel with delivery flags.
    ///
    /// # Errors
    ///
    /// Uses the same validation as [`PluginContext::subscribe_trailer_at`].
    pub fn subscribe_trailer_at_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
        index: SdkIndex,
        flags: ChannelFlags,
    ) -> PluginResult {
        self.subscribe_trailer_at_with_flags_and_requirement(
            channel,
            trailer_index,
            index,
            flags,
            SubscriptionRequirement::Required,
        )
    }

    /// Requests an indexed member of one trailer channel when available.
    ///
    /// Both index domains remain explicit and strictly validated. Expected SDK
    /// capability absence skips only this declaration.
    ///
    /// # Errors
    ///
    /// Returns the same descriptor, index, duplicate, and lifecycle declaration
    /// errors as [`PluginContext::subscribe_trailer_at`].
    pub fn subscribe_trailer_at_optional<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
        index: SdkIndex,
    ) -> PluginResult {
        self.subscribe_trailer_at_optional_with_flags(
            channel,
            trailer_index,
            index,
            ChannelFlags::NONE,
        )
    }

    /// Requests an optional indexed trailer channel with delivery flags.
    ///
    /// # Errors
    ///
    /// Uses the declaration validation documented by
    /// [`PluginContext::subscribe_trailer_at_optional`].
    pub fn subscribe_trailer_at_optional_with_flags<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
        index: SdkIndex,
        flags: ChannelFlags,
    ) -> PluginResult {
        self.subscribe_trailer_at_with_flags_and_requirement(
            channel,
            trailer_index,
            index,
            flags,
            SubscriptionRequirement::Optional,
        )
    }

    fn subscribe_trailer_at_with_flags_and_requirement<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: TrailerIndex,
        index: SdkIndex,
        flags: ChannelFlags,
        requirement: SubscriptionRequirement,
    ) -> PluginResult {
        if !channel.is_indexed() {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                "scalar trailer channels do not accept an SDK index",
            ));
        }
        let name = trailer_channel_name(channel, trailer_index)?;
        self.push_subscription(
            channel.erase(),
            name,
            Some(index),
            Some(trailer_index),
            flags,
            requirement,
        )
    }

    fn push_subscription(
        &mut self,
        channel: AnyChannel,
        registered_name: CString,
        sdk_index: Option<SdkIndex>,
        trailer_index: Option<TrailerIndex>,
        flags: ChannelFlags,
        requirement: SubscriptionRequirement,
    ) -> PluginResult {
        let api_version = self.telemetry_api_version();
        let descriptor_minimum = self.game.minimum_schema_for(channel.availability());
        let descriptor_supported = self.game.supports(channel.availability());
        let trailer_minimum = trailer_index.and_then(|_| {
            self.game
                .minimum_schema_for(scs_sdk::game::capabilities::MULTI_TRAILER)
        });
        let trailer_supported = trailer_index.is_none()
            || self
                .game
                .supports(scs_sdk::game::capabilities::MULTI_TRAILER);
        let Some(subscriptions) = self.subscriptions.as_deref_mut() else {
            return Err(PluginError::new(
                SdkError::NotNow,
                "channels may only be subscribed during plugin initialization",
            ));
        };

        let value_type = channel.value_type();
        let minimum_api = value_type.minimum_api_version();
        if !telemetry_api_satisfies(api_version, minimum_api)
            && requirement == SubscriptionRequirement::Required
        {
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!(
                    "channel {registered_name:?} requests {value_type:?}, which requires telemetry API {minimum_api}; negotiated {api_version}",
                ),
            ));
        }
        if (!descriptor_supported || !trailer_supported)
            && requirement == SubscriptionRequirement::Required
        {
            let (capability, minimum) = if descriptor_supported {
                ("numbered multi-trailer namespace", trailer_minimum)
            } else {
                ("channel descriptor", descriptor_minimum)
            };
            let detail = minimum.map_or_else(
                || format!("is not available for {:?}", self.game.kind()),
                |minimum| {
                    format!(
                        "requires {:?} telemetry schema {minimum}; detected {}",
                        self.game.kind(),
                        self.game.schema_version(),
                    )
                },
            );
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!("{capability} {registered_name:?} {detail}"),
            ));
        }

        let duplicate = subscriptions.iter().any(|candidate| {
            candidate.registered_name == registered_name
                && candidate.sdk_index == sdk_index
                && candidate.channel.value_type() == channel.value_type()
        });
        if duplicate {
            return Err(PluginError::new(
                SdkError::AlreadyRegistered,
                format!("duplicate subscription for {registered_name:?}, index {sdk_index:?}"),
            ));
        }

        subscriptions.push(SubscriptionSpec {
            channel,
            registered_name,
            sdk_index,
            trailer_index,
            flags,
            requirement,
        });
        Ok(())
    }
}

fn trailer_channel_name<T: ChannelValue>(
    channel: Channel<T>,
    trailer_index: TrailerIndex,
) -> PluginResult<CString> {
    let name = channel.name().to_str().map_err(|error| {
        PluginError::new(
            SdkError::InvalidParameter,
            format!("channel name is not UTF-8: {error}"),
        )
    })?;
    let Some(suffix) = name.strip_prefix("trailer.") else {
        return Err(PluginError::new(
            SdkError::InvalidParameter,
            format!("channel {name:?} is not a trailer channel"),
        ));
    };
    CString::new(format!("trailer.{trailer_index}.{suffix}")).map_err(|error| {
        PluginError::new(
            SdkError::InvalidParameter,
            format!("generated trailer channel contains a NUL byte: {error}"),
        )
    })
}

/// One safely borrowed channel callback.
///
/// Numeric and geometry values may be copied out and retained. String values
/// borrow storage owned by the game and therefore remain tied to this update's
/// callback lifetime.
#[derive(Clone, Copy)]
pub struct ChannelUpdate<'a> {
    channel: AnyChannel,
    registered_name: &'a CStr,
    index: Option<SdkIndex>,
    trailer_index: Option<TrailerIndex>,
    flags: ChannelFlags,
    value: Option<ValueRef<'a>>,
}

impl<'a> ChannelUpdate<'a> {
    pub(crate) const fn new(
        channel: AnyChannel,
        registered_name: &'a CStr,
        index: Option<SdkIndex>,
        trailer_index: Option<TrailerIndex>,
        flags: ChannelFlags,
        value: Option<ValueRef<'a>>,
    ) -> Self {
        Self {
            channel,
            registered_name,
            index,
            trailer_index,
            flags,
            value,
        }
    }

    /// Type-erased canonical descriptor originally supplied by the plugin.
    #[must_use]
    pub const fn channel(self) -> AnyChannel {
        self.channel
    }

    /// Exact name registered with SCS, including a multi-trailer prefix.
    ///
    /// Official channel names are ASCII. The lossy return type also keeps this
    /// method total if a future SDK accepts a non-UTF-8 custom channel name.
    #[must_use]
    pub fn registered_name(self) -> Cow<'a, str> {
        self.registered_name.to_string_lossy()
    }

    /// Zero-based SDK array index, or `None` for a scalar channel.
    #[must_use]
    pub const fn index(self) -> Option<SdkIndex> {
        self.index
    }

    /// Zero-based trailer number embedded in the registered name.
    #[must_use]
    pub const fn trailer_index(self) -> Option<TrailerIndex> {
        self.trailer_index
    }

    /// Delivery flags used for this subscription.
    #[must_use]
    pub const fn flags(self) -> ChannelFlags {
        self.flags
    }

    /// Raw safe view of the tagged value, absent for `NO_VALUE` subscriptions.
    #[must_use]
    pub const fn value_ref(self) -> Option<ValueRef<'a>> {
        self.value
    }

    /// Tests whether this update belongs to a particular typed descriptor.
    #[must_use]
    pub fn is<T: ChannelValue>(self, channel: Channel<T>) -> bool {
        self.channel == channel.erase()
    }

    /// Decodes the update using the supplied typed descriptor.
    ///
    /// `None` means either the descriptor does not match this registration, the
    /// subscription requested `NO_VALUE`, or SCS supplied a different union tag.
    #[must_use]
    pub fn value<T: ChannelValue>(self, channel: Channel<T>) -> Option<T::Decoded<'a>> {
        if !self.is(channel) {
            return None;
        }
        channel.decode(self.value?)
    }
}

/// High-level view over one SDK configuration event.
///
/// This wrapper keeps the exact zero-allocation typed decoder in `scs-sdk`
/// while adding Rust-text accessors for product plugins. All borrowed data is
/// still restricted to the current callback.
#[derive(Clone, Copy)]
pub struct ConfigurationEvent<'a> {
    inner: ConfigurationRef<'a>,
}

impl<'a> ConfigurationEvent<'a> {
    pub(crate) const fn new(inner: ConfigurationRef<'a>) -> Self {
        Self { inner }
    }

    /// Tests the event against a typed SDK configuration identifier.
    #[must_use]
    pub fn is(self, id: ConfigurationId) -> bool {
        self.inner.is(id)
    }

    /// Classifies the legacy or numbered trailer configuration identity.
    #[must_use]
    pub fn trailer(self) -> Option<TrailerConfigurationId> {
        self.inner.trailer()
    }

    /// Returns the numbered trailer index, excluding the legacy alias.
    #[must_use]
    pub fn trailer_index(self) -> Option<TrailerIndex> {
        self.inner.trailer_index()
    }

    /// Whether this is the legacy unnumbered `trailer` configuration.
    #[must_use]
    pub fn is_legacy_trailer(self) -> bool {
        self.inner.is_legacy_trailer()
    }

    /// Configuration identifier as Rust text.
    #[must_use]
    pub fn id(self) -> Cow<'a, str> {
        self.inner.id().to_string_lossy()
    }

    /// Whether the event contains at least one attribute.
    ///
    /// SCS sends an empty job configuration when the active job disappears.
    #[must_use]
    pub fn has_attributes(self) -> bool {
        self.inner.attributes().next().is_some()
    }

    /// Decodes a scalar typed attribute.
    #[must_use]
    pub fn get<T: SdkValue>(self, attribute: Attribute<T>) -> Option<T::Decoded<'a>> {
        self.inner.attributes().get(attribute)
    }

    /// Decodes one member of an indexed typed attribute.
    #[must_use]
    pub fn get_at<T: SdkValue>(
        self,
        attribute: Attribute<T>,
        index: SdkIndex,
    ) -> Option<T::Decoded<'a>> {
        self.inner.attributes().get_at(attribute, index)
    }

    /// Decodes a string attribute as callback-lifetime Rust text.
    #[must_use]
    pub fn string(self, attribute: Attribute<StringValue>) -> Option<Cow<'a, str>> {
        self.get(attribute).map(CStr::to_string_lossy)
    }

    /// Copies a string attribute for storage beyond the current callback.
    #[must_use]
    pub fn string_owned(self, attribute: Attribute<StringValue>) -> Option<String> {
        self.string(attribute).map(Cow::into_owned)
    }

    /// Decodes the documented `controls` shifter type as a Rust enum.
    ///
    /// `None` covers an absent attribute and a future string which SDK 1.14 did
    /// not document. Call [`ConfigurationEvent::string`] with
    /// [`sdk::configuration::attributes::SHIFTER_TYPE`] when the original
    /// unknown text must be retained for forward-compatible diagnostics.
    #[must_use]
    pub fn shifter_type(self) -> Option<sdk::configuration::ShifterType> {
        let value = self.string(sdk::configuration::attributes::SHIFTER_TYPE)?;
        value.parse().ok()
    }

    /// Decodes the documented active-job market as a Rust enum.
    ///
    /// Unknown future values remain accessible through the generic string
    /// accessor instead of being reclassified as one of the known SDK 1.14
    /// variants.
    #[must_use]
    pub fn job_market(self) -> Option<sdk::configuration::JobMarket> {
        let value = self.string(sdk::configuration::attributes::JOB_MARKET)?;
        value.parse().ok()
    }
}

/// High-level view over one SDK gameplay event.
///
/// Gameplay payloads share the same typed attribute descriptors as the middle
/// layer but expose dedicated Rust-text helpers for string values.
#[derive(Clone, Copy)]
pub struct GameplayEvent<'a> {
    inner: GameplayEventRef<'a>,
}

impl<'a> GameplayEvent<'a> {
    pub(crate) const fn new(inner: GameplayEventRef<'a>) -> Self {
        Self { inner }
    }

    /// Tests the event against a typed SDK gameplay identifier.
    #[must_use]
    pub fn is(self, id: GameplayEventId) -> bool {
        self.inner.is(id)
    }

    /// Gameplay event identifier as Rust text.
    #[must_use]
    pub fn id(self) -> Cow<'a, str> {
        self.inner.id().to_string_lossy()
    }

    /// Decodes a scalar typed attribute.
    #[must_use]
    pub fn get<T: SdkValue>(self, attribute: Attribute<T>) -> Option<T::Decoded<'a>> {
        self.inner.attributes().get(attribute)
    }

    /// Decodes one member of an indexed typed attribute.
    #[must_use]
    pub fn get_at<T: SdkValue>(
        self,
        attribute: Attribute<T>,
        index: SdkIndex,
    ) -> Option<T::Decoded<'a>> {
        self.inner.attributes().get_at(attribute, index)
    }

    /// Decodes a string attribute as callback-lifetime Rust text.
    #[must_use]
    pub fn string(self, attribute: Attribute<StringValue>) -> Option<Cow<'a, str>> {
        self.get(attribute).map(CStr::to_string_lossy)
    }

    /// Copies a string attribute for storage beyond the current callback.
    #[must_use]
    pub fn string_owned(self, attribute: Attribute<StringValue>) -> Option<String> {
        self.string(attribute).map(Cow::into_owned)
    }

    /// Decodes a documented `player.fined` offence value as a Rust enum.
    ///
    /// An absent attribute and a future additive offence both return `None`.
    /// The generic string accessor preserves the original text when callers
    /// need to distinguish those cases.
    #[must_use]
    pub fn fine_offence(self) -> Option<sdk::gameplay::FineOffence> {
        let value = self.string(sdk::gameplay::attributes::FINE_OFFENCE)?;
        value.parse().ok()
    }
}

/// Safe event payload delivered to [`TelemetryPlugin::event`].
///
/// Pointer-bearing SDK structures are converted to lifetime-bound views before
/// application code runs. Events without a payload use unit variants.
#[derive(Clone, Copy)]
pub enum TelemetryEvent<'a> {
    FrameStart(FrameStartRef<'a>),
    FrameEnd,
    Paused,
    Started,
    Configuration(ConfigurationEvent<'a>),
    Gameplay(GameplayEvent<'a>),
}

/// Static identity reported by the framework during plugin lifecycle logging.
///
/// Metadata is supplied by the product instead of inferred from Rust type names
/// or Cargo package names. This keeps the user-facing identity explicit and
/// stable even when the implementation type, workspace layout, or crate name
/// changes independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginMetadata {
    name: &'static str,
    version: &'static str,
}

impl PluginMetadata {
    /// Creates product metadata from static build information.
    ///
    /// `env!("CARGO_PKG_VERSION")` is suitable for `version`; the product name
    /// should be the stable human-facing plugin identity shown in game logs.
    #[must_use]
    pub const fn new(name: &'static str, version: &'static str) -> Self {
        Self { name, version }
    }

    /// Stable human-facing plugin name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Product version embedded when the plugin was built.
    #[must_use]
    pub const fn version(self) -> &'static str {
        self.version
    }
}

/// Application-facing telemetry plugin lifecycle.
///
/// Every method is called synchronously on the SDK thread. The framework catches
/// unwinding panics at every ABI entry point; release builds in this workspace
/// additionally use `panic = "abort"`, matching a native game plugin's usual
/// failure policy.
pub trait TelemetryPlugin: Send + 'static {
    /// Declares the stable product name and build version used in runtime logs.
    ///
    /// This method is required so framework diagnostics never silently fall
    /// back to an implementation type name or an unrelated workspace package.
    fn metadata(&self) -> PluginMetadata;

    /// Declares the Telemetry API and per-game schema requirements of this
    /// product.
    ///
    /// This method is required rather than defaulting to the widest possible
    /// range. The runtime validates the declaration before invoking initialize,
    /// so an incompatible game never reaches product state setup or SDK
    /// registration.
    fn compatibility(&self) -> PluginCompatibility;

    /// Declares subscriptions and initializes plugin-owned state.
    ///
    /// Returning an error aborts initialization before a successful result is
    /// reported to the game. If later SDK registration fails, the framework
    /// unregisters the completed prefix and calls [`TelemetryPlugin::shutdown`].
    ///
    /// # Errors
    ///
    /// Implementations may return [`PluginError`] to reject the loading game,
    /// report invalid configuration, or propagate a subscription failure.
    fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult;

    /// Receives one subscribed telemetry channel update.
    fn channel(&mut self, _context: &mut PluginContext<'_>, _update: ChannelUpdate<'_>) {}

    /// Receives one SDK lifecycle, configuration, gameplay, or frame event.
    fn event(&mut self, _context: &mut PluginContext<'_>, _event: TelemetryEvent<'_>) {}

    /// Releases plugin-owned resources while the SDK logging function is valid.
    fn shutdown(&mut self, _context: &mut PluginContext<'_>) {}
}

/// Items referenced by [`export_plugin!`] expansions.
///
/// They are public for macro hygiene rather than as an application API. Their
/// signatures may evolve together with the macro crate between releases.
#[doc(hidden)]
pub mod __private {
    pub use crate::input_runtime::InputRuntime;
    pub use crate::runtime::Runtime;
    pub use scs_sdk_sys::{ScsInputInitParams, ScsResult, ScsTelemetryInitParams, ScsU32};
}

#[cfg(test)]
mod tests {
    use super::*;
    use scs_sdk::channels;

    #[test]
    fn optional_subscriptions_tolerate_only_capability_absence() {
        assert!(
            SubscriptionRequirement::Optional
                .tolerates_channel_registration_error(SdkError::NotFound)
        );
        assert!(
            SubscriptionRequirement::Optional
                .tolerates_channel_registration_error(SdkError::UnsupportedType)
        );
        for error in [
            SdkError::Unsupported,
            SdkError::InvalidParameter,
            SdkError::AlreadyRegistered,
            SdkError::NotNow,
            SdkError::Generic,
        ] {
            assert!(
                !SubscriptionRequirement::Optional.tolerates_channel_registration_error(error),
                "optional registration unexpectedly tolerated {error}"
            );
        }
        for error in [
            SdkError::NotFound,
            SdkError::UnsupportedType,
            SdkError::Generic,
        ] {
            assert!(
                !SubscriptionRequirement::Required.tolerates_channel_registration_error(error),
                "required registration unexpectedly tolerated {error}"
            );
        }

        for error in [SdkError::Unsupported, SdkError::NotFound] {
            assert!(
                SubscriptionRequirement::Optional.tolerates_event_registration_error(error),
                "optional event registration did not tolerate {error}"
            );
        }
        for error in [
            SdkError::InvalidParameter,
            SdkError::AlreadyRegistered,
            SdkError::UnsupportedType,
            SdkError::NotNow,
            SdkError::Generic,
        ] {
            assert!(
                !SubscriptionRequirement::Optional.tolerates_event_registration_error(error),
                "optional event registration unexpectedly tolerated {error}"
            );
        }
    }

    #[test]
    fn game_info_owns_rust_text_and_classifies_known_ids() {
        let ets2 = GameInfo::new(
            c"Euro Truck Simulator 2",
            c"eut2",
            GameSchemaVersion::new(1, 56),
        );
        assert_eq!(ets2.kind(), Game::EuroTruckSimulator2);
        assert_eq!(ets2.name(), "Euro Truck Simulator 2");
        assert_eq!(ets2.id(), "eut2");
        assert_eq!(ets2.schema_version(), GameSchemaVersion::new(1, 56));
        assert!(ets2.supports(sdk::game::capabilities::MULTI_TRAILER));
        assert_eq!(
            ets2.minimum_schema_for(sdk::game::capabilities::MULTI_TRAILER),
            Some(sdk::game::ets2::V1_14)
        );

        let ats = GameInfo::new(
            c"American Truck Simulator",
            c"ats",
            GameSchemaVersion::new(0, 0),
        );
        assert_eq!(ats.kind(), Game::AmericanTruckSimulator);
        assert!(!ats.supports(channels::truck::ADBLUE.availability()));

        let future = GameInfo::new(c"Future Truck", c"future", GameSchemaVersion::new(0, 0));
        assert_eq!(future.kind(), Game::Other);
        assert_eq!(future.id(), "future");
        assert_eq!(
            future.minimum_schema_for(sdk::game::capabilities::MULTI_TRAILER),
            None
        );
        assert!(!future.supports(sdk::game::capabilities::MULTI_TRAILER));
    }

    #[test]
    fn trailer_names_follow_the_official_zero_based_scheme() {
        let first = trailer_channel_name(channels::trailer::CONNECTED, TrailerIndex::ZERO)
            .expect("first trailer name should be valid");
        assert_eq!(first.as_c_str(), c"trailer.0.connected");
        let last_index = TrailerIndex::new(9).expect("index nine is in the SDK range");
        let last = trailer_channel_name(channels::trailer::WHEEL_ROTATION, last_index)
            .expect("last trailer name should be valid");
        assert_eq!(last.as_c_str(), c"trailer.9.wheel.rotation");
        assert_eq!(TrailerIndex::new(10), None);
        assert_eq!(
            trailer_channel_name(channels::truck::SPEED, TrailerIndex::ZERO)
                .expect_err("truck channels must not enter trailer naming")
                .result(),
            SdkError::InvalidParameter
        );
    }

    #[test]
    fn channel_update_requires_the_original_typed_descriptor() {
        let raw = scs_sdk_sys::ScsValue {
            type_: scs_sdk_sys::SCS_VALUE_TYPE_FLOAT,
            padding: scs_sdk_sys::ScsPadding::uninit(),
            value: scs_sdk_sys::ScsValueData {
                value_float: scs_sdk_sys::ScsValueFloat { value: 42.5 },
            },
        };
        let value =
            unsafe { ValueRef::from_ptr(&raw const raw) }.expect("test value should be present");
        let update = ChannelUpdate::new(
            channels::truck::SPEED.erase(),
            c"truck.speed",
            None,
            None,
            ChannelFlags::NONE,
            Some(value),
        );

        assert_eq!(update.value(channels::truck::SPEED), Some(42.5));
        assert_eq!(update.value(channels::truck::ENGINE_RPM), None);
        assert_eq!(update.registered_name(), "truck.speed");
    }

    #[test]
    fn channel_update_preserves_both_strong_index_domains() {
        let raw = scs_sdk_sys::ScsValue {
            type_: scs_sdk_sys::SCS_VALUE_TYPE_FLOAT,
            padding: scs_sdk_sys::ScsPadding::uninit(),
            value: scs_sdk_sys::ScsValueData {
                value_float: scs_sdk_sys::ScsValueFloat { value: 1.25 },
            },
        };
        let value =
            unsafe { ValueRef::from_ptr(&raw const raw) }.expect("test value should be present");
        let sdk_index = SdkIndex::new(2).expect("ordinary SDK index");
        let trailer_index = TrailerIndex::new(1).expect("second trailer");
        let update = ChannelUpdate::new(
            channels::trailer::WHEEL_ROTATION.erase(),
            c"trailer.1.wheel.rotation",
            Some(sdk_index),
            Some(trailer_index),
            ChannelFlags::EACH_FRAME,
            Some(value),
        );

        assert_eq!(update.index(), Some(sdk_index));
        assert_eq!(update.trailer_index(), Some(trailer_index));
        assert_eq!(update.registered_name(), "trailer.1.wheel.rotation");
        assert_eq!(update.value(channels::trailer::WHEEL_ROTATION), Some(1.25));
    }

    #[test]
    fn high_level_configuration_values_parse_known_and_preserve_unknown_text() {
        for (raw_market, expected) in [
            (
                c"external_contracts",
                Some(sdk::configuration::JobMarket::ExternalContracts),
            ),
            (c"future_market", None),
        ] {
            let attributes = [
                scs_sdk_sys::ScsNamedValue {
                    name: c"job.market".as_ptr(),
                    index: scs_sdk_sys::SCS_U32_NIL,
                    padding: scs_sdk_sys::ScsPadding::uninit(),
                    value: scs_sdk_sys::ScsValue {
                        type_: scs_sdk_sys::SCS_VALUE_TYPE_STRING,
                        padding: scs_sdk_sys::ScsPadding::uninit(),
                        value: scs_sdk_sys::ScsValueData {
                            value_string: scs_sdk_sys::ScsValueString {
                                value: raw_market.as_ptr(),
                            },
                        },
                    },
                },
                scs_sdk_sys::ScsNamedValue {
                    name: std::ptr::null(),
                    index: 0,
                    padding: scs_sdk_sys::ScsPadding::uninit(),
                    value: scs_sdk_sys::ScsValue {
                        type_: scs_sdk_sys::SCS_VALUE_TYPE_INVALID,
                        padding: scs_sdk_sys::ScsPadding::uninit(),
                        value: scs_sdk_sys::ScsValueData {
                            value_u64: scs_sdk_sys::ScsValueU64 { value: 0 },
                        },
                    },
                },
            ];
            let raw = scs_sdk_sys::ScsTelemetryConfiguration {
                id: c"job".as_ptr(),
                attributes: attributes.as_ptr(),
            };
            let inner = unsafe {
                ConfigurationRef::from_event_info((&raw const raw).cast::<std::ffi::c_void>())
            }
            .expect("configuration fixture");
            let event = ConfigurationEvent::new(inner);

            assert_eq!(event.job_market(), expected);
            assert_eq!(
                event.string(sdk::configuration::attributes::JOB_MARKET),
                Some(Cow::Borrowed(raw_market.to_str().expect("ASCII fixture")))
            );
        }
    }

    #[test]
    fn high_level_shifter_values_parse_known_and_preserve_unknown_text() {
        for (raw_shifter, expected) in [
            (c"hshifter", Some(sdk::configuration::ShifterType::HShifter)),
            (c"future_shifter", None),
        ] {
            let attributes = [
                scs_sdk_sys::ScsNamedValue {
                    name: c"shifter.type".as_ptr(),
                    index: scs_sdk_sys::SCS_U32_NIL,
                    padding: scs_sdk_sys::ScsPadding::uninit(),
                    value: scs_sdk_sys::ScsValue {
                        type_: scs_sdk_sys::SCS_VALUE_TYPE_STRING,
                        padding: scs_sdk_sys::ScsPadding::uninit(),
                        value: scs_sdk_sys::ScsValueData {
                            value_string: scs_sdk_sys::ScsValueString {
                                value: raw_shifter.as_ptr(),
                            },
                        },
                    },
                },
                scs_sdk_sys::ScsNamedValue {
                    name: std::ptr::null(),
                    index: 0,
                    padding: scs_sdk_sys::ScsPadding::uninit(),
                    value: scs_sdk_sys::ScsValue {
                        type_: scs_sdk_sys::SCS_VALUE_TYPE_INVALID,
                        padding: scs_sdk_sys::ScsPadding::uninit(),
                        value: scs_sdk_sys::ScsValueData {
                            value_u64: scs_sdk_sys::ScsValueU64 { value: 0 },
                        },
                    },
                },
            ];
            let raw = scs_sdk_sys::ScsTelemetryConfiguration {
                id: c"controls".as_ptr(),
                attributes: attributes.as_ptr(),
            };
            let inner = unsafe {
                ConfigurationRef::from_event_info((&raw const raw).cast::<std::ffi::c_void>())
            }
            .expect("configuration fixture");
            let event = ConfigurationEvent::new(inner);

            assert_eq!(event.shifter_type(), expected);
            assert_eq!(
                event.string(sdk::configuration::attributes::SHIFTER_TYPE),
                Some(Cow::Borrowed(raw_shifter.to_str().expect("ASCII fixture")))
            );
        }
    }

    #[test]
    fn high_level_gameplay_values_parse_known_and_preserve_unknown_text() {
        for (raw_offence, expected) in [
            (
                c"damaged_vehicle_usage",
                Some(sdk::gameplay::FineOffence::DamagedVehicleUsage),
            ),
            (c"future_offence", None),
        ] {
            let attributes = [
                scs_sdk_sys::ScsNamedValue {
                    name: c"fine.offence".as_ptr(),
                    index: scs_sdk_sys::SCS_U32_NIL,
                    padding: scs_sdk_sys::ScsPadding::uninit(),
                    value: scs_sdk_sys::ScsValue {
                        type_: scs_sdk_sys::SCS_VALUE_TYPE_STRING,
                        padding: scs_sdk_sys::ScsPadding::uninit(),
                        value: scs_sdk_sys::ScsValueData {
                            value_string: scs_sdk_sys::ScsValueString {
                                value: raw_offence.as_ptr(),
                            },
                        },
                    },
                },
                scs_sdk_sys::ScsNamedValue {
                    name: std::ptr::null(),
                    index: 0,
                    padding: scs_sdk_sys::ScsPadding::uninit(),
                    value: scs_sdk_sys::ScsValue {
                        type_: scs_sdk_sys::SCS_VALUE_TYPE_INVALID,
                        padding: scs_sdk_sys::ScsPadding::uninit(),
                        value: scs_sdk_sys::ScsValueData {
                            value_u64: scs_sdk_sys::ScsValueU64 { value: 0 },
                        },
                    },
                },
            ];
            let raw = scs_sdk_sys::ScsTelemetryGameplayEvent {
                id: c"player.fined".as_ptr(),
                attributes: attributes.as_ptr(),
            };
            let inner = unsafe {
                GameplayEventRef::from_event_info((&raw const raw).cast::<std::ffi::c_void>())
            }
            .expect("gameplay fixture");
            let event = GameplayEvent::new(inner);

            assert_eq!(event.fine_offence(), expected);
            assert_eq!(
                event.string(sdk::gameplay::attributes::FINE_OFFENCE),
                Some(Cow::Borrowed(raw_offence.to_str().expect("ASCII fixture")))
            );
        }
    }
}
