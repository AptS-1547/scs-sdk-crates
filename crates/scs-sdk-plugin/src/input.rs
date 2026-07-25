//! Safe application framework for the SCS Input Device API.
//!
//! Input devices use a pull model: while a registered device is active, SCS
//! repeatedly asks the plugin for the next event in the current frame. The
//! public types in this module keep device declarations, callback identity,
//! event flags, indices, and values in safe Rust. Raw callback pointers and
//! the lifetime of their opaque contexts remain owned by the runtime.

use std::ffi::CString;
use std::fmt;

use scs_sdk::{InputApiVersion, InputGameVersion, LogLevel, ScopedLogger, SdkError};

use crate::{Game, PluginError, PluginMetadata, PluginResult, classify_game_id};

pub use scs_sdk::input::{
    InputAxisValue, InputAxisValueError, InputDeviceType, InputEvent, InputEventFlags, InputIndex,
    InputValue, InputValueType,
};

/// Stable identity assigned to one device declared during plugin initialization.
///
/// The value is intentionally not interchangeable with an input index. SCS
/// calls each device through a distinct opaque context, while [`InputIndex`]
/// selects one entry inside that device's registered input array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputDeviceId(u32);

impl InputDeviceId {
    pub(crate) const fn from_ordinal(ordinal: u32) -> Self {
        Self(ordinal)
    }

    /// Zero-based declaration ordinal, useful for logs and application tables.
    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.0
    }
}

/// One bool or normalized float-axis input exposed by an input device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputSpec {
    name: &'static str,
    display_name: &'static str,
    value_type: InputValueType,
}

impl InputSpec {
    /// Describes one input using the exact names consumed by SCS.
    ///
    /// Syntax is validated when the containing device is registered. The
    /// configuration name accepts lowercase ASCII letters, digits, and
    /// underscores. The display name accepts ASCII letters, digits,
    /// underscores, spaces, and dots.
    #[must_use]
    pub const fn new(
        name: &'static str,
        display_name: &'static str,
        value_type: InputValueType,
    ) -> Self {
        Self {
            name,
            display_name,
            value_type,
        }
    }

    /// Name persisted by the game or interpreted as a semantical mix name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Human-facing name shown by the game's input UI.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        self.display_name
    }

    /// SDK representation expected for events targeting this input.
    #[must_use]
    pub const fn value_type(self) -> InputValueType {
        self.value_type
    }
}

/// Explicit declaration of one SCS input device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputDeviceSpec {
    name: &'static str,
    display_name: &'static str,
    device_type: InputDeviceType,
    inputs: &'static [InputSpec],
    activity_notifications: bool,
}

impl InputDeviceSpec {
    /// Creates a device declaration without the optional activity callback.
    ///
    /// Call [`InputDeviceSpec::with_activity_notifications`] when the product
    /// needs explicit activation and deactivation notifications.
    #[must_use]
    pub const fn new(
        name: &'static str,
        display_name: &'static str,
        device_type: InputDeviceType,
        inputs: &'static [InputSpec],
    ) -> Self {
        Self {
            name,
            display_name,
            device_type,
            inputs,
            activity_notifications: false,
        }
    }

    /// Explicitly requests the optional SCS device-activity callback.
    #[must_use]
    pub const fn with_activity_notifications(mut self) -> Self {
        self.activity_notifications = true;
        self
    }

    /// Unique device configuration name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Human-facing device name.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        self.display_name
    }

    /// Generic bindable or semantical device class.
    #[must_use]
    pub const fn device_type(self) -> InputDeviceType {
        self.device_type
    }

    /// Inputs in the exact zero-based order used by [`InputIndex`].
    #[must_use]
    pub const fn inputs(self) -> &'static [InputSpec] {
        self.inputs
    }

    /// Whether the runtime should register the optional activity callback.
    #[must_use]
    pub const fn activity_notifications(self) -> bool {
        self.activity_notifications
    }
}

/// Owned identity and input-specific version of the loading game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputGameInfo {
    name: String,
    id: String,
    kind: Game,
    version: InputGameVersion,
}

impl InputGameInfo {
    pub(crate) fn new(
        name: &std::ffi::CStr,
        id: &std::ffi::CStr,
        version: InputGameVersion,
    ) -> Self {
        Self {
            name: name.to_string_lossy().into_owned(),
            id: id.to_string_lossy().into_owned(),
            kind: classify_game_id(id),
            version,
        }
    }

    /// Human-facing name copied from the initialization parameters.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Stable textual game identifier such as `eut2` or `ats`.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Recognized game classification.
    #[must_use]
    pub const fn kind(&self) -> Game {
        self.kind
    }

    /// Game-specific Input API version, separate from telemetry schema.
    #[must_use]
    pub const fn version(&self) -> InputGameVersion {
        self.version
    }
}

/// Minimum Input API game version required for one recognized game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputGameCompatibility {
    game: Game,
    minimum_version: InputGameVersion,
}

impl InputGameCompatibility {
    /// Declares one recognized game and its oldest accepted Input API version.
    #[must_use]
    pub const fn new(game: Game, minimum_version: InputGameVersion) -> Self {
        Self {
            game,
            minimum_version,
        }
    }

    /// Game governed by this compatibility entry.
    #[must_use]
    pub const fn game(self) -> Game {
        self.game
    }

    /// Oldest accepted game-specific Input API version.
    #[must_use]
    pub const fn minimum_version(self) -> InputGameVersion {
        self.minimum_version
    }
}

/// Explicit API and game requirements of one input plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputPluginCompatibility {
    minimum_input_api: InputApiVersion,
    games: &'static [InputGameCompatibility],
}

impl InputPluginCompatibility {
    /// Creates a product compatibility declaration validated before devices
    /// are registered with SCS.
    #[must_use]
    pub const fn new(
        minimum_input_api: InputApiVersion,
        games: &'static [InputGameCompatibility],
    ) -> Self {
        Self {
            minimum_input_api,
            games,
        }
    }

    /// Oldest Input API layout required by the product.
    #[must_use]
    pub const fn minimum_input_api(self) -> InputApiVersion {
        self.minimum_input_api
    }

    /// Per-game input-version requirements.
    #[must_use]
    pub const fn games(self) -> &'static [InputGameCompatibility] {
        self.games
    }
}

/// Data supplied each time SCS asks one device for its next event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputEventRequest {
    device: InputDeviceId,
    flags: InputEventFlags,
}

impl InputEventRequest {
    pub(crate) const fn new(device: InputDeviceId, flags: InputEventFlags) -> Self {
        Self { device, flags }
    }

    /// Device whose callback SCS is currently polling.
    #[must_use]
    pub const fn device(self) -> InputDeviceId {
        self.device
    }

    /// Frame and activation boundary flags supplied by SCS.
    #[must_use]
    pub const fn flags(self) -> InputEventFlags {
        self.flags
    }
}

/// Safe capabilities available during one input-plugin hook.
pub struct InputPluginContext<'scope> {
    logger: ScopedLogger<'scope>,
    api_version: InputApiVersion,
    game: InputGameInfo,
    devices: Option<&'scope mut Vec<InputDeviceSpec>>,
}

impl<'scope> InputPluginContext<'scope> {
    pub(crate) fn initializing(
        logger: ScopedLogger<'scope>,
        api_version: InputApiVersion,
        game: InputGameInfo,
        devices: &'scope mut Vec<InputDeviceSpec>,
    ) -> Self {
        Self {
            logger,
            api_version,
            game,
            devices: Some(devices),
        }
    }

    pub(crate) fn callback(
        logger: ScopedLogger<'scope>,
        api_version: InputApiVersion,
        game: InputGameInfo,
    ) -> Self {
        Self {
            logger,
            api_version,
            game,
            devices: None,
        }
    }

    /// Loading-game identity copied from the Input API initialization call.
    #[must_use]
    pub const fn game(&self) -> &InputGameInfo {
        &self.game
    }

    /// Input API version selected by the SCS loader.
    #[must_use]
    pub const fn input_api_version(&self) -> InputApiVersion {
        self.api_version
    }

    /// Declares one input device and returns its callback identity.
    ///
    /// Registration remains explicit: the runtime does not infer a device from
    /// hook implementations or automatically install activity callbacks.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::NotNow`] outside [`InputPlugin::initialize`].
    /// Invalid names, empty or oversized input arrays, duplicate device/input
    /// names, and an unrepresentable device ordinal return
    /// [`SdkError::InvalidParameter`] or [`SdkError::AlreadyRegistered`].
    pub fn register_device(&mut self, spec: InputDeviceSpec) -> PluginResult<InputDeviceId> {
        let Some(devices) = self.devices.as_deref_mut() else {
            return Err(PluginError::new(
                SdkError::NotNow,
                "input devices may only be registered during plugin initialization",
            ));
        };
        validate_device_spec(spec)?;
        if devices.iter().any(|device| device.name() == spec.name()) {
            return Err(PluginError::new(
                SdkError::AlreadyRegistered,
                format!("duplicate input device name {:?}", spec.name()),
            ));
        }
        let ordinal = u32::try_from(devices.len()).map_err(|_| {
            PluginError::new(
                SdkError::InvalidParameter,
                "input device count exceeds the framework identity range",
            )
        })?;
        let id = InputDeviceId(ordinal);
        devices.push(spec);
        Ok(id)
    }

    /// Formats and writes one message to the game log.
    pub fn log(&self, level: LogLevel, arguments: fmt::Arguments<'_>) {
        let rendered = format!("{arguments}").replace('\0', " ");
        if let Ok(message) = CString::new(rendered) {
            self.logger.log(level, &message);
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
}

fn validate_device_spec(spec: InputDeviceSpec) -> PluginResult {
    if !valid_configuration_name(spec.name()) {
        return Err(PluginError::new(
            SdkError::InvalidParameter,
            format!("invalid input device configuration name {:?}", spec.name()),
        ));
    }
    if !valid_display_name(spec.display_name()) {
        return Err(PluginError::new(
            SdkError::InvalidParameter,
            format!(
                "invalid input device display name {:?}",
                spec.display_name()
            ),
        ));
    }
    if spec.inputs().is_empty() || spec.inputs().len() > InputIndex::MAX_COUNT as usize {
        return Err(PluginError::new(
            SdkError::InvalidParameter,
            format!(
                "input device {:?} must declare between 1 and {} inputs",
                spec.name(),
                InputIndex::MAX_COUNT,
            ),
        ));
    }
    for (position, input) in spec.inputs().iter().copied().enumerate() {
        if !valid_configuration_name(input.name()) {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                format!(
                    "invalid input name {:?} at position {position} for device {:?}",
                    input.name(),
                    spec.name(),
                ),
            ));
        }
        if !valid_display_name(input.display_name()) {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                format!(
                    "invalid input display name {:?} at position {position} for device {:?}",
                    input.display_name(),
                    spec.name(),
                ),
            ));
        }
        if spec.inputs()[..position]
            .iter()
            .any(|previous| previous.name() == input.name())
        {
            return Err(PluginError::new(
                SdkError::AlreadyRegistered,
                format!(
                    "duplicate input name {:?} for device {:?}",
                    input.name(),
                    spec.name(),
                ),
            ));
        }
    }
    Ok(())
}

fn valid_configuration_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_display_name(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphabetic()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b' ' | b'.')
        })
}

/// Application-facing lifecycle for one SCS input plugin.
pub trait InputPlugin: Send + 'static {
    /// Stable product identity used in runtime lifecycle logs.
    fn metadata(&self) -> PluginMetadata;

    /// Explicit Input API and per-game compatibility requirements.
    fn compatibility(&self) -> InputPluginCompatibility;

    /// Declares devices and initializes plugin-owned input state.
    ///
    /// # Errors
    ///
    /// Implementations may return [`PluginError`] to reject configuration or
    /// propagate a device-declaration failure before SCS registration begins.
    fn initialize(&mut self, context: &mut InputPluginContext<'_>) -> PluginResult;

    /// Receives an optional activation or deactivation notification.
    fn device_active(
        &mut self,
        _context: &mut InputPluginContext<'_>,
        _device: InputDeviceId,
        _active: bool,
    ) {
    }

    /// Returns the next event for the polled device, or `None` when the current
    /// frame has no more events.
    fn next_input_event(
        &mut self,
        _context: &mut InputPluginContext<'_>,
        _request: InputEventRequest,
    ) -> Option<InputEvent> {
        None
    }

    /// Releases plugin-owned resources after SCS has unregistered all devices.
    fn shutdown(&mut self, _context: &mut InputPluginContext<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOL: InputSpec = InputSpec::new("button", "Button 1", InputValueType::Bool);
    const DUPLICATES: [InputSpec; 2] = [BOOL, BOOL];

    #[test]
    fn validates_the_exact_header_name_character_sets() {
        assert!(valid_configuration_name("device_01"));
        assert!(!valid_configuration_name("Device"));
        assert!(!valid_configuration_name("device-name"));
        assert!(!valid_configuration_name(""));

        assert!(valid_display_name("Example Device 1.0"));
        assert!(!valid_display_name("Example/Device"));
        assert!(!valid_display_name("设备"));
    }

    #[test]
    fn validates_device_input_count_and_duplicate_names() {
        let empty = InputDeviceSpec::new("empty", "Empty", InputDeviceType::Generic, &[]);
        assert_eq!(
            validate_device_spec(empty)
                .err()
                .map(|error| error.result()),
            Some(SdkError::InvalidParameter)
        );

        let duplicate = InputDeviceSpec::new(
            "duplicate",
            "Duplicate",
            InputDeviceType::Generic,
            &DUPLICATES,
        );
        assert_eq!(
            validate_device_spec(duplicate)
                .err()
                .map(|error| error.result()),
            Some(SdkError::AlreadyRegistered)
        );
    }
}
