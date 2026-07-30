//! Internal runtime for the SCS Input Device API.
//!
//! Unlike telemetry subscriptions, input devices have no explicit unregister
//! function. SCS automatically unregisters successful-session devices before
//! calling `scs_input_shutdown`. If initialization fails after a prefix of
//! devices was registered, however, this runtime conservatively retains those
//! opaque callback contexts for the process lifetime. Generation and lifecycle
//! checks make every such stale callback inert.

use std::ffi::{CString, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use scs_sdk::input::{
    InputApi, InputCall, InputDeviceInput, InputDeviceRegistration, InputInitCall, InputSession,
};
use scs_sdk::{InputApiVersion, SdkError};
use scs_sdk_sys as sys;

use crate::{
    Game, InputDeviceId, InputDeviceSpec, InputDeviceType, InputEventFlags, InputEventRequest,
    InputGameInfo, InputPlugin, InputPluginCompatibility, InputPluginContext, InputSpec,
    InputValueType, PluginError, PluginMetadata, PluginResult,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputLifecycle {
    Idle,
    Initializing,
    Active,
    ShuttingDown,
}

struct OwnedInput {
    name: CString,
    display_name: CString,
    value_type: InputValueType,
}

/// Stable allocation used as one registered device's callback context.
///
/// An `Arc` keeps the pointee address stable while handles move between the
/// active and retired tables. The SCS callback receives only `Arc::as_ptr`; it
/// never owns a strong count and therefore cannot race Rust-side deallocation.
struct InputDeviceRegistrationContext {
    runtime: &'static InputRuntime,
    generation: u64,
    id: InputDeviceId,
    name: CString,
    display_name: CString,
    device_type: InputDeviceType,
    inputs: Vec<OwnedInput>,
    activity_notifications: bool,
    registered: AtomicBool,
}

struct PreparedInputPlugin {
    plugin: Box<dyn InputPlugin>,
    metadata: PluginMetadata,
    devices: Vec<InputDeviceSpec>,
}

struct InputRuntimeState {
    lifecycle: InputLifecycle,
    generation: u64,
    session: Option<InputSession>,
    game: Option<InputGameInfo>,
    plugin: Option<Box<dyn InputPlugin>>,
    metadata: Option<PluginMetadata>,
    devices: Vec<Arc<InputDeviceRegistrationContext>>,
    retired_devices: Vec<Arc<InputDeviceRegistrationContext>>,
}

impl InputRuntimeState {
    const fn new() -> Self {
        Self {
            lifecycle: InputLifecycle::Idle,
            generation: 0,
            session: None,
            game: None,
            plugin: None,
            metadata: None,
            devices: Vec::new(),
            retired_devices: Vec::new(),
        }
    }

    fn begin_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.generation
    }
}

/// Process-lifetime owner for one exported SCS input plugin.
///
/// Macro expansion stores this value in a `static`; every device callback
/// context refers back to that stable runtime address.
pub struct InputRuntime {
    state: Mutex<InputRuntimeState>,
}

impl InputRuntime {
    /// Creates an idle runtime suitable for static storage.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(InputRuntimeState::new()),
        }
    }

    /// Initializes an input plugin from the raw loader ABI.
    ///
    /// Every panic is contained before returning to SCS. A rejected or
    /// partially registered generation is made inactive; successful device
    /// contexts from a failed generation are retained conservatively because
    /// the Input API exposes no explicit unregistration function.
    ///
    /// # Safety
    ///
    /// `params` must point to the live initialization layout matching
    /// `version`. The call must be the direct serialized SCS input
    /// initialization invocation on the game main thread, and `self` must have
    /// a process-lifetime stable address.
    pub unsafe fn initialize<F>(
        &'static self,
        version: sys::ScsU32,
        params: *const sys::ScsInputInitParams,
        factory: F,
    ) -> sys::ScsResult
    where
        F: FnOnce() -> Box<dyn InputPlugin>,
    {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: The caller forwards the exact loader and pointer
            // contract documented above. All borrowed foreign data is consumed
            // before `initialize_inner` returns.
            unsafe { self.initialize_inner(version, params, factory) }
        }));

        match outcome {
            Ok(Ok(())) => sys::SCS_RESULT_OK,
            Ok(Err(error)) => error.result().code(),
            Err(_) => {
                // SAFETY: Recovery still runs inside the direct initialization
                // call, so the copied logger remains callable.
                unsafe { self.recover_after_initialization_panic() };
                sys::SCS_RESULT_GENERIC_ERROR
            }
        }
    }

    unsafe fn initialize_inner<F>(
        &'static self,
        version: sys::ScsU32,
        params: *const sys::ScsInputInitParams,
        factory: F,
    ) -> PluginResult
    where
        F: FnOnce() -> Box<dyn InputPlugin>,
    {
        let requested_version = InputApiVersion::from_raw(version);
        // SAFETY: This function inherits the initialization pointer contract
        // documented by `InputRuntime::initialize`.
        let api = unsafe { InputApi::from_raw(requested_version, params) }
            .map_err(|error| Self::api_initialization_error(requested_version, error))?;
        let game = InputGameInfo::new(api.game_name(), api.game_id(), api.game_version());

        self.ensure_idle()?;
        let prepared = Self::prepare_plugin(&api, &game, factory)?;
        let metadata = prepared.metadata;
        let generation = self.install_generation(&api, &game, prepared)?;

        let registration_result = api.with_init_call(|call| self.register_all(call, generation));
        if let Err(error) = registration_result {
            api.with_init_call(|call| {
                let context = InputPluginContext::callback(
                    call.logger(),
                    call.input_api_version(),
                    game.clone(),
                );
                context.error(format_args!("input device registration failed: {error}"));
                self.fail_generation(call, generation, true);
            });
            return Err(error);
        }

        if !self.activate_generation(generation) {
            let error = PluginError::new(
                SdkError::NotNow,
                "input runtime changed state during initialization",
            );
            api.with_init_call(|call| self.fail_generation(call, generation, true));
            return Err(error);
        }

        let registered_count = self
            .lock_state()
            .devices
            .iter()
            .filter(|device| device.registered.load(Ordering::Acquire))
            .count();
        api.with_init_call(|call| {
            let context =
                InputPluginContext::callback(call.logger(), call.input_api_version(), game);
            context.message(format_args!(
                concat!(
                    "[scs-sdk-plugin/input] initialized plugin name={:?} version={:?} ",
                    "devices={}"
                ),
                metadata.name(),
                metadata.version(),
                registered_count,
            ));
        });

        Ok(())
    }

    fn api_initialization_error(version: InputApiVersion, error: SdkError) -> PluginError {
        if error != SdkError::Unsupported {
            return PluginError::from(error);
        }
        let supported_versions = InputApi::SUPPORTED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        PluginError::new(
            error,
            format!("unsupported input API {version}, supported versions are {supported_versions}"),
        )
    }

    fn ensure_idle(&self) -> PluginResult {
        if self.lock_state().lifecycle == InputLifecycle::Idle {
            return Ok(());
        }
        Err(PluginError::new(
            SdkError::AlreadyRegistered,
            "input runtime is already initialized",
        ))
    }

    fn prepare_plugin<F>(
        api: &InputApi<'_>,
        game: &InputGameInfo,
        factory: F,
    ) -> PluginResult<PreparedInputPlugin>
    where
        F: FnOnce() -> Box<dyn InputPlugin>,
    {
        let mut plugin = factory();
        let metadata = plugin.metadata();
        if metadata.name().trim().is_empty() || metadata.version().trim().is_empty() {
            let error = PluginError::new(
                SdkError::InvalidParameter,
                "input plugin metadata name and version must both be non-empty",
            );
            api.with_init_call(|call| {
                let context = InputPluginContext::callback(
                    call.logger(),
                    call.input_api_version(),
                    game.clone(),
                );
                context.error(format_args!("input plugin metadata is invalid: {error}"));
            });
            return Err(error);
        }

        api.with_init_call(|call| {
            let context =
                InputPluginContext::callback(call.logger(), call.input_api_version(), game.clone());
            context.message(format_args!(
                concat!(
                    "[scs-sdk-plugin/input] starting plugin name={:?} version={:?} ",
                    "framework_version={:?}"
                ),
                metadata.name(),
                metadata.version(),
                env!("CARGO_PKG_VERSION"),
            ));
            context.message(format_args!(
                concat!(
                    "[scs-sdk-plugin/input] detected game_display_name={:?} game_id={:?} ",
                    "input_api={} input_game_version={}"
                ),
                game.name(),
                game.id(),
                call.input_api_version(),
                game.version(),
            ));
        });

        let compatibility = plugin.compatibility();
        if let Err(error) = Self::validate_compatibility(compatibility, api.version(), game) {
            api.with_init_call(|call| {
                let context = InputPluginContext::callback(
                    call.logger(),
                    call.input_api_version(),
                    game.clone(),
                );
                context.error(format_args!("input plugin compatibility rejected: {error}"));
            });
            return Err(error);
        }

        let mut devices = Vec::new();
        let result = api.with_init_call(|call| {
            let mut context = InputPluginContext::initializing(
                call.logger(),
                call.input_api_version(),
                game.clone(),
                &mut devices,
            );
            plugin.initialize(&mut context)
        });
        if let Err(error) = result {
            api.with_init_call(|call| {
                let mut context = InputPluginContext::callback(
                    call.logger(),
                    call.input_api_version(),
                    game.clone(),
                );
                context.error(format_args!("input plugin initialization failed: {error}"));
                let shutdown = catch_unwind(AssertUnwindSafe(|| plugin.shutdown(&mut context)));
                if shutdown.is_err() {
                    context.error(format_args!(
                        "input plugin panicked during rejected initialization shutdown"
                    ));
                }
            });
            return Err(error);
        }

        Ok(PreparedInputPlugin {
            plugin,
            metadata,
            devices,
        })
    }

    fn validate_compatibility(
        compatibility: InputPluginCompatibility,
        api_version: InputApiVersion,
        game: &InputGameInfo,
    ) -> PluginResult {
        let minimum_api = compatibility.minimum_input_api();
        if !InputApi::supports_version(minimum_api) {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                format!(
                    "plugin requires input API {minimum_api}, which has no audited framework adapter"
                ),
            ));
        }
        if !version_satisfies(api_version.raw(), minimum_api.raw()) {
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!(
                    "plugin requires input API {minimum_api} or newer within major {}, negotiated {api_version}",
                    minimum_api.major(),
                ),
            ));
        }

        let games = compatibility.games();
        if games.is_empty() {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                "input plugin compatibility must declare at least one supported game",
            ));
        }
        for (position, declared) in games.iter().copied().enumerate() {
            if declared.game() == Game::Other {
                return Err(PluginError::new(
                    SdkError::InvalidParameter,
                    "input plugin compatibility cannot use Game::Other",
                ));
            }
            if games[..position]
                .iter()
                .any(|previous| previous.game() == declared.game())
            {
                return Err(PluginError::new(
                    SdkError::InvalidParameter,
                    format!(
                        "input plugin compatibility declares game {:?} more than once",
                        declared.game(),
                    ),
                ));
            }
        }
        let Some(declared) = games
            .iter()
            .copied()
            .find(|declared| declared.game() == game.kind())
        else {
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!(
                    "input plugin does not support game {:?} with id {:?}",
                    game.kind(),
                    game.id(),
                ),
            ));
        };

        let minimum = declared.minimum_version();
        let actual = game.version();
        if !version_satisfies(actual.raw(), minimum.raw()) {
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!(
                    "plugin requires {:?} input game version {minimum} or newer within major {}, detected {actual}",
                    declared.game(),
                    minimum.major(),
                ),
            ));
        }
        Ok(())
    }

    fn install_generation(
        &'static self,
        api: &InputApi<'_>,
        game: &InputGameInfo,
        prepared: PreparedInputPlugin,
    ) -> PluginResult<u64> {
        let mut state = self.lock_state();
        let generation = state.begin_generation();
        let mut registrations = Vec::with_capacity(prepared.devices.len());
        for (position, spec) in prepared.devices.into_iter().enumerate() {
            let ordinal = u32::try_from(position).map_err(|_| {
                PluginError::new(
                    SdkError::InvalidParameter,
                    "input device count exceeds the framework identity range",
                )
            })?;
            registrations.push(Arc::new(Self::prepare_device(
                self,
                generation,
                InputDeviceId::from_ordinal(ordinal),
                spec,
            )?));
        }
        state.lifecycle = InputLifecycle::Initializing;
        state.session = Some(api.session());
        state.game = Some(game.clone());
        state.plugin = Some(prepared.plugin);
        state.metadata = Some(prepared.metadata);
        state.devices = registrations;
        Ok(generation)
    }

    fn prepare_device(
        runtime: &'static Self,
        generation: u64,
        id: InputDeviceId,
        spec: InputDeviceSpec,
    ) -> PluginResult<InputDeviceRegistrationContext> {
        let name = CString::new(spec.name()).map_err(|_| {
            PluginError::new(SdkError::InvalidParameter, "input device name contains NUL")
        })?;
        let display_name = CString::new(spec.display_name()).map_err(|_| {
            PluginError::new(
                SdkError::InvalidParameter,
                "input device display name contains NUL",
            )
        })?;
        let mut inputs = Vec::with_capacity(spec.inputs().len());
        for input in spec.inputs().iter().copied() {
            inputs.push(Self::prepare_input(input)?);
        }
        Ok(InputDeviceRegistrationContext {
            runtime,
            generation,
            id,
            name,
            display_name,
            device_type: spec.device_type(),
            inputs,
            activity_notifications: spec.activity_notifications(),
            registered: AtomicBool::new(false),
        })
    }

    fn prepare_input(spec: InputSpec) -> PluginResult<OwnedInput> {
        let name = CString::new(spec.name())
            .map_err(|_| PluginError::new(SdkError::InvalidParameter, "input name contains NUL"))?;
        let display_name = CString::new(spec.display_name()).map_err(|_| {
            PluginError::new(
                SdkError::InvalidParameter,
                "input display name contains NUL",
            )
        })?;
        Ok(OwnedInput {
            name,
            display_name,
            value_type: spec.value_type(),
        })
    }

    fn register_all(&'static self, call: &InputInitCall<'_>, generation: u64) -> PluginResult {
        let devices = self.lock_state().devices.clone();
        for device in devices {
            if device.generation != generation {
                return Err(PluginError::new(
                    SdkError::NotNow,
                    "input device generation changed during registration",
                ));
            }
            let descriptors = device
                .inputs
                .iter()
                .map(|input| {
                    InputDeviceInput::new(
                        input.name.as_c_str(),
                        input.display_name.as_c_str(),
                        input.value_type,
                    )
                })
                .collect::<Vec<_>>();
            let context = Arc::as_ptr(&device).cast_mut().cast::<c_void>();
            let activity_callback = device
                .activity_notifications
                .then_some(input_active_trampoline as sys::ScsInputActiveCallback);
            // SAFETY: `device` is an Arc allocation retained by RuntimeState
            // before this registration call. Its address and the runtime
            // backlink remain valid through shutdown, or in the retired table
            // after a failed initialization. Both trampolines contain panics
            // and perform generation validation before reaching product code.
            let registration = unsafe {
                InputDeviceRegistration::new(
                    device.name.as_c_str(),
                    device.display_name.as_c_str(),
                    device.device_type,
                    &descriptors,
                    context,
                    activity_callback,
                    input_event_trampoline,
                )
            }
            .map_err(PluginError::from)?;
            call.register_device(&registration).map_err(|error| {
                PluginError::new(
                    error,
                    format!("failed to register input device {:?}: {error}", device.name,),
                )
            })?;
            device.registered.store(true, Ordering::Release);
        }
        Ok(())
    }

    fn activate_generation(&self, generation: u64) -> bool {
        let mut state = self.lock_state();
        if state.generation != generation || state.lifecycle != InputLifecycle::Initializing {
            return false;
        }
        state.lifecycle = InputLifecycle::Active;
        true
    }

    fn fail_generation(
        &'static self,
        call: &InputInitCall<'_>,
        generation: u64,
        call_plugin: bool,
    ) {
        if call_plugin {
            let (game, mut plugin) = {
                let mut state = self.lock_state();
                (state.game.clone(), state.plugin.take())
            };
            if let (Some(game), Some(plugin)) = (game, plugin.as_deref_mut()) {
                let mut context =
                    InputPluginContext::callback(call.logger(), call.input_api_version(), game);
                let result = catch_unwind(AssertUnwindSafe(|| plugin.shutdown(&mut context)));
                if result.is_err() {
                    context.error(format_args!(
                        "input plugin panicked during failed initialization shutdown"
                    ));
                }
            }
        }
        self.retire_failed_generation(generation);
    }

    fn retire_failed_generation(&self, generation: u64) {
        let mut state = self.lock_state();
        if state.generation != generation {
            return;
        }
        let devices = std::mem::take(&mut state.devices);
        for device in devices {
            if device.registered.load(Ordering::Acquire) {
                // There is no Input API unregister function. Keeping this Arc
                // alive preserves the exact pointer previously given to SCS.
                state.retired_devices.push(device);
            }
        }
        state.plugin = None;
        state.metadata = None;
        state.game = None;
        state.session = None;
        state.lifecycle = InputLifecycle::Idle;
    }

    /// Shuts down the current input plugin after SCS removed its devices.
    ///
    /// Repeated calls while idle are harmless. Panics in product shutdown are
    /// contained and active device contexts are still released because the
    /// official lifecycle guarantees unregistration before this entry point.
    pub fn shutdown(&'static self) {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: The proc macro invokes this method only from the direct
            // SCS input shutdown entry point on the game main thread.
            unsafe { self.shutdown_inner() };
        }));
        if outcome.is_err() {
            self.finish_successful_shutdown();
        }
    }

    unsafe fn shutdown_inner(&'static self) {
        let (session, game, metadata, generation) = {
            let mut state = self.lock_state();
            if state.lifecycle == InputLifecycle::Idle {
                return;
            }
            state.lifecycle = InputLifecycle::ShuttingDown;
            (
                state.session,
                state.game.clone(),
                state.metadata,
                state.generation,
            )
        };
        let Some(session) = session else {
            self.finish_successful_shutdown();
            return;
        };
        // SAFETY: SCS has already unregistered every input device and is
        // directly invoking shutdown while the copied logger remains valid.
        unsafe {
            session.with_call(|call| {
                let mut plugin = self.lock_state().plugin.take();
                if let (Some(game), Some(plugin)) = (game.clone(), plugin.as_deref_mut()) {
                    let mut context =
                        InputPluginContext::callback(call.logger(), call.input_api_version(), game);
                    let result = catch_unwind(AssertUnwindSafe(|| plugin.shutdown(&mut context)));
                    if result.is_err() {
                        context.error(format_args!("input plugin panicked during shutdown"));
                    }
                }
                if let (Some(game), Some(metadata)) = (game, metadata) {
                    let context =
                        InputPluginContext::callback(call.logger(), call.input_api_version(), game);
                    context.message(format_args!(
                        "[scs-sdk-plugin/input] shutdown complete plugin name={:?} version={:?}",
                        metadata.name(),
                        metadata.version(),
                    ));
                }
            });
        }
        let current_generation = self.lock_state().generation;
        if current_generation == generation {
            self.finish_successful_shutdown();
        }
    }

    fn finish_successful_shutdown(&self) {
        let mut state = self.lock_state();
        for device in &state.devices {
            device.registered.store(false, Ordering::Release);
        }
        state.devices.clear();
        state.plugin = None;
        state.metadata = None;
        state.game = None;
        state.session = None;
        state.lifecycle = InputLifecycle::Idle;
    }

    unsafe fn recover_after_initialization_panic(&'static self) {
        let (session, game, generation) = {
            let state = self.lock_state();
            (state.session, state.game.clone(), state.generation)
        };
        if let Some(session) = session {
            // SAFETY: Recovery runs before the direct initialization call
            // returns, so the logger function remains in scope.
            unsafe {
                session.with_call(|call| {
                    if let Some(game) = game {
                        let context = InputPluginContext::callback(
                            call.logger(),
                            call.input_api_version(),
                            game,
                        );
                        context.error(format_args!("input plugin panicked during initialization"));
                    }
                    self.retire_failed_generation(generation);
                });
            }
        } else {
            self.retire_failed_generation(generation);
        }
    }

    fn dispatch_active(&'static self, registration: &InputDeviceRegistrationContext, active: bool) {
        let (session, game, enabled) = {
            let state = self.lock_state();
            (
                state.session,
                state.game.clone(),
                state.generation == registration.generation
                    && registration.registered.load(Ordering::Acquire)
                    && matches!(
                        state.lifecycle,
                        InputLifecycle::Initializing | InputLifecycle::Active
                    ),
            )
        };
        let (Some(session), Some(game)) = (session, game) else {
            return;
        };
        if !enabled {
            return;
        }
        // SAFETY: Dispatch occurs synchronously inside the SCS device callback
        // on the main thread while the session is active.
        unsafe {
            session.with_call(|call| {
                let mut state = self.lock_state();
                if state.generation != registration.generation
                    || !registration.registered.load(Ordering::Acquire)
                    || !matches!(
                        state.lifecycle,
                        InputLifecycle::Initializing | InputLifecycle::Active
                    )
                {
                    return;
                }
                let Some(plugin) = state.plugin.as_deref_mut() else {
                    return;
                };
                let mut context =
                    InputPluginContext::callback(call.logger(), call.input_api_version(), game);
                let result = catch_unwind(AssertUnwindSafe(|| {
                    plugin.device_active(&mut context, registration.id, active);
                }));
                if result.is_err() {
                    context.error(format_args!(
                        "input plugin panicked during activity callback for device {:?}",
                        registration.name,
                    ));
                }
            });
        }
    }

    fn dispatch_event(
        &'static self,
        registration: &InputDeviceRegistrationContext,
        output: *mut sys::ScsInputEvent,
        raw_flags: sys::ScsU32,
    ) -> sys::ScsResult {
        let (session, game, enabled) = {
            let state = self.lock_state();
            (
                state.session,
                state.game.clone(),
                state.generation == registration.generation
                    && registration.registered.load(Ordering::Acquire)
                    && matches!(
                        state.lifecycle,
                        InputLifecycle::Initializing | InputLifecycle::Active
                    ),
            )
        };
        let (Some(session), Some(game)) = (session, game) else {
            return sys::SCS_RESULT_NOT_FOUND;
        };
        if !enabled {
            return sys::SCS_RESULT_NOT_FOUND;
        }
        if output.is_null() {
            return sys::SCS_RESULT_INVALID_PARAMETER;
        }

        let dispatch = |call: &InputCall<'_>| {
            let mut state = self.lock_state();
            if state.generation != registration.generation
                || !registration.registered.load(Ordering::Acquire)
                || !matches!(
                    state.lifecycle,
                    InputLifecycle::Initializing | InputLifecycle::Active
                )
            {
                return sys::SCS_RESULT_NOT_FOUND;
            }
            let Some(plugin) = state.plugin.as_deref_mut() else {
                return sys::SCS_RESULT_NOT_FOUND;
            };
            let mut context =
                InputPluginContext::callback(call.logger(), call.input_api_version(), game);
            let request =
                InputEventRequest::new(registration.id, InputEventFlags::from_raw(raw_flags));
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                plugin.next_input_event(&mut context, request)
            }));
            let event = match outcome {
                Ok(Some(event)) => event,
                Ok(None) => return sys::SCS_RESULT_NOT_FOUND,
                Err(_) => {
                    context.error(format_args!(
                        "input plugin panicked while producing an event for device {:?}",
                        registration.name,
                    ));
                    return sys::SCS_RESULT_GENERIC_ERROR;
                }
            };
            let Some(expected_type) = registration
                .inputs
                .get(event.index().raw() as usize)
                .map(|input| input.value_type)
            else {
                context.error(format_args!(
                    concat!(
                        "input plugin returned out-of-range index {} for device {:?} ",
                        "with {} inputs"
                    ),
                    event.index().raw(),
                    registration.name,
                    registration.inputs.len(),
                ));
                return sys::SCS_RESULT_INVALID_PARAMETER;
            };
            if event.value().value_type() != expected_type {
                context.error(format_args!(
                    concat!(
                        "input plugin returned {:?} for device {:?} input {}, ",
                        "registered as {:?}"
                    ),
                    event.value().value_type(),
                    registration.name,
                    event.index().raw(),
                    expected_type,
                ));
                return sys::SCS_RESULT_INVALID_PARAMETER;
            }
            // SAFETY: `output` was checked non-null and is the live buffer
            // supplied for this direct callback. The device-local index
            // and registered value type were validated immediately above.
            match unsafe { event.write_to(output, expected_type) } {
                Ok(()) => sys::SCS_RESULT_OK,
                Err(error) => error.code(),
            }
        };

        // SAFETY: Dispatch occurs synchronously inside the SCS event callback
        // on the main thread while the copied session is active.
        unsafe { session.with_call(dispatch) }
    }

    fn lock_state(&self) -> MutexGuard<'_, InputRuntimeState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl Default for InputRuntime {
    fn default() -> Self {
        Self::new()
    }
}

const fn version_satisfies(actual: u32, minimum: u32) -> bool {
    sys::version_major(actual) == sys::version_major(minimum) && actual >= minimum
}

/// Optional activity callback shared by every registered input device.
///
/// # Safety
///
/// SCS must pass the exact stable context pointer supplied while registering
/// this device. Successful-session pointers remain live until SCS unregisters
/// devices before shutdown; failed-session pointers remain in the retired table.
unsafe extern "system" fn input_active_trampoline(active: sys::ScsU8, context: sys::ScsContext) {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: The registration contract above ties this pointer to one
        // stable Arc allocation retained by the runtime.
        let Some(registration) =
            (unsafe { context.cast::<InputDeviceRegistrationContext>().as_ref() })
        else {
            return;
        };
        registration
            .runtime
            .dispatch_active(registration, active != 0);
    }));
    if outcome.is_err() {
        // No independent logger is available if context recovery itself
        // panics. Swallowing the panic preserves the foreign ABI boundary.
    }
}

/// Required event callback shared by every registered input device.
///
/// # Safety
///
/// SCS must pass the exact stable registration context and a writable event
/// buffer for the duration of this direct main-thread callback.
unsafe extern "system" fn input_event_trampoline(
    event_info: *mut sys::ScsInputEvent,
    flags: sys::ScsU32,
    context: sys::ScsContext,
) -> sys::ScsResult {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Every installed context points to an Arc allocation retained
        // by either the active or retired runtime table.
        let Some(registration) =
            (unsafe { context.cast::<InputDeviceRegistrationContext>().as_ref() })
        else {
            return sys::SCS_RESULT_INVALID_PARAMETER;
        };
        registration
            .runtime
            .dispatch_event(registration, event_info, flags)
    }));
    outcome.unwrap_or(sys::SCS_RESULT_GENERIC_ERROR)
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, c_void};
    use std::mem::MaybeUninit;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

    use scs_sdk::input::game;

    use super::*;
    use crate::{InputAxisValue, InputGameCompatibility, InputIndex, InputValue};

    const INPUTS: [InputSpec; 2] = [
        InputSpec::new("button", "Button", InputValueType::Bool),
        InputSpec::new("axis", "Axis", InputValueType::Float),
    ];
    const SECOND_INPUTS: [InputSpec; 1] = [InputSpec::new(
        "second_button",
        "Second Button",
        InputValueType::Bool,
    )];
    const GAMES: [InputGameCompatibility; 2] = [
        InputGameCompatibility::new(Game::EuroTruckSimulator2, game::ets2::V1_00),
        InputGameCompatibility::new(Game::AmericanTruckSimulator, game::ats::V1_00),
    ];

    struct DeviceRecord {
        name: String,
        value_types: Vec<sys::ScsValueType>,
        context: AtomicPtr<c_void>,
        active_callback: Option<sys::ScsInputActiveCallback>,
        event_callback: sys::ScsInputEventCallback,
    }

    struct Harness {
        logs: Vec<(sys::ScsLogType, String)>,
        devices: Vec<DeviceRecord>,
        fail_registration_at: Option<usize>,
    }

    impl Harness {
        const fn new() -> Self {
            Self {
                logs: Vec::new(),
                devices: Vec::new(),
                fail_registration_at: None,
            }
        }

        fn reset(&mut self) {
            self.logs.clear();
            self.devices.clear();
            self.fail_registration_at = None;
        }
    }

    static HARNESS: Mutex<Harness> = Mutex::new(Harness::new());
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock_harness() -> MutexGuard<'static, Harness> {
        match HARNESS.lock() {
            Ok(harness) => harness,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn lock_tests() -> MutexGuard<'static, ()> {
        match TEST_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    unsafe extern "system" fn fake_log(level: sys::ScsLogType, message: sys::ScsString) {
        if message.is_null() {
            return;
        }
        // SAFETY: The runtime supplies a live NUL-terminated string for this
        // complete synchronous logger call.
        let message = unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned();
        lock_harness().logs.push((level, message));
    }

    unsafe extern "system" fn fake_register(device: *const sys::ScsInputDevice) -> sys::ScsResult {
        // SAFETY: InputRuntime supplies a live descriptor for this complete
        // registration call.
        let Some(device) = (unsafe { device.as_ref() }) else {
            return sys::SCS_RESULT_INVALID_PARAMETER;
        };
        let mut harness = lock_harness();
        if harness.fail_registration_at == Some(harness.devices.len()) {
            return sys::SCS_RESULT_GENERIC_ERROR;
        }
        if device.name.is_null() || device.inputs.is_null() {
            return sys::SCS_RESULT_INVALID_PARAMETER;
        }
        let Ok(input_count) = usize::try_from(device.input_count) else {
            return sys::SCS_RESULT_INVALID_PARAMETER;
        };
        // SAFETY: The descriptor declares this many entries and they remain
        // live for the complete registration call.
        let inputs = unsafe { std::slice::from_raw_parts(device.inputs, input_count) };
        // SAFETY: The framework retains this name for the registration call.
        let name = unsafe { CStr::from_ptr(device.name) }
            .to_string_lossy()
            .into_owned();
        harness.devices.push(DeviceRecord {
            name,
            value_types: inputs.iter().map(|input| input.value_type).collect(),
            context: AtomicPtr::new(device.callback_context),
            active_callback: device.input_active_callback,
            event_callback: device.input_event_callback,
        });
        sys::SCS_RESULT_OK
    }

    fn raw_api() -> sys::ScsInputInitParamsV100 {
        sys::ScsInputInitParamsV100 {
            common: sys::ScsSdkInitParamsV100 {
                game_name: c"Euro Truck Simulator 2".as_ptr(),
                game_id: c"eut2".as_ptr(),
                game_version: sys::SCS_INPUT_EUT2_GAME_VERSION_1_00,
                padding: MaybeUninit::uninit(),
                log: fake_log,
            },
            register_device: fake_register,
        }
    }

    #[derive(Clone, Copy)]
    enum EventBehavior {
        ValidThenNone,
        WrongType,
        OutOfRange,
        Panic,
        None,
    }

    struct Counts {
        active_calls: AtomicUsize,
        last_active: AtomicBool,
        event_calls: AtomicUsize,
        shutdown_calls: AtomicUsize,
    }

    impl Counts {
        const fn new() -> Self {
            Self {
                active_calls: AtomicUsize::new(0),
                last_active: AtomicBool::new(false),
                event_calls: AtomicUsize::new(0),
                shutdown_calls: AtomicUsize::new(0),
            }
        }
    }

    struct TestPlugin {
        counts: Arc<Counts>,
        behavior: EventBehavior,
        two_devices: bool,
    }

    impl TestPlugin {
        fn new(counts: Arc<Counts>, behavior: EventBehavior) -> Self {
            Self {
                counts,
                behavior,
                two_devices: false,
            }
        }

        fn with_two_devices(mut self) -> Self {
            self.two_devices = true;
            self
        }
    }

    impl InputPlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Input runtime test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> InputPluginCompatibility {
            InputPluginCompatibility::new(InputApiVersion::V1_00, &GAMES)
        }

        fn initialize(&mut self, context: &mut InputPluginContext<'_>) -> PluginResult {
            let first = context.register_device(
                InputDeviceSpec::new(
                    "test_device",
                    "Test Device",
                    InputDeviceType::Generic,
                    &INPUTS,
                )
                .with_activity_notifications(),
            )?;
            assert_eq!(first.ordinal(), 0);
            if self.two_devices {
                let second = context.register_device(InputDeviceSpec::new(
                    "second_device",
                    "Second Device",
                    InputDeviceType::Generic,
                    &SECOND_INPUTS,
                ))?;
                assert_eq!(second.ordinal(), 1);
            }
            Ok(())
        }

        fn device_active(
            &mut self,
            _context: &mut InputPluginContext<'_>,
            _device: InputDeviceId,
            active: bool,
        ) {
            self.counts.active_calls.fetch_add(1, Ordering::Relaxed);
            self.counts.last_active.store(active, Ordering::Relaxed);
        }

        fn next_input_event(
            &mut self,
            _context: &mut InputPluginContext<'_>,
            _request: InputEventRequest,
        ) -> Option<crate::InputEvent> {
            let call = self.counts.event_calls.fetch_add(1, Ordering::Relaxed);
            match self.behavior {
                EventBehavior::ValidThenNone if call == 0 => Some(crate::InputEvent::new(
                    InputIndex::new(0).expect("zero is valid"),
                    InputValue::Bool(true),
                )),
                EventBehavior::ValidThenNone | EventBehavior::None => None,
                EventBehavior::WrongType => Some(crate::InputEvent::new(
                    InputIndex::new(0).expect("zero is valid"),
                    InputValue::Float(
                        InputAxisValue::new(0.5).expect("value is a normalized axis position"),
                    ),
                )),
                EventBehavior::OutOfRange => Some(crate::InputEvent::new(
                    InputIndex::new(399).expect("399 is valid globally"),
                    InputValue::Bool(true),
                )),
                EventBehavior::Panic => panic!("intentional input callback panic"),
            }
        }

        fn shutdown(&mut self, _context: &mut InputPluginContext<'_>) {
            self.counts.shutdown_calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    type CallbackRecord = (
        Option<sys::ScsInputActiveCallback>,
        sys::ScsInputEventCallback,
        *mut c_void,
    );

    fn first_callbacks() -> CallbackRecord {
        let harness = lock_harness();
        let device = harness.devices.first().expect("registered device");
        (
            device.active_callback,
            device.event_callback,
            device.context.load(Ordering::Acquire),
        )
    }

    fn initialize(runtime: &'static InputRuntime, plugin: TestPlugin) -> sys::ScsResult {
        let raw = raw_api();
        // SAFETY: The local structure is the live matching v1.00 layout for
        // this complete synchronous call.
        unsafe {
            runtime.initialize(sys::SCS_INPUT_VERSION_1_00, (&raw const raw).cast(), || {
                Box::new(plugin)
            })
        }
    }

    /// Owns an Input runtime at the stable address required by device contexts.
    ///
    /// Production macro expansion uses a process `static`. Tests instead keep
    /// the stable allocation behind an explicit owner so it can be reclaimed
    /// after the fake SDK has forgotten every registered callback pointer. This
    /// preserves the real self-referential address invariant without disabling
    /// Miri's leak checker.
    struct TestInputRuntimeOwner {
        pointer: *mut InputRuntime,
    }

    impl TestInputRuntimeOwner {
        fn new() -> Self {
            Self {
                pointer: Box::into_raw(Box::new(InputRuntime::new())),
            }
        }

        fn runtime(&self) -> &'static InputRuntime {
            // SAFETY: `pointer` comes from `Box::into_raw`, remains owned by
            // this value, and is reclaimed only in `Drop` after the test stops
            // using the returned process-lifetime simulation.
            unsafe { &*self.pointer }
        }
    }

    impl Drop for TestInputRuntimeOwner {
        fn drop(&mut self) {
            // Every test resets the fake SDK before dropping the owner, so its
            // captured raw callback pointers are no longer foreign-reachable.
            // Release active and intentionally retired Arc contexts before the
            // allocation to which their `runtime` backlink refers.
            let runtime = self.runtime();
            let mut state = runtime.lock_state();
            state.devices.clear();
            state.retired_devices.clear();
            drop(state);

            // SAFETY: This is the unique pointer returned by `Box::into_raw`
            // in `new`, and all callback contexts containing a runtime backlink
            // were released above after the fake SDK discarded its pointers.
            unsafe { drop(Box::from_raw(self.pointer)) };
        }
    }

    #[test]
    fn successful_callbacks_and_shutdown_cross_the_real_trampolines() {
        let _test = lock_tests();
        lock_harness().reset();
        let counts = Arc::new(Counts::new());
        let runtime_owner = TestInputRuntimeOwner::new();
        let runtime = runtime_owner.runtime();
        assert_eq!(
            initialize(
                runtime,
                TestPlugin::new(Arc::clone(&counts), EventBehavior::ValidThenNone)
            ),
            sys::SCS_RESULT_OK
        );
        {
            let harness = lock_harness();
            assert_eq!(harness.devices.len(), 1);
            assert_eq!(harness.devices[0].name, "test_device");
            assert_eq!(
                harness.devices[0].value_types,
                [sys::SCS_VALUE_TYPE_BOOL, sys::SCS_VALUE_TYPE_FLOAT]
            );
        }

        let (active, event, context) = first_callbacks();
        // SAFETY: These are the exact callback and context captured by the fake
        // SDK during successful registration.
        unsafe { active.expect("activity callback requested")(1, context) };
        assert_eq!(counts.active_calls.load(Ordering::Relaxed), 1);
        assert!(counts.last_active.load(Ordering::Relaxed));

        let mut output = MaybeUninit::<sys::ScsInputEvent>::uninit();
        // SAFETY: `event` and `context` are the exact pair captured during
        // successful registration, and `output` is aligned writable storage
        // that remains live for this synchronous callback.
        let result = unsafe {
            event(
                output.as_mut_ptr(),
                sys::SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_IN_FRAME
                    | sys::SCS_INPUT_EVENT_CALLBACK_FLAG_FIRST_AFTER_ACTIVATION,
                context,
            )
        };
        assert_eq!(result, sys::SCS_RESULT_OK);
        // SAFETY: A successful callback initialized the complete event.
        let output = unsafe { output.assume_init() };
        assert_eq!(output.input_index, 0);
        // SAFETY: Input zero is registered as bool and runtime validated it.
        assert_eq!(unsafe { output.value.value_bool.value }, 1);

        let mut exhausted = MaybeUninit::<sys::ScsInputEvent>::uninit();
        // SAFETY: This reuses the live registered callback/context pair and
        // supplies aligned writable storage. The plugin's exhausted sequence
        // returns before writing, so the buffer is not subsequently read.
        let result = unsafe { event(exhausted.as_mut_ptr(), 0, context) };
        assert_eq!(result, sys::SCS_RESULT_NOT_FOUND);
        runtime.shutdown();
        assert_eq!(counts.shutdown_calls.load(Ordering::Relaxed), 1);
        let state = runtime.lock_state();
        assert_eq!(state.lifecycle, InputLifecycle::Idle);
        assert!(state.devices.is_empty());
        drop(state);
        lock_harness().reset();
        drop(runtime_owner);
    }

    #[test]
    fn callback_validation_and_panics_map_to_exact_sdk_results() {
        let _test = lock_tests();
        for (behavior, expected) in [
            (EventBehavior::WrongType, sys::SCS_RESULT_INVALID_PARAMETER),
            (EventBehavior::OutOfRange, sys::SCS_RESULT_INVALID_PARAMETER),
            (EventBehavior::Panic, sys::SCS_RESULT_GENERIC_ERROR),
            (EventBehavior::None, sys::SCS_RESULT_NOT_FOUND),
        ] {
            lock_harness().reset();
            let runtime_owner = TestInputRuntimeOwner::new();
            let runtime = runtime_owner.runtime();
            assert_eq!(
                initialize(runtime, TestPlugin::new(Arc::new(Counts::new()), behavior)),
                sys::SCS_RESULT_OK
            );
            let (_, callback, context) = first_callbacks();
            let mut output = MaybeUninit::<sys::ScsInputEvent>::uninit();
            // SAFETY: The fake SDK captured this exact callback with its stable
            // runtime-owned context, and `output` provides aligned writable
            // storage for every behavior that reaches event serialization.
            let result = unsafe { callback(output.as_mut_ptr(), 0, context) };
            assert_eq!(result, expected);
            runtime.shutdown();
            lock_harness().reset();
            drop(runtime_owner);
        }
    }

    #[test]
    fn partial_failure_retires_and_isolates_stale_contexts() {
        let _test = lock_tests();
        lock_harness().reset();
        lock_harness().fail_registration_at = Some(1);
        let counts = Arc::new(Counts::new());
        let runtime_owner = TestInputRuntimeOwner::new();
        let runtime = runtime_owner.runtime();
        assert_eq!(
            initialize(
                runtime,
                TestPlugin::new(Arc::clone(&counts), EventBehavior::ValidThenNone)
                    .with_two_devices()
            ),
            sys::SCS_RESULT_GENERIC_ERROR
        );
        let (_, stale_callback, stale_context) = first_callbacks();
        assert_eq!(counts.shutdown_calls.load(Ordering::Relaxed), 1);
        {
            let state = runtime.lock_state();
            assert_eq!(state.lifecycle, InputLifecycle::Idle);
            assert_eq!(state.retired_devices.len(), 1);
        }

        let mut output = MaybeUninit::<sys::ScsInputEvent>::uninit();
        // SAFETY: The retired allocation is deliberately retained. This
        // replay uses the exact captured callback/context pair and aligned
        // output storage to prove the generation is inert rather than dangling.
        let result = unsafe { stale_callback(output.as_mut_ptr(), 0, stale_context) };
        assert_eq!(result, sys::SCS_RESULT_NOT_FOUND);
        assert_eq!(counts.event_calls.load(Ordering::Relaxed), 0);

        lock_harness().reset();
        assert_eq!(
            initialize(
                runtime,
                TestPlugin::new(Arc::new(Counts::new()), EventBehavior::ValidThenNone)
            ),
            sys::SCS_RESULT_OK
        );
        // SAFETY: The prior callback context remains conservatively allocated,
        // and the same aligned output storage is valid for this synchronous
        // replay. Its old generation must reject dispatch before writing.
        let result = unsafe { stale_callback(output.as_mut_ptr(), 0, stale_context) };
        assert_eq!(result, sys::SCS_RESULT_NOT_FOUND);
        runtime.shutdown();
        lock_harness().reset();
        drop(runtime_owner);
    }

    #[test]
    fn null_output_is_rejected_before_product_dispatch() {
        let _test = lock_tests();
        lock_harness().reset();
        let counts = Arc::new(Counts::new());
        let runtime_owner = TestInputRuntimeOwner::new();
        let runtime = runtime_owner.runtime();
        assert_eq!(
            initialize(
                runtime,
                TestPlugin::new(Arc::clone(&counts), EventBehavior::ValidThenNone)
            ),
            sys::SCS_RESULT_OK
        );
        let (_, callback, context) = first_callbacks();
        // SAFETY: `callback` and `context` are the exact registered pair. A
        // null event output is the deliberate foreign input under test and the
        // trampoline validates it before any dereference.
        let result = unsafe { callback(ptr::null_mut(), 0, context) };
        assert_eq!(result, sys::SCS_RESULT_INVALID_PARAMETER);
        assert_eq!(counts.event_calls.load(Ordering::Relaxed), 0);
        runtime.shutdown();
        lock_harness().reset();
        drop(runtime_owner);
    }
}
