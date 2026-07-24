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

mod runtime;

use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::fmt;

use scs_sdk::{
    AnyChannel, Attribute, Channel, ChannelFlags, ChannelValue, ConfigurationId, ConfigurationRef,
    Event, FrameStartRef, GameSchemaVersion, GameplayEventId, GameplayEventRef, LogLevel, SdkCall,
    SdkError, SdkValue, StringValue, ValueRef,
};

/// Typed descriptor and value layer used when implementing plugin hooks.
///
/// Re-exporting the middle layer keeps application manifests dependent on the
/// framework crate alone while preserving its normal module organization.
pub use scs_sdk as sdk;
pub use scs_sdk_plugin_macros::export_plugin;

/// Result type returned by plugin initialization and subscription helpers.
pub type PluginResult<T = ()> = Result<T, PluginError>;

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
        let kind = match id.to_bytes() {
            b"eut2" => Game::EuroTruckSimulator2,
            b"ats" => Game::AmericanTruckSimulator,
            _ => Game::Other,
        };
        Self {
            name: name.to_string_lossy().into_owned(),
            id: id.to_string_lossy().into_owned(),
            kind,
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
}

#[derive(Clone, Debug)]
pub(crate) struct SubscriptionSpec {
    pub(crate) channel: AnyChannel,
    pub(crate) registered_name: CString,
    pub(crate) sdk_index: Option<u32>,
    pub(crate) trailer_index: Option<u32>,
    pub(crate) flags: ChannelFlags,
}

/// Event capability which a plugin may explicitly request during initialization.
///
/// This descriptor is intentionally separate from [`TelemetryEvent`]. A kind
/// represents registration intent and contains no borrowed callback payload;
/// `TelemetryEvent` represents one actual invocation after the runtime has
/// validated and decoded the event-specific SDK data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryEventKind {
    FrameStart,
    FrameEnd,
    Paused,
    Started,
    Configuration,
    Gameplay,
}

impl TelemetryEventKind {
    pub(crate) const fn sdk_event(self) -> Event {
        match self {
            Self::FrameStart => Event::FrameStart,
            Self::FrameEnd => Event::FrameEnd,
            Self::Paused => Event::Paused,
            Self::Started => Event::Started,
            Self::Configuration => Event::Configuration,
            Self::Gameplay => Event::Gameplay,
        }
    }
}

/// Safe capabilities available while the framework calls plugin code.
///
/// Logging is valid in every hook. Channel registration is intentionally open
/// only during [`TelemetryPlugin::initialize`]; calling a subscription method
/// from another hook returns [`SdkError::NotNow`] without touching the SDK.
pub struct PluginContext<'scope> {
    call: &'scope SdkCall<'scope>,
    game: GameInfo,
    events: Option<&'scope mut Vec<TelemetryEventKind>>,
    subscriptions: Option<&'scope mut Vec<SubscriptionSpec>>,
}

impl<'scope> PluginContext<'scope> {
    pub(crate) fn initializing(
        call: &'scope SdkCall<'scope>,
        game: GameInfo,
        events: &'scope mut Vec<TelemetryEventKind>,
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
    /// shutdown hook instead of initialization.
    pub fn subscribe_event(&mut self, event: TelemetryEventKind) -> PluginResult {
        let Some(events) = self.events.as_deref_mut() else {
            return Err(PluginError::new(
                SdkError::NotNow,
                "events may only be subscribed during plugin initialization",
            ));
        };
        if events.contains(&event) {
            return Err(PluginError::new(
                SdkError::AlreadyRegistered,
                format!("duplicate event subscription for {event:?}"),
            ));
        }
        events.push(event);
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
    /// [`SdkError::NotNow`] outside initialization.
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
        index: u32,
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
        index: u32,
        flags: ChannelFlags,
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
    /// scalar `trailer.*` channel or `trailer_index` is at least
    /// [`scs_sdk::configuration::MAX_TRAILERS`].
    pub fn subscribe_trailer<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: u32,
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
        trailer_index: u32,
        flags: ChannelFlags,
    ) -> PluginResult {
        if channel.is_indexed() {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                "indexed trailer channels require subscribe_trailer_at",
            ));
        }
        let name = trailer_channel_name(channel, trailer_index)?;
        self.push_subscription(channel.erase(), name, None, Some(trailer_index), flags)
    }

    /// Subscribes to one SDK-indexed member of an explicit trailer channel.
    ///
    /// For example, `trailer_index = 1` and `index = 2` select wheel 2 of the
    /// second trailer. Both values are zero-based.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::InvalidParameter`] unless `channel` is an indexed
    /// `trailer.*` descriptor and the trailer number is in the SDK range.
    pub fn subscribe_trailer_at<T: ChannelValue>(
        &mut self,
        channel: Channel<T>,
        trailer_index: u32,
        index: u32,
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
        trailer_index: u32,
        index: u32,
        flags: ChannelFlags,
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
        )
    }

    fn push_subscription(
        &mut self,
        channel: AnyChannel,
        registered_name: CString,
        sdk_index: Option<u32>,
        trailer_index: Option<u32>,
        flags: ChannelFlags,
    ) -> PluginResult {
        let Some(subscriptions) = self.subscriptions.as_deref_mut() else {
            return Err(PluginError::new(
                SdkError::NotNow,
                "channels may only be subscribed during plugin initialization",
            ));
        };

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
        });
        Ok(())
    }
}

fn trailer_channel_name<T: ChannelValue>(
    channel: Channel<T>,
    trailer_index: u32,
) -> PluginResult<CString> {
    let maximum = u32::try_from(scs_sdk::configuration::MAX_TRAILERS).map_err(|error| {
        PluginError::new(
            SdkError::Generic,
            format!("SDK trailer limit does not fit u32: {error}"),
        )
    })?;
    if trailer_index >= maximum {
        return Err(PluginError::new(
            SdkError::InvalidParameter,
            format!("trailer index {trailer_index} is outside 0..{maximum}"),
        ));
    }

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
    index: Option<u32>,
    trailer_index: Option<u32>,
    flags: ChannelFlags,
    value: Option<ValueRef<'a>>,
}

impl<'a> ChannelUpdate<'a> {
    pub(crate) const fn new(
        channel: AnyChannel,
        registered_name: &'a CStr,
        index: Option<u32>,
        trailer_index: Option<u32>,
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
    pub const fn index(self) -> Option<u32> {
        self.index
    }

    /// Zero-based trailer number embedded in the registered name.
    #[must_use]
    pub const fn trailer_index(self) -> Option<u32> {
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
        index: u32,
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
        index: u32,
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
    pub use crate::runtime::Runtime;
    pub use scs_sdk_sys::{ScsResult, ScsTelemetryInitParams, ScsU32};
}

#[cfg(test)]
mod tests {
    use super::*;
    use scs_sdk::channels;

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

        let ats = GameInfo::new(
            c"American Truck Simulator",
            c"ats",
            GameSchemaVersion::new(0, 0),
        );
        assert_eq!(ats.kind(), Game::AmericanTruckSimulator);

        let future = GameInfo::new(c"Future Truck", c"future", GameSchemaVersion::new(0, 0));
        assert_eq!(future.kind(), Game::Other);
        assert_eq!(future.id(), "future");
    }

    #[test]
    fn trailer_names_follow_the_official_zero_based_scheme() {
        let first = trailer_channel_name(channels::trailer::CONNECTED, 0)
            .expect("first trailer name should be valid");
        assert_eq!(first.as_c_str(), c"trailer.0.connected");
        let last = trailer_channel_name(channels::trailer::WHEEL_ROTATION, 9)
            .expect("last trailer name should be valid");
        assert_eq!(last.as_c_str(), c"trailer.9.wheel.rotation");
        assert_eq!(
            trailer_channel_name(channels::trailer::CONNECTED, 10)
                .expect_err("trailer index ten is outside the SDK limit")
                .result(),
            SdkError::InvalidParameter
        );
        assert_eq!(
            trailer_channel_name(channels::truck::SPEED, 0)
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
}
