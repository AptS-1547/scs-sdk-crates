//! Internal SCS ABI runtime.
//!
//! No application-facing type in this module exposes a raw pointer. The macro
//! crate calls [`Runtime::initialize`] and [`Runtime::shutdown`], while SCS calls
//! the two trampolines registered below. Every unsafe operation is kept close
//! to the SDK invariant which justifies it.

use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use scs_sdk::{
    Event, FrameStartRef, SdkCall, SdkError, SdkIndex, TelemetryApi, TelemetryApiVersion,
    TelemetrySession, ValueRef,
};
use scs_sdk_sys as sys;

use crate::{
    ChannelUpdate, ConfigurationEvent, EventSubscriptionSpec, Game, GameInfo, GameplayEvent,
    PluginCompatibility, PluginContext, PluginError, PluginMetadata, PluginResult,
    SubscriptionSpec, TelemetryEvent, TelemetryPlugin, telemetry_api_satisfies,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lifecycle {
    Idle,
    Initializing,
    Active,
    ShuttingDown,
}

/// Heap allocation used as one channel callback's opaque context.
///
/// `Arc<ChannelRegistration>` gives the SDK a stable shared pointee address.
/// Moving an `Arc` handle between active and retired tables neither relocates
/// the allocation nor creates a new unique borrow of its contents. If SCS
/// rejects unregistration, the handle is retained instead of releasing the
/// foreign context. That deliberately trades a tiny bounded allocation for
/// freedom from a stale pointer.
struct ChannelRegistration {
    runtime: &'static Runtime,
    generation: u64,
    spec: SubscriptionSpec,
    registered: AtomicBool,
}

/// Stable event callback context for one runtime generation.
///
/// Event unregistration identifies only the event number, not the callback or
/// context. Keeping the generation beside the event ensures that a delayed
/// callback from an earlier session is ignored instead of reaching a new plugin
/// instance which reuses the same process-wide runtime.
struct EventRegistration {
    runtime: &'static Runtime,
    generation: u64,
    event: Event,
    requirement: crate::SubscriptionRequirement,
    registered: AtomicBool,
}

/// Product value and explicit capability declarations collected before any SDK
/// registration is attempted.
struct PreparedPlugin {
    plugin: Box<dyn TelemetryPlugin>,
    metadata: PluginMetadata,
    events: Vec<EventSubscriptionSpec>,
    channels: Vec<SubscriptionSpec>,
}

struct RuntimeState {
    lifecycle: Lifecycle,
    generation: u64,
    session: Option<TelemetrySession>,
    game: Option<GameInfo>,
    plugin: Option<Box<dyn TelemetryPlugin>>,
    metadata: Option<PluginMetadata>,
    events: Vec<Arc<EventRegistration>>,
    channels: Vec<Arc<ChannelRegistration>>,
    retired_events: Vec<Arc<EventRegistration>>,
    retired_channels: Vec<Arc<ChannelRegistration>>,
}

impl RuntimeState {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::Idle,
            generation: 0,
            session: None,
            game: None,
            plugin: None,
            metadata: None,
            events: Vec::new(),
            channels: Vec::new(),
            retired_events: Vec::new(),
            retired_channels: Vec::new(),
        }
    }

    fn begin_generation(&mut self) -> u64 {
        // Generation zero is reserved for the never-initialized state. Wrapping
        // after 2^64 initializations is harmless because no context from that
        // many completed lifecycles can remain registered in a real process.
        self.generation = self.generation.wrapping_add(1).max(1);
        self.generation
    }
}

/// Process-lifetime owner for one exported telemetry plugin.
///
/// Macro expansions place this value in a `static`. Event and channel callbacks
/// point to separate shared generation registrations which refer back to this
/// process-lifetime runtime.
pub struct Runtime {
    state: Mutex<RuntimeState>,
}

impl Runtime {
    /// Creates an idle runtime suitable for placement in a process static.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(RuntimeState::new()),
        }
    }

    /// Initializes a plugin from the raw SCS ABI entry point.
    ///
    /// This method never lets a Rust panic unwind into the game. A panic maps to
    /// `SCS_RESULT_GENERIC_ERROR`; any already-installed callback contexts are
    /// retired or unregistered before the result is returned.
    ///
    /// # Safety
    ///
    /// `params` must point to the live SCS telemetry initialization structure
    /// corresponding to `version`. The call must be the direct, serialized SDK
    /// initialization invocation on the game main thread, and `self` must have a
    /// process-lifetime stable address.
    pub unsafe fn initialize<F>(
        &'static self,
        version: sys::ScsU32,
        params: *const sys::ScsTelemetryInitParams,
        factory: F,
    ) -> sys::ScsResult
    where
        F: FnOnce() -> Box<dyn TelemetryPlugin>,
    {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: The caller forwards the SDK initialization contract to
            // this method. `initialize_inner` consumes all pointer borrows
            // before returning.
            unsafe { self.initialize_inner(version, params, factory) }
        }));

        match outcome {
            Ok(Ok(())) => sys::SCS_RESULT_OK,
            Ok(Err(error)) => error.result().code(),
            Err(_) => {
                // SAFETY: We are still executing inside the direct SDK init
                // call. Any session installed before the panic is therefore
                // valid for best-effort rollback and logging.
                unsafe { self.recover_after_panic("plugin panicked during initialization") };
                sys::SCS_RESULT_GENERIC_ERROR
            }
        }
    }

    unsafe fn initialize_inner<F>(
        &'static self,
        version: sys::ScsU32,
        params: *const sys::ScsTelemetryInitParams,
        factory: F,
    ) -> PluginResult
    where
        F: FnOnce() -> Box<dyn TelemetryPlugin>,
    {
        let requested_version = TelemetryApiVersion::from_raw(version);

        // SAFETY: `initialize_inner` inherits the raw initialization pointer
        // contract documented by `Runtime::initialize`.
        let api = unsafe { TelemetryApi::from_raw(requested_version, params) }
            .map_err(|error| Self::api_initialization_error(requested_version, error))?;
        let game = GameInfo::new(api.game_name(), api.game_id(), api.game_schema_version());

        self.ensure_idle()?;
        let prepared = Self::prepare_plugin(&api, &game, factory)?;
        let metadata = prepared.metadata;
        let generation = self.install_generation(&api, &game, prepared);

        let registration_result = api.with_call(|call| self.register_all(call));
        if let Err(error) = registration_result {
            api.with_call(|call| {
                let game_for_log = game.clone();
                let context = PluginContext::callback(call, game_for_log);
                context.error(format_args!("SDK registration failed: {error}"));
                self.rollback(call, generation, true);
            });
            return Err(error);
        }

        if !self.activate_generation(generation) {
            let error = PluginError::new(
                SdkError::NotNow,
                "telemetry runtime changed state during initialization",
            );
            api.with_call(|call| self.rollback(call, generation, true));
            return Err(error);
        }

        let (event_count, channel_count) = {
            let state = self.lock_state();
            let events = state
                .events
                .iter()
                .filter(|registration| registration.registered.load(Ordering::Acquire))
                .count();
            let channels = state
                .channels
                .iter()
                .filter(|registration| registration.registered.load(Ordering::Acquire))
                .count();
            (events, channels)
        };

        api.with_call(|call| {
            let context = PluginContext::callback(call, game);
            context.message(format_args!(
                concat!(
                    "[scs-sdk-plugin] initialized plugin name={:?} version={:?} ",
                    "events={} channels={}"
                ),
                metadata.name(),
                metadata.version(),
                event_count,
                channel_count,
            ));
        });

        Ok(())
    }

    fn api_initialization_error(version: TelemetryApiVersion, error: SdkError) -> PluginError {
        if error != SdkError::Unsupported {
            return PluginError::from(error);
        }

        // TelemetryApi is the sole owner of the audited adapter list. Runtime
        // formats that list for diagnostics but deliberately does not repeat a
        // version match of its own, so adding a future adapter cannot leave two
        // independent compatibility policies behind.
        let supported_versions = TelemetryApi::SUPPORTED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        PluginError::new(
            error,
            format!(
                "unsupported telemetry API {version}, supported versions are {supported_versions}"
            ),
        )
    }

    fn validate_compatibility(
        compatibility: PluginCompatibility,
        api_version: TelemetryApiVersion,
        game: &GameInfo,
    ) -> PluginResult {
        let minimum_api = compatibility.minimum_telemetry_api();
        if !TelemetryApi::supports_version(minimum_api) {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                format!(
                    "plugin requires telemetry API {minimum_api}, which has no audited framework adapter"
                ),
            ));
        }

        // A minimum is meaningful only inside one additive API-major family.
        // A future major may change semantics even if its numeric value is
        // greater, so it requires a new explicit product declaration.
        if !telemetry_api_satisfies(api_version, minimum_api) {
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!(
                    "plugin requires telemetry API {minimum_api} or newer within major {}, negotiated {api_version}",
                    minimum_api.major(),
                ),
            ));
        }

        let games = compatibility.games();
        if games.is_empty() {
            return Err(PluginError::new(
                SdkError::InvalidParameter,
                "plugin compatibility must declare at least one supported game",
            ));
        }

        for (position, declared) in games.iter().copied().enumerate() {
            if declared.game() == Game::Other {
                return Err(PluginError::new(
                    SdkError::InvalidParameter,
                    "plugin compatibility cannot use the ambiguous Game::Other classification",
                ));
            }
            if games[..position]
                .iter()
                .any(|previous| previous.game() == declared.game())
            {
                return Err(PluginError::new(
                    SdkError::InvalidParameter,
                    format!(
                        "plugin compatibility declares game {:?} more than once",
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
                    "plugin does not support game {:?} with id {:?}",
                    game.kind(),
                    game.id(),
                ),
            ));
        };

        let minimum_schema = declared.minimum_schema();
        let actual_schema = game.schema_version();
        if actual_schema.major() != minimum_schema.major() || actual_schema < minimum_schema {
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!(
                    "plugin requires {:?} telemetry schema {minimum_schema} or newer within major {}, detected {actual_schema}",
                    declared.game(),
                    minimum_schema.major(),
                ),
            ));
        }

        Ok(())
    }

    fn ensure_idle(&self) -> PluginResult {
        if self.lock_state().lifecycle == Lifecycle::Idle {
            return Ok(());
        }
        Err(PluginError::new(
            SdkError::AlreadyRegistered,
            "telemetry runtime is already initialized",
        ))
    }

    fn prepare_plugin<F>(
        api: &TelemetryApi<'_>,
        game: &GameInfo,
        factory: F,
    ) -> PluginResult<PreparedPlugin>
    where
        F: FnOnce() -> Box<dyn TelemetryPlugin>,
    {
        // TelemetryApi owns the negotiated version together with the audited
        // function table. Read it from that canonical session view instead of
        // threading a second copy through the plugin runtime.
        let api_version = api.version();
        let mut plugin = factory();
        let metadata = plugin.metadata();
        if metadata.name().trim().is_empty() || metadata.version().trim().is_empty() {
            let error = PluginError::new(
                SdkError::InvalidParameter,
                "plugin metadata name and version must both be non-empty",
            );
            api.with_call(|call| {
                let context = PluginContext::callback(call, game.clone());
                context.error(format_args!("plugin metadata is invalid: {error}"));
            });
            return Err(error);
        }

        api.with_call(|call| {
            let context = PluginContext::callback(call, game.clone());
            context.message(format_args!(
                concat!(
                    "[scs-sdk-plugin] starting plugin name={:?} version={:?} ",
                    "framework_version={:?}"
                ),
                metadata.name(),
                metadata.version(),
                env!("CARGO_PKG_VERSION"),
            ));
            context.message(format_args!(
                concat!(
                    "[scs-sdk-plugin] detected game_display_name={:?} game_id={:?} ",
                    "telemetry_api={} telemetry_schema={}"
                ),
                game.name(),
                game.id(),
                api_version,
                game.schema_version(),
            ));
        });

        let compatibility = plugin.compatibility();
        if let Err(error) = Self::validate_compatibility(compatibility, api_version, game) {
            api.with_call(|call| {
                let context = PluginContext::callback(call, game.clone());
                context.error(format_args!("plugin compatibility rejected: {error}"));
            });
            return Err(error);
        }

        let mut events = Vec::new();
        let mut subscriptions = Vec::new();
        let result = api.with_call(|call| {
            let mut context =
                PluginContext::initializing(call, game.clone(), &mut events, &mut subscriptions);
            plugin.initialize(&mut context)
        });
        if let Err(error) = result {
            api.with_call(|call| {
                let mut context = PluginContext::callback(call, game.clone());
                context.error(format_args!("plugin initialization failed: {error}"));
                plugin.shutdown(&mut context);
            });
            return Err(error);
        }
        Ok(PreparedPlugin {
            plugin,
            metadata,
            events,
            channels: subscriptions,
        })
    }

    fn install_generation(
        &'static self,
        api: &TelemetryApi<'_>,
        game: &GameInfo,
        prepared: PreparedPlugin,
    ) -> u64 {
        let mut state = self.lock_state();
        let generation = state.begin_generation();
        state.lifecycle = Lifecycle::Initializing;
        state.session = Some(api.session());
        state.game = Some(game.clone());
        state.plugin = Some(prepared.plugin);
        state.metadata = Some(prepared.metadata);
        state.events = prepared
            .events
            .into_iter()
            .map(|spec| {
                Arc::new(EventRegistration {
                    runtime: self,
                    generation,
                    event: spec.event,
                    requirement: spec.requirement,
                    registered: AtomicBool::new(false),
                })
            })
            .collect();
        state.channels = prepared
            .channels
            .into_iter()
            .map(|spec| {
                Arc::new(ChannelRegistration {
                    runtime: self,
                    generation,
                    spec,
                    registered: AtomicBool::new(false),
                })
            })
            .collect();
        generation
    }

    fn activate_generation(&self, generation: u64) -> bool {
        let mut state = self.lock_state();
        if state.generation != generation || state.lifecycle != Lifecycle::Initializing {
            return false;
        }
        state.lifecycle = Lifecycle::Active;
        true
    }

    fn register_all(&'static self, call: &SdkCall<'_>) -> PluginResult {
        self.register_events(call)?;
        self.register_channels(call)
    }

    fn register_events(&'static self, call: &SdkCall<'_>) -> PluginResult {
        let (event_count, game) = {
            let state = self.lock_state();
            let Some(game) = state.game.clone() else {
                return Err(PluginError::new(
                    SdkError::NotNow,
                    "event registration requires an installed game session",
                ));
            };
            (state.events.len(), game)
        };
        for position in 0..event_count {
            let registration = {
                let state = self.lock_state();
                let Some(registration) = state.events.get(position) else {
                    return Err(PluginError::new(
                        SdkError::Generic,
                        "event registration table changed during initialization",
                    ));
                };
                Arc::as_ptr(registration)
            };

            // SAFETY: The pointer targets a shared event context retained by
            // this runtime generation until unregistration succeeds.
            let registration_ref = unsafe { &*registration };
            let event = registration_ref.event;
            let event_context = registration.cast_mut().cast::<c_void>();

            let api_version = call.telemetry_api_version();
            let minimum_api = event.minimum_api_version();
            if !telemetry_api_satisfies(api_version, minimum_api) {
                if matches!(
                    registration_ref.requirement,
                    crate::SubscriptionRequirement::Optional
                ) {
                    continue;
                }
                return Err(PluginError::new(
                    SdkError::Unsupported,
                    format!(
                        "registering required event {event:?} requires telemetry API {minimum_api}; negotiated {api_version}"
                    ),
                ));
            }

            let minimum_schema = game.minimum_schema_for(event.availability());
            let schema_supported = game.supports(event.availability());
            if !schema_supported {
                if matches!(
                    registration_ref.requirement,
                    crate::SubscriptionRequirement::Optional
                ) {
                    continue;
                }
                let detail = minimum_schema.map_or_else(
                    || format!("is not available for {:?}", game.kind()),
                    |minimum| {
                        format!(
                            "requires {:?} telemetry schema {minimum}; detected {}",
                            game.kind(),
                            game.schema_version(),
                        )
                    },
                );
                return Err(PluginError::new(
                    SdkError::Unsupported,
                    format!("registering required event {event:?} {detail}"),
                ));
            }

            // SAFETY: The stable context records its generation, the callback
            // uses the SDK ABI, catches panics, and decodes event data only for
            // the matching event kind.
            if let Err(error) =
                unsafe { call.register_event(event, event_trampoline, event_context) }
            {
                if registration_ref
                    .requirement
                    .tolerates_event_registration_error(error)
                {
                    continue;
                }
                return Err(PluginError::new(
                    error,
                    format!("registering event {event:?} failed: {error}"),
                ));
            }

            registration_ref.registered.store(true, Ordering::Release);
        }

        Ok(())
    }

    fn register_channels(&'static self, call: &SdkCall<'_>) -> PluginResult {
        let (channel_count, game) = {
            let state = self.lock_state();
            let Some(game) = state.game.clone() else {
                return Err(PluginError::new(
                    SdkError::NotNow,
                    "channel registration requires an installed game session",
                ));
            };
            (state.channels.len(), game)
        };
        for position in 0..channel_count {
            let registration = {
                let state = self.lock_state();
                let Some(registration) = state.channels.get(position) else {
                    return Err(PluginError::new(
                        SdkError::Generic,
                        "channel registration table changed during initialization",
                    ));
                };
                Arc::as_ptr(registration)
            };

            // SAFETY: The pointer targets a shared registration retained by the
            // current runtime generation. No lifecycle operation can remove it
            // during this serialized SDK initialization call.
            let registration_ref = unsafe { &*registration };
            let context = registration.cast_mut().cast::<c_void>();

            let value_type = registration_ref.spec.channel.value_type();
            let minimum_api = value_type.minimum_api_version();
            let api_version = call.telemetry_api_version();
            if !telemetry_api_satisfies(api_version, minimum_api) {
                if matches!(
                    registration_ref.spec.requirement,
                    crate::SubscriptionRequirement::Optional
                ) {
                    continue;
                }
                return Err(PluginError::new(
                    SdkError::Unsupported,
                    format!(
                        "registering required channel {:?}, index {:?}, type {value_type:?} requires telemetry API {minimum_api}; negotiated {api_version}",
                        registration_ref.spec.registered_name, registration_ref.spec.sdk_index,
                    ),
                ));
            }

            if !Self::channel_schema_is_available(&registration_ref.spec, &game)? {
                continue;
            }

            let result = unsafe {
                call.register_erased_channel(
                    &registration_ref.spec.registered_name,
                    registration_ref.spec.sdk_index,
                    value_type,
                    registration_ref.spec.flags,
                    channel_trampoline,
                    context,
                )
            };
            if let Err(error) = result {
                if registration_ref
                    .spec
                    .requirement
                    .tolerates_channel_registration_error(error)
                {
                    continue;
                }
                return Err(PluginError::new(
                    error,
                    format!(
                        "registering channel {:?}, index {:?}, type {:?} failed: {error}",
                        registration_ref.spec.registered_name,
                        registration_ref.spec.sdk_index,
                        value_type,
                    ),
                ));
            }

            registration_ref.registered.store(true, Ordering::Release);
        }

        Ok(())
    }

    /// Applies per-game descriptor history before a foreign channel call.
    ///
    /// `Ok(false)` is reserved for an optional capability which the detected
    /// schema does not expose. A required capability returns a contextual
    /// `Unsupported` error so the normal initialization rollback path remains
    /// responsible for every earlier registration.
    fn channel_schema_is_available(spec: &SubscriptionSpec, game: &GameInfo) -> PluginResult<bool> {
        let descriptor_minimum = game.minimum_schema_for(spec.channel.availability());
        let descriptor_supported = game.supports(spec.channel.availability());
        let trailer_minimum = spec
            .trailer_index
            .and_then(|_| game.minimum_schema_for(scs_sdk::game::capabilities::MULTI_TRAILER));
        let trailer_supported = spec.trailer_index.is_none()
            || game.supports(scs_sdk::game::capabilities::MULTI_TRAILER);
        if descriptor_supported && trailer_supported {
            return Ok(true);
        }
        if matches!(spec.requirement, crate::SubscriptionRequirement::Optional) {
            return Ok(false);
        }

        let (capability, minimum) = if descriptor_supported {
            ("numbered multi-trailer namespace", trailer_minimum)
        } else {
            ("channel descriptor", descriptor_minimum)
        };
        let detail = minimum.map_or_else(
            || format!("is not available for {:?}", game.kind()),
            |minimum| {
                format!(
                    "requires {:?} telemetry schema {minimum}; detected {}",
                    game.kind(),
                    game.schema_version(),
                )
            },
        );
        Err(PluginError::new(
            SdkError::Unsupported,
            format!(
                "registering required {capability} {:?}, index {:?} {detail}",
                spec.registered_name, spec.sdk_index,
            ),
        ))
    }

    /// Shuts down the current runtime from the generated ABI entry point.
    ///
    /// Calling this method while idle is a no-op, which makes a defensive or
    /// repeated SDK shutdown harmless. Panics in plugin shutdown hooks are
    /// contained and the runtime still returns to the idle state.
    pub fn shutdown(&'static self) {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: The proc-macro only calls this method from the direct SCS
            // shutdown entry point on the SDK thread.
            unsafe { self.shutdown_inner() };
        }));
        if outcome.is_err() {
            // No SDK calls are attempted after a shutdown panic because the
            // panic might have occurred inside the game's unregistration table.
            self.force_idle();
        }
    }

    unsafe fn shutdown_inner(&'static self) {
        let session = {
            let mut state = self.lock_state();
            if state.lifecycle == Lifecycle::Idle {
                return;
            }
            state.lifecycle = Lifecycle::ShuttingDown;
            state.session
        };

        if let Some(session) = session {
            // SAFETY: `shutdown_inner` is called directly from SCS shutdown on
            // the main thread while the session's function table remains live.
            unsafe {
                session.with_call(|call| {
                    let generation = self.lock_state().generation;
                    self.rollback(call, generation, true);
                });
            }
        } else {
            self.force_idle();
        }
    }

    fn rollback(&'static self, call: &SdkCall<'_>, generation: u64, call_plugin: bool) {
        self.unregister_channels(call, generation);
        self.unregister_events(call, generation);

        if call_plugin {
            let (game, mut plugin) = {
                let mut state = self.lock_state();
                (state.game.clone(), state.plugin.take())
            };
            if let (Some(game), Some(plugin)) = (game, plugin.as_deref_mut()) {
                let mut context = PluginContext::callback(call, game);
                let shutdown_result = catch_unwind(AssertUnwindSafe(|| {
                    plugin.shutdown(&mut context);
                }));
                if shutdown_result.is_err() {
                    context.error(format_args!("plugin panicked during shutdown"));
                }
            }
        }

        let (game, metadata) = {
            let state = self.lock_state();
            (state.game.clone(), state.metadata)
        };
        if let (Some(game), Some(metadata)) = (game, metadata) {
            let context = PluginContext::callback(call, game);
            context.message(format_args!(
                "[scs-sdk-plugin] shutdown complete plugin name={:?} version={:?}",
                metadata.name(),
                metadata.version(),
            ));
        }
        self.finish_generation(generation);
    }

    fn unregister_channels(&self, call: &SdkCall<'_>, generation: u64) {
        let channel_count = self.lock_state().channels.len();
        for position in (0..channel_count).rev() {
            let registration = {
                let state = self.lock_state();
                state.channels.get(position).map(Arc::as_ptr)
            };
            let Some(registration) = registration else {
                continue;
            };

            // SAFETY: Pointees remain allocated until `finish_generation`, which is
            // called only after this reverse unregistration pass completes.
            let registration_ref = unsafe { &*registration };
            if registration_ref.generation != generation
                || !registration_ref.registered.load(Ordering::Acquire)
            {
                continue;
            }
            let result = unsafe {
                call.unregister_erased_channel(
                    &registration_ref.spec.registered_name,
                    registration_ref.spec.sdk_index,
                    registration_ref.spec.channel.value_type(),
                )
            };
            match result {
                Ok(()) => {
                    registration_ref.registered.store(false, Ordering::Release);
                }
                Err(error) => {
                    let game = self.lock_state().game.clone();
                    if let Some(game) = game {
                        let context = PluginContext::callback(call, game);
                        context.warning(format_args!(
                            "failed to unregister channel {:?}, index {:?}: {error}",
                            registration_ref.spec.registered_name, registration_ref.spec.sdk_index,
                        ));
                    }
                }
            }
        }
    }

    fn unregister_events(&self, call: &SdkCall<'_>, generation: u64) {
        let event_count = self.lock_state().events.len();
        for position in (0..event_count).rev() {
            let registration = {
                let state = self.lock_state();
                state.events.get(position).map(Arc::as_ptr)
            };
            let Some(registration) = registration else {
                continue;
            };

            // SAFETY: Pointees remain allocated until `finish_generation`, after
            // this reverse unregistration pass has completed.
            let registration_ref = unsafe { &*registration };
            if registration_ref.generation != generation
                || !registration_ref.registered.load(Ordering::Acquire)
            {
                continue;
            }
            let event = registration_ref.event;
            let result = unsafe { call.unregister_event(event) };
            match result {
                Ok(()) => {
                    registration_ref.registered.store(false, Ordering::Release);
                }
                Err(error) => {
                    let game = self.lock_state().game.clone();
                    if let Some(game) = game {
                        let context = PluginContext::callback(call, game);
                        context.warning(format_args!(
                            "failed to unregister event {event:?}: {error}"
                        ));
                    }
                }
            }
        }
    }

    fn finish_generation(&self, generation: u64) {
        let mut state = self.lock_state();
        if state.generation != generation {
            return;
        }

        let channels = std::mem::take(&mut state.channels);
        for channel in channels {
            if channel.registered.load(Ordering::Acquire) {
                // SCS still knows this opaque pointer. Keep its allocation and
                // runtime backlink alive even though generation checks prevent
                // it from reaching a future plugin instance.
                state.retired_channels.push(channel);
            }
        }
        let events = std::mem::take(&mut state.events);
        for event in events {
            if event.registered.load(Ordering::Acquire) {
                state.retired_events.push(event);
            }
        }
        state.plugin = None;
        state.metadata = None;
        state.game = None;
        state.session = None;
        state.lifecycle = Lifecycle::Idle;
    }

    fn force_idle(&self) {
        let generation = self.lock_state().generation;
        let mut state = self.lock_state();
        if state.generation != generation {
            return;
        }
        let channels = std::mem::take(&mut state.channels);
        state.retired_channels.extend(channels);
        let events = std::mem::take(&mut state.events);
        state.retired_events.extend(events);
        state.plugin = None;
        state.metadata = None;
        state.game = None;
        state.session = None;
        state.lifecycle = Lifecycle::Idle;
    }

    unsafe fn recover_after_panic(&'static self, message: &str) {
        let (session, game, generation) = {
            let state = self.lock_state();
            (state.session, state.game.clone(), state.generation)
        };
        if let Some(session) = session {
            // SAFETY: Recovery runs inside the same direct initialization call
            // that installed this session.
            unsafe {
                session.with_call(|call| {
                    if let Some(game) = game {
                        let context = PluginContext::callback(call, game);
                        context.error(format_args!("{message}"));
                    }
                    self.rollback(call, generation, false);
                });
            }
        } else {
            self.force_idle();
        }
    }

    fn dispatch_event(
        &'static self,
        registration: &EventRegistration,
        raw_event: sys::ScsEvent,
        event_info: *const c_void,
    ) {
        let (session, game, generation, active) = {
            let state = self.lock_state();
            (
                state.session,
                state.game.clone(),
                state.generation,
                state.generation == registration.generation
                    && registration.registered.load(Ordering::Acquire)
                    && matches!(state.lifecycle, Lifecycle::Initializing | Lifecycle::Active),
            )
        };
        let (Some(session), Some(game)) = (session, game) else {
            return;
        };
        if !active {
            return;
        }
        if raw_event != registration.event.raw() {
            return;
        }

        // SAFETY: This function is reached only from `event_trampoline`, a
        // direct callback from SCS on the active SDK thread.
        unsafe {
            session.with_call(|call| {
                // `raw_event` was checked against this registration's typed
                // descriptor above. Decode by that canonical descriptor rather
                // than maintaining another raw-number-to-event table here.
                let event = match registration.event {
                    Event::FrameStart => {
                        // SAFETY: SCS associates a live frame-start structure
                        // with this exact event discriminator.
                        let Some(frame) = FrameStartRef::from_event_info(event_info) else {
                            return;
                        };
                        TelemetryEvent::FrameStart(frame)
                    }
                    Event::FrameEnd => TelemetryEvent::FrameEnd,
                    Event::Paused => TelemetryEvent::Paused,
                    Event::Started => TelemetryEvent::Started,
                    Event::Configuration => {
                        // SAFETY: SCS associates a live configuration structure
                        // with this exact event discriminator.
                        let Some(configuration) =
                            scs_sdk::ConfigurationRef::from_event_info(event_info)
                        else {
                            return;
                        };
                        TelemetryEvent::Configuration(ConfigurationEvent::new(configuration))
                    }
                    Event::Gameplay => {
                        // SAFETY: SCS associates a live gameplay structure with
                        // this exact event discriminator.
                        let Some(gameplay) = scs_sdk::GameplayEventRef::from_event_info(event_info)
                        else {
                            return;
                        };
                        TelemetryEvent::Gameplay(GameplayEvent::new(gameplay))
                    }
                };

                let mut state = self.lock_state();
                if state.generation != generation
                    || !matches!(state.lifecycle, Lifecycle::Initializing | Lifecycle::Active)
                {
                    return;
                }
                let Some(plugin) = state.plugin.as_deref_mut() else {
                    return;
                };
                let mut context = PluginContext::callback(call, game);
                let result = catch_unwind(AssertUnwindSafe(|| {
                    plugin.event(&mut context, event);
                }));
                if result.is_err() {
                    context.error(format_args!("plugin panicked while handling an event"));
                }
            });
        }
    }

    fn dispatch_channel(
        &'static self,
        registration: &ChannelRegistration,
        callback_index: sys::ScsU32,
        raw_value: *const sys::ScsValue,
    ) {
        let (session, game, active) = {
            let state = self.lock_state();
            (
                state.session,
                state.game.clone(),
                state.generation == registration.generation
                    && matches!(state.lifecycle, Lifecycle::Initializing | Lifecycle::Active)
                    && registration.registered.load(Ordering::Acquire),
            )
        };
        let (Some(session), Some(game)) = (session, game) else {
            return;
        };
        if !active {
            return;
        }

        // SAFETY: The SDK callback contract guarantees either a null pointer for
        // `NO_VALUE` or a live tagged value for the duration of this callback.
        let value = unsafe { ValueRef::from_ptr(raw_value) };
        let index = SdkIndex::new(callback_index);

        // SAFETY: This function is called synchronously from the registered SDK
        // channel trampoline while the copied session remains active.
        unsafe {
            session.with_call(|call| {
                let update = ChannelUpdate::new(
                    registration.spec.channel,
                    &registration.spec.registered_name,
                    index,
                    registration.spec.trailer_index,
                    registration.spec.flags,
                    value,
                );
                let mut state = self.lock_state();
                if state.generation != registration.generation
                    || !matches!(state.lifecycle, Lifecycle::Initializing | Lifecycle::Active)
                {
                    return;
                }
                let Some(plugin) = state.plugin.as_deref_mut() else {
                    return;
                };
                let mut context = PluginContext::callback(call, game);
                let result = catch_unwind(AssertUnwindSafe(|| {
                    plugin.channel(&mut context, update);
                }));
                if result.is_err() {
                    context.error(format_args!("plugin panicked while handling a channel"));
                }
            });
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, RuntimeState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// Single event callback registered for every public telemetry event.
///
/// # Safety
///
/// SCS must pass the stable `EventRegistration` pointer used for this exact
/// registration and an event-specific info pointer satisfying the SDK header.
unsafe extern "system" fn event_trampoline(
    event: sys::ScsEvent,
    event_info: *const c_void,
    context: sys::ScsContext,
) {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Every event registration installs a shared generation context.
        // Failed unregistrations retain that allocation in `retired_events`.
        let Some(registration) = (unsafe { context.cast::<EventRegistration>().as_ref() }) else {
            return;
        };
        registration
            .runtime
            .dispatch_event(registration, event, event_info);
    }));
    if outcome.is_err() {
        // There is no independent logger capability when context recovery itself
        // fails. Swallowing the panic is the only ABI-safe action here.
    }
}

/// Single channel callback shared by every type-erased subscription.
///
/// # Safety
///
/// SCS must pass the stable `ChannelRegistration` pointer used for this exact
/// registration and keep `value` live for the duration specified by the SDK.
unsafe extern "system" fn channel_trampoline(
    _name: sys::ScsString,
    index: sys::ScsU32,
    value: *const sys::ScsValue,
    context: sys::ScsContext,
) {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Each registration installs its own stable shared context. A
        // failed unregistration moves that box to `retired_channels` rather
        // than releasing the pointee.
        let Some(registration) = (unsafe { context.cast::<ChannelRegistration>().as_ref() }) else {
            return;
        };
        registration
            .runtime
            .dispatch_channel(registration, index, value);
    }));
    if outcome.is_err() {
        // Panic containment at the outermost ABI frame is mandatory even though
        // individual plugin hooks are also caught inside the runtime.
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, c_void};
    use std::ptr;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicU32, AtomicUsize, Ordering};

    use scs_sdk::channels;

    use super::*;

    struct EventRecord {
        event: sys::ScsEvent,
        callback: sys::ScsTelemetryEventCallback,
        context: AtomicPtr<c_void>,
    }

    struct ChannelRecord {
        name: Vec<u8>,
        index: sys::ScsU32,
        value_type: sys::ScsValueType,
        callback: sys::ScsTelemetryChannelCallback,
        context: AtomicPtr<c_void>,
    }

    type EventInvocation = (
        sys::ScsEvent,
        sys::ScsTelemetryEventCallback,
        sys::ScsContext,
    );

    struct Harness {
        logs: Vec<(sys::ScsLogType, String)>,
        events: Vec<EventRecord>,
        channels: Vec<ChannelRecord>,
        selective_event_failure: Option<(sys::ScsEvent, sys::ScsResult)>,
        fail_channel_registration: bool,
        selective_channel_failure: Option<(Vec<u8>, sys::ScsResult)>,
        fail_event_unregistration: bool,
    }

    impl Harness {
        const fn new() -> Self {
            Self {
                logs: Vec::new(),
                events: Vec::new(),
                channels: Vec::new(),
                selective_event_failure: None,
                fail_channel_registration: false,
                selective_channel_failure: None,
                fail_event_unregistration: false,
            }
        }

        fn reset(&mut self) {
            self.logs.clear();
            self.events.clear();
            self.channels.clear();
            self.selective_event_failure = None;
            self.fail_channel_registration = false;
            self.selective_channel_failure = None;
            self.fail_event_unregistration = false;
        }
    }

    static HARNESS: Mutex<Harness> = Mutex::new(Harness::new());
    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    fn harness() -> MutexGuard<'static, Harness> {
        match HARNESS.lock() {
            Ok(harness) => harness,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn serial_guard() -> MutexGuard<'static, ()> {
        match TEST_SERIAL.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    unsafe extern "system" fn fake_log(level: sys::ScsLogType, message: sys::ScsString) {
        // SAFETY: The framework passes a live NUL-terminated CString for the
        // duration of this direct logger invocation.
        let message = unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned();
        harness().logs.push((level, message));
    }

    unsafe extern "system" fn fake_register_event(
        event: sys::ScsEvent,
        callback: sys::ScsTelemetryEventCallback,
        context: sys::ScsContext,
    ) -> sys::ScsResult {
        let mut harness = harness();
        if let Some((failure_event, result)) = harness.selective_event_failure {
            if failure_event == event {
                return result;
            }
        }
        harness.events.push(EventRecord {
            event,
            callback,
            context: AtomicPtr::new(context),
        });
        sys::SCS_RESULT_OK
    }

    unsafe extern "system" fn fake_unregister_event(event: sys::ScsEvent) -> sys::ScsResult {
        let mut harness = harness();
        if harness.fail_event_unregistration {
            return sys::SCS_RESULT_GENERIC_ERROR;
        }
        let Some(position) = harness
            .events
            .iter()
            .rposition(|candidate| candidate.event == event)
        else {
            return sys::SCS_RESULT_NOT_FOUND;
        };
        harness.events.remove(position);
        sys::SCS_RESULT_OK
    }

    unsafe extern "system" fn fake_register_channel(
        name: sys::ScsString,
        index: sys::ScsU32,
        value_type: sys::ScsValueType,
        _flags: sys::ScsU32,
        callback: sys::ScsTelemetryChannelCallback,
        context: sys::ScsContext,
    ) -> sys::ScsResult {
        let mut harness = harness();
        // SAFETY: The runtime passes a live NUL-terminated registration name
        // and retains its owning CString until unregistration completes.
        let name = unsafe { CStr::from_ptr(name) }.to_bytes();
        if harness.fail_channel_registration {
            return sys::SCS_RESULT_UNSUPPORTED_TYPE;
        }
        if let Some((failure_name, result)) = harness.selective_channel_failure.as_ref() {
            if failure_name == name {
                return *result;
            }
        }
        harness.channels.push(ChannelRecord {
            name: name.to_vec(),
            index,
            value_type,
            callback,
            context: AtomicPtr::new(context),
        });
        sys::SCS_RESULT_OK
    }

    unsafe extern "system" fn fake_unregister_channel(
        name: sys::ScsString,
        index: sys::ScsU32,
        value_type: sys::ScsValueType,
    ) -> sys::ScsResult {
        // SAFETY: The runtime retains the same valid channel name used for the
        // successful registration.
        let name = unsafe { CStr::from_ptr(name) }.to_bytes();
        let mut harness = harness();
        let Some(position) = harness.channels.iter().rposition(|candidate| {
            candidate.name == name && candidate.index == index && candidate.value_type == value_type
        }) else {
            return sys::SCS_RESULT_NOT_FOUND;
        };
        harness.channels.remove(position);
        sys::SCS_RESULT_OK
    }

    fn parameters() -> sys::ScsTelemetryInitParamsV101 {
        sys::ScsTelemetryInitParamsV101 {
            common: sys::ScsSdkInitParamsV100 {
                game_name: c"Euro Truck Simulator 2".as_ptr(),
                game_id: c"eut2".as_ptr(),
                game_version: sys::make_version(1, 56),
                padding: sys::ScsPadding::uninit(),
                log: fake_log,
            },
            register_for_event: fake_register_event,
            unregister_from_event: fake_unregister_event,
            register_for_channel: fake_register_channel,
            unregister_from_channel: fake_unregister_channel,
        }
    }

    #[derive(Default)]
    struct Counts {
        initializes: AtomicUsize,
        channels: AtomicUsize,
        events: AtomicUsize,
        shutdowns: AtomicUsize,
        api_version: AtomicU32,
        speed_bits: AtomicU32,
        callback_subscription_result: AtomicI32,
    }

    struct TestPlugin {
        counts: Arc<Counts>,
    }

    static TEST_SUPPORTED_GAMES: [crate::GameCompatibility; 1] = [crate::GameCompatibility::new(
        Game::EuroTruckSimulator2,
        scs_sdk::GameSchemaVersion::new(1, 0),
    )];

    const fn test_compatibility() -> PluginCompatibility {
        PluginCompatibility::new(TelemetryApiVersion::V1_00, &TEST_SUPPORTED_GAMES)
    }

    impl TelemetryPlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Runtime test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            self.counts.initializes.fetch_add(1, Ordering::Relaxed);
            self.counts
                .api_version
                .store(context.telemetry_api_version().raw(), Ordering::Relaxed);
            context.subscribe_event(Event::Started)?;
            context.subscribe(channels::truck::SPEED)
        }

        fn channel(&mut self, _context: &mut PluginContext<'_>, update: ChannelUpdate<'_>) {
            let Some(speed) = update.value(channels::truck::SPEED) else {
                return;
            };
            self.counts
                .speed_bits
                .store(speed.to_bits(), Ordering::Relaxed);
            self.counts.channels.fetch_add(1, Ordering::Relaxed);
        }

        fn event(&mut self, context: &mut PluginContext<'_>, event: TelemetryEvent<'_>) {
            if matches!(event, TelemetryEvent::Started) {
                self.counts.events.fetch_add(1, Ordering::Relaxed);
                let result = match context.subscribe_event(Event::FrameEnd) {
                    Ok(()) => sys::SCS_RESULT_OK,
                    Err(error) => error.result().code(),
                };
                self.counts
                    .callback_subscription_result
                    .store(result, Ordering::Relaxed);
            }
        }

        fn shutdown(&mut self, _context: &mut PluginContext<'_>) {
            self.counts.shutdowns.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn initialize(runtime: &'static Runtime, counts: &Arc<Counts>) -> sys::ScsResult {
        let parameters = parameters();
        let counts = Arc::clone(counts);
        // SAFETY: The test parameter table and all pointed-to strings remain
        // live for this direct, serialized initialization invocation.
        unsafe {
            runtime.initialize(
                sys::SCS_TELEMETRY_VERSION_CURRENT,
                ptr::from_ref(&parameters).cast(),
                move || Box::new(TestPlugin { counts }),
            )
        }
    }

    #[test]
    fn telemetry_api_negotiation_accepts_only_the_audited_layout() {
        let future = Runtime::api_initialization_error(
            TelemetryApiVersion::new(1, 2),
            SdkError::Unsupported,
        );
        assert_eq!(future.result(), SdkError::Unsupported);
        assert_eq!(
            future.message(),
            "unsupported telemetry API 1.2, supported versions are 1.0, 1.1",
        );
    }

    #[test]
    fn loader_retry_after_an_unsupported_future_api_starts_cleanly() {
        let _serial_guard = serial_guard();
        harness().reset();

        let owner = TestRuntimeOwner::new();
        let runtime = owner.runtime();
        let factory_calls = Arc::new(AtomicUsize::new(0));
        let factory_counter = Arc::clone(&factory_calls);

        // SAFETY: An unknown API must be rejected before the intentionally null
        // parameter pointer is inspected. This models the loader trying a
        // future version before falling back to the newest audited adapter.
        let unsupported = unsafe {
            runtime.initialize(
                TelemetryApiVersion::new(1, 2).raw(),
                ptr::null(),
                move || {
                    factory_counter.fetch_add(1, Ordering::Relaxed);
                    Box::new(EmptyPlugin)
                },
            )
        };
        assert_eq!(unsupported, sys::SCS_RESULT_UNSUPPORTED);
        assert_eq!(factory_calls.load(Ordering::Relaxed), 0);
        assert!(harness().events.is_empty());
        assert!(harness().channels.is_empty());

        assert_eq!(
            initialize_without_counts_at(runtime, TelemetryApiVersion::V1_01, EmptyPlugin,),
            sys::SCS_RESULT_OK
        );
        runtime.shutdown();
        harness().reset();
        drop(owner);

        // API 1.00 remains a directly audited adapter as well, even though a
        // modern loader normally succeeds on 1.01 before reaching it.
        let v100_owner = TestRuntimeOwner::new();
        let v100_runtime = v100_owner.runtime();
        assert_eq!(
            initialize_without_counts_at(v100_runtime, TelemetryApiVersion::V1_00, EmptyPlugin,),
            sys::SCS_RESULT_OK
        );
        v100_runtime.shutdown();

        harness().reset();
        drop(v100_owner);
    }

    #[test]
    fn plugin_compatibility_separates_framework_adapters_from_product_requirements() {
        static ETS2_FROM_1_14: [crate::GameCompatibility; 1] = [crate::GameCompatibility::new(
            Game::EuroTruckSimulator2,
            scs_sdk::GameSchemaVersion::new(1, 14),
        )];
        static DUPLICATE_ETS2: [crate::GameCompatibility; 2] = [
            crate::GameCompatibility::new(
                Game::EuroTruckSimulator2,
                scs_sdk::GameSchemaVersion::new(1, 0),
            ),
            crate::GameCompatibility::new(
                Game::EuroTruckSimulator2,
                scs_sdk::GameSchemaVersion::new(1, 14),
            ),
        ];
        static AMBIGUOUS_OTHER: [crate::GameCompatibility; 1] = [crate::GameCompatibility::new(
            Game::Other,
            scs_sdk::GameSchemaVersion::new(1, 0),
        )];
        static NO_GAMES: [crate::GameCompatibility; 0] = [];

        let current_ets2 = GameInfo::new(
            c"Euro Truck Simulator 2",
            c"eut2",
            scs_sdk::GameSchemaVersion::new(1, 56),
        );
        let requirements = PluginCompatibility::new(TelemetryApiVersion::V1_01, &ETS2_FROM_1_14);
        assert!(
            Runtime::validate_compatibility(
                requirements,
                TelemetryApiVersion::V1_01,
                &current_ets2,
            )
            .is_ok()
        );

        let old_api = Runtime::validate_compatibility(
            requirements,
            TelemetryApiVersion::V1_00,
            &current_ets2,
        )
        .expect_err("product requirements must reject an older negotiated API");
        assert_eq!(old_api.result(), SdkError::Unsupported);

        let unknown_required_api = Runtime::validate_compatibility(
            PluginCompatibility::new(TelemetryApiVersion::new(1, 2), &ETS2_FROM_1_14),
            TelemetryApiVersion::V1_01,
            &current_ets2,
        )
        .expect_err("a product cannot require an API without a framework adapter");
        assert_eq!(unknown_required_api.result(), SdkError::InvalidParameter);

        let old_schema = GameInfo::new(
            c"Euro Truck Simulator 2",
            c"eut2",
            scs_sdk::GameSchemaVersion::new(1, 13),
        );
        let old_schema_error =
            Runtime::validate_compatibility(requirements, TelemetryApiVersion::V1_01, &old_schema)
                .expect_err("schema below the product minimum must be rejected");
        assert_eq!(old_schema_error.result(), SdkError::Unsupported);

        let future_major = GameInfo::new(
            c"Euro Truck Simulator 2",
            c"eut2",
            scs_sdk::GameSchemaVersion::new(2, 0),
        );
        assert_eq!(
            Runtime::validate_compatibility(
                requirements,
                TelemetryApiVersion::V1_01,
                &future_major,
            )
            .expect_err("a future schema major requires explicit review")
            .result(),
            SdkError::Unsupported,
        );

        let ats = GameInfo::new(
            c"American Truck Simulator",
            c"ats",
            scs_sdk::GameSchemaVersion::new(1, 5),
        );
        assert_eq!(
            Runtime::validate_compatibility(requirements, TelemetryApiVersion::V1_01, &ats,)
                .expect_err("undeclared games are not product-compatible")
                .result(),
            SdkError::Unsupported,
        );

        for invalid in [
            PluginCompatibility::new(TelemetryApiVersion::V1_00, &NO_GAMES),
            PluginCompatibility::new(TelemetryApiVersion::V1_00, &DUPLICATE_ETS2),
            PluginCompatibility::new(TelemetryApiVersion::V1_00, &AMBIGUOUS_OTHER),
        ] {
            assert_eq!(
                Runtime::validate_compatibility(
                    invalid,
                    TelemetryApiVersion::V1_01,
                    &current_ets2,
                )
                .expect_err("malformed declarations must fail before product initialization")
                .result(),
                SdkError::InvalidParameter,
            );
        }
    }

    fn started_record(first: bool) -> EventInvocation {
        let harness = harness();
        let mut records = harness
            .events
            .iter()
            .filter(|record| record.event == sys::SCS_TELEMETRY_EVENT_STARTED);
        let record = if first {
            records
                .next()
                .expect("started callback should be registered")
        } else {
            records
                .next_back()
                .expect("started callback should be registered")
        };
        (
            record.event,
            record.callback,
            record.context.load(Ordering::Relaxed),
        )
    }

    fn invoke_event(record: EventInvocation) {
        // SAFETY: The harness captures the callback and its exact stable context
        // from a successful runtime registration.
        unsafe { (record.1)(record.0, ptr::null(), record.2) };
    }

    fn invoke_speed(value: f32) {
        let record = {
            let harness = harness();
            let record = harness
                .channels
                .iter()
                .rfind(|record| record.name == b"truck.speed")
                .expect("speed callback should be registered");
            (
                record.callback,
                record.index,
                record.context.load(Ordering::Relaxed),
            )
        };
        let raw = sys::ScsValue {
            type_: sys::SCS_VALUE_TYPE_FLOAT,
            padding: sys::ScsPadding::uninit(),
            value: sys::ScsValueData {
                value_float: sys::ScsValueFloat { value },
            },
        };
        // SAFETY: The harness captured the callback/context pair from the speed
        // registration and the tagged value remains live for this invocation.
        unsafe { (record.0)(c"truck.speed".as_ptr(), record.1, &raw const raw, record.2) };
    }

    /// Owns a runtime at the stable address required by its callback contexts.
    ///
    /// The production macro uses a process static. This test owner instead
    /// reclaims the allocation after the fake SDK has forgotten every context,
    /// allowing Miri's leak checker to remain enabled.
    struct TestRuntimeOwner {
        pointer: *mut Runtime,
    }

    impl TestRuntimeOwner {
        fn new() -> Self {
            Self {
                pointer: Box::into_raw(Box::new(Runtime::new())),
            }
        }

        fn runtime(&self) -> &'static Runtime {
            // SAFETY: `pointer` came from `Box::into_raw`, remains owned by this
            // value, and is reclaimed only in `Drop` after the test stops using
            // the returned process-lifetime simulation.
            unsafe { &*self.pointer }
        }
    }

    impl Drop for TestRuntimeOwner {
        fn drop(&mut self) {
            // The fake SDK is cleared before each owner is dropped. Retired
            // registrations are therefore no longer foreign-reachable and may
            // be released before the self-referential runtime allocation.
            let runtime = self.runtime();
            let mut state = runtime.lock_state();
            state.retired_events.clear();
            state.retired_channels.clear();
            drop(state);

            // SAFETY: This is the unique pointer produced by `Box::into_raw` in
            // `new`, and all Arc registrations referring to the runtime have
            // been released above.
            unsafe { drop(Box::from_raw(self.pointer)) };
        }
    }

    #[test]
    fn runtime_dispatches_rolls_back_and_rejects_stale_contexts() {
        let _serial_guard = serial_guard();
        harness().reset();
        let runtime_owner = TestRuntimeOwner::new();
        let runtime = runtime_owner.runtime();
        let first = Arc::new(Counts::default());

        assert_eq!(initialize(runtime, &first), sys::SCS_RESULT_OK);
        assert_eq!(first.initializes.load(Ordering::Relaxed), 1);
        assert_eq!(
            first.api_version.load(Ordering::Relaxed),
            sys::SCS_TELEMETRY_VERSION_CURRENT,
        );
        assert_eq!(harness().events.len(), 1);
        assert_eq!(harness().channels.len(), 1);
        assert_eq!(
            harness().logs,
            vec![
                (
                    sys::SCS_LOG_TYPE_MESSAGE,
                    format!(
                        concat!(
                            "[scs-sdk-plugin] starting plugin ",
                            "name=\"Runtime test plugin\" version=\"0.0.0-test\" ",
                            "framework_version=\"{}\""
                        ),
                        env!("CARGO_PKG_VERSION"),
                    ),
                ),
                (
                    sys::SCS_LOG_TYPE_MESSAGE,
                    concat!(
                        "[scs-sdk-plugin] detected ",
                        "game_display_name=\"Euro Truck Simulator 2\" game_id=\"eut2\" ",
                        "telemetry_api=1.1 telemetry_schema=1.56"
                    )
                    .to_owned(),
                ),
                (
                    sys::SCS_LOG_TYPE_MESSAGE,
                    concat!(
                        "[scs-sdk-plugin] initialized plugin ",
                        "name=\"Runtime test plugin\" version=\"0.0.0-test\" ",
                        "events=1 channels=1"
                    )
                    .to_owned(),
                ),
            ],
        );
        invoke_speed(27.5);
        invoke_event(started_record(true));
        assert_eq!(first.speed_bits.load(Ordering::Relaxed), 27.5_f32.to_bits());
        assert_eq!(first.channels.load(Ordering::Relaxed), 1);
        assert_eq!(first.events.load(Ordering::Relaxed), 1);
        assert_eq!(
            first.callback_subscription_result.load(Ordering::Relaxed),
            sys::SCS_RESULT_NOT_NOW
        );

        let stale_started = started_record(true);
        harness().fail_event_unregistration = true;
        runtime.shutdown();
        assert_eq!(first.shutdowns.load(Ordering::Relaxed), 1);
        assert!(harness().channels.is_empty());
        assert!(harness().logs.iter().any(|(_, message)| {
            message
                == concat!(
                    "[scs-sdk-plugin] shutdown complete plugin ",
                    "name=\"Runtime test plugin\" version=\"0.0.0-test\""
                )
        }));

        harness().fail_event_unregistration = false;
        let second = Arc::new(Counts::default());
        assert_eq!(initialize(runtime, &second), sys::SCS_RESULT_OK);
        invoke_event(stale_started);
        assert_eq!(second.events.load(Ordering::Relaxed), 0);
        invoke_event(started_record(false));
        assert_eq!(second.events.load(Ordering::Relaxed), 1);
        runtime.shutdown();

        harness().reset();
        drop(runtime_owner);

        harness().fail_channel_registration = true;
        let failing_runtime_owner = TestRuntimeOwner::new();
        let failing_runtime = failing_runtime_owner.runtime();
        let failing = Arc::new(Counts::default());
        assert_eq!(
            initialize(failing_runtime, &failing),
            sys::SCS_RESULT_UNSUPPORTED_TYPE
        );
        assert_eq!(failing.shutdowns.load(Ordering::Relaxed), 1);
        assert!(harness().events.is_empty());
        assert!(harness().channels.is_empty());
        harness().reset();
        drop(failing_runtime_owner);
    }

    struct EmptyPlugin;

    impl TelemetryPlugin for EmptyPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Empty test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, _context: &mut PluginContext<'_>) -> PluginResult {
            Ok(())
        }
    }

    struct InvalidMetadataPlugin;

    impl TelemetryPlugin for InvalidMetadataPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("", "")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, _context: &mut PluginContext<'_>) -> PluginResult {
            panic!("invalid metadata must be rejected before product initialization");
        }
    }

    struct DuplicateEventPlugin;

    impl TelemetryPlugin for DuplicateEventPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Duplicate event test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe_event(Event::Started)?;
            context.subscribe_event(Event::Started)
        }
    }

    struct GameplayOnV100Plugin;

    impl TelemetryPlugin for GameplayOnV100Plugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Gameplay API guard test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe_event(Event::Gameplay)
        }
    }

    struct Signed64OnV100Plugin;

    impl TelemetryPlugin for Signed64OnV100Plugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Signed 64-bit API guard test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe(channels::truck::SPEED.requesting::<i64>())
        }
    }

    struct OptionalChannelPlugin;

    impl TelemetryPlugin for OptionalChannelPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Optional channel transaction test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe(channels::truck::SPEED)?;
            context.subscribe_optional(channels::truck::ENGINE_RPM)?;
            context.subscribe(channels::truck::ENGINE_GEAR)?;
            context.subscribe_optional(channels::truck::SPEED.requesting::<i64>())
        }
    }

    struct OptionalGameplayPlugin;

    impl TelemetryPlugin for OptionalGameplayPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Optional gameplay event test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe_event(Event::Started)?;
            context.subscribe_event_optional(Event::Gameplay)?;
            context.subscribe_event(Event::FrameEnd)
        }
    }

    struct RequiredNavigationSchemaPlugin;

    impl TelemetryPlugin for RequiredNavigationSchemaPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Required navigation schema test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe(channels::truck::NAVIGATION_DISTANCE)
        }
    }

    struct RequiredGameplaySchemaPlugin;

    impl TelemetryPlugin for RequiredGameplaySchemaPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Required gameplay schema test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe_event(Event::Gameplay)
        }
    }

    struct RequiredMultiTrailerSchemaPlugin;

    impl TelemetryPlugin for RequiredMultiTrailerSchemaPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Required multi-trailer schema test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe_trailer(channels::trailer::CONNECTED, scs_sdk::TrailerIndex::ZERO)
        }
    }

    struct OptionalSchemaCapabilitiesPlugin;

    impl TelemetryPlugin for OptionalSchemaCapabilitiesPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("Optional schema capabilities test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            context.subscribe_event_optional(Event::Gameplay)?;
            context.subscribe_optional(channels::truck::NAVIGATION_DISTANCE)?;
            context.subscribe_trailer_optional(
                channels::trailer::CONNECTED,
                scs_sdk::TrailerIndex::ZERO,
            )?;
            context.subscribe(channels::truck::SPEED)
        }
    }

    static ATS_TEST_SUPPORTED_GAMES: [crate::GameCompatibility; 1] =
        [crate::GameCompatibility::new(
            Game::AmericanTruckSimulator,
            scs_sdk::game::ats::V1_00,
        )];

    const fn ats_test_compatibility() -> PluginCompatibility {
        PluginCompatibility::new(TelemetryApiVersion::V1_00, &ATS_TEST_SUPPORTED_GAMES)
    }

    struct AtsAdbluePlugin {
        requirement: crate::SubscriptionRequirement,
    }

    impl TelemetryPlugin for AtsAdbluePlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new("ATS AdBlue availability test plugin", "0.0.0-test")
        }

        fn compatibility(&self) -> PluginCompatibility {
            ats_test_compatibility()
        }

        fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
            match self.requirement {
                crate::SubscriptionRequirement::Required => {
                    context.subscribe(channels::truck::ADBLUE)?;
                }
                crate::SubscriptionRequirement::Optional => {
                    context.subscribe_optional(channels::truck::ADBLUE)?;
                }
            }
            context.subscribe(channels::truck::SPEED)
        }
    }

    fn initialize_without_counts<P>(runtime: &'static Runtime, plugin: P) -> sys::ScsResult
    where
        P: TelemetryPlugin,
    {
        initialize_without_counts_at(runtime, TelemetryApiVersion::CURRENT, plugin)
    }

    fn initialize_without_counts_at<P>(
        runtime: &'static Runtime,
        api_version: TelemetryApiVersion,
        plugin: P,
    ) -> sys::ScsResult
    where
        P: TelemetryPlugin,
    {
        initialize_without_counts_for_game(
            runtime,
            api_version,
            c"Euro Truck Simulator 2",
            c"eut2",
            scs_sdk::GameSchemaVersion::new(1, 56),
            plugin,
        )
    }

    fn initialize_without_counts_for_game<P>(
        runtime: &'static Runtime,
        api_version: TelemetryApiVersion,
        game_name: &'static CStr,
        game_id: &'static CStr,
        game_schema: scs_sdk::GameSchemaVersion,
        plugin: P,
    ) -> sys::ScsResult
    where
        P: TelemetryPlugin,
    {
        let mut parameters = parameters();
        parameters.common.game_name = game_name.as_ptr();
        parameters.common.game_id = game_id.as_ptr();
        parameters.common.game_version = game_schema.raw();
        // SAFETY: The fake parameter structure remains live throughout this
        // direct initialization call, and the runtime owner provides a stable
        // address for every context created by the framework.
        unsafe {
            runtime.initialize(
                api_version.raw(),
                ptr::from_ref(&parameters).cast(),
                move || Box::new(plugin),
            )
        }
    }

    #[test]
    fn event_registration_is_explicit_and_rejects_duplicates() {
        let _serial_guard = serial_guard();
        harness().reset();

        let empty_owner = TestRuntimeOwner::new();
        let empty_runtime = empty_owner.runtime();
        assert_eq!(
            initialize_without_counts(empty_runtime, EmptyPlugin),
            sys::SCS_RESULT_OK
        );
        assert!(harness().events.is_empty());
        assert!(harness().channels.is_empty());
        empty_runtime.shutdown();
        harness().reset();
        drop(empty_owner);

        let invalid_metadata_owner = TestRuntimeOwner::new();
        let invalid_metadata_runtime = invalid_metadata_owner.runtime();
        assert_eq!(
            initialize_without_counts(invalid_metadata_runtime, InvalidMetadataPlugin),
            sys::SCS_RESULT_INVALID_PARAMETER,
        );
        assert!(harness().events.is_empty());
        assert!(harness().channels.is_empty());
        assert!(harness().logs.iter().any(|(level, message)| {
            *level == sys::SCS_LOG_TYPE_ERROR
                && message
                    == concat!(
                        "plugin metadata is invalid: ",
                        "plugin metadata name and version must both be non-empty"
                    )
        }));
        harness().reset();
        drop(invalid_metadata_owner);

        let duplicate_owner = TestRuntimeOwner::new();
        assert_eq!(
            initialize_without_counts(duplicate_owner.runtime(), DuplicateEventPlugin),
            sys::SCS_RESULT_ALREADY_REGISTERED
        );
        assert!(harness().events.is_empty());
        assert!(harness().channels.is_empty());
        harness().reset();
        drop(duplicate_owner);
    }

    #[test]
    fn gameplay_subscription_requires_telemetry_api_v101() {
        let _serial_guard = serial_guard();
        harness().reset();
        let runtime_owner = TestRuntimeOwner::new();
        let result = initialize_without_counts_at(
            runtime_owner.runtime(),
            TelemetryApiVersion::V1_00,
            GameplayOnV100Plugin,
        );

        assert_eq!(result, sys::SCS_RESULT_UNSUPPORTED);
        assert!(harness().events.is_empty());
        assert!(harness().logs.iter().any(|(_, message)| {
            message.contains("event Gameplay requires telemetry API 1.1, negotiated 1.0")
        }));

        harness().reset();
        drop(runtime_owner);
    }

    #[test]
    fn signed_64_bit_channel_requests_require_telemetry_api_v101() {
        let _serial_guard = serial_guard();
        harness().reset();

        let v100_owner = TestRuntimeOwner::new();
        let result = initialize_without_counts_at(
            v100_owner.runtime(),
            TelemetryApiVersion::V1_00,
            Signed64OnV100Plugin,
        );
        assert_eq!(result, sys::SCS_RESULT_UNSUPPORTED);
        assert!(harness().channels.is_empty());
        assert!(harness().logs.iter().any(|(_, message)| {
            message.contains("requests I64, which requires telemetry API 1.1; negotiated 1.0")
        }));
        harness().reset();
        drop(v100_owner);

        let v101_owner = TestRuntimeOwner::new();
        let v101_runtime = v101_owner.runtime();
        assert_eq!(
            initialize_without_counts_at(
                v101_runtime,
                TelemetryApiVersion::V1_01,
                Signed64OnV100Plugin,
            ),
            sys::SCS_RESULT_OK
        );
        assert_eq!(harness().channels.len(), 1);
        assert_eq!(harness().channels[0].value_type, sys::SCS_VALUE_TYPE_S64);
        v101_runtime.shutdown();
        assert!(harness().channels.is_empty());

        harness().reset();
        drop(v101_owner);
    }

    #[test]
    fn descriptor_schema_history_gates_required_and_optional_registrations() {
        let _serial_guard = serial_guard();

        for plugin in [0_u8, 1_u8, 2_u8] {
            harness().reset();
            let owner = TestRuntimeOwner::new();
            let result = match plugin {
                0 => initialize_without_counts_for_game(
                    owner.runtime(),
                    TelemetryApiVersion::V1_01,
                    c"Euro Truck Simulator 2",
                    c"eut2",
                    scs_sdk::game::ets2::V1_11,
                    RequiredNavigationSchemaPlugin,
                ),
                1 => initialize_without_counts_for_game(
                    owner.runtime(),
                    TelemetryApiVersion::V1_01,
                    c"Euro Truck Simulator 2",
                    c"eut2",
                    scs_sdk::game::ets2::V1_13,
                    RequiredGameplaySchemaPlugin,
                ),
                _ => initialize_without_counts_for_game(
                    owner.runtime(),
                    TelemetryApiVersion::V1_01,
                    c"Euro Truck Simulator 2",
                    c"eut2",
                    scs_sdk::game::ets2::V1_13,
                    RequiredMultiTrailerSchemaPlugin,
                ),
            };
            assert_eq!(result, sys::SCS_RESULT_UNSUPPORTED);
            assert!(harness().events.is_empty());
            assert!(harness().channels.is_empty());
            harness().reset();
            drop(owner);
        }

        let optional_owner = TestRuntimeOwner::new();
        let optional_runtime = optional_owner.runtime();
        assert_eq!(
            initialize_without_counts_for_game(
                optional_runtime,
                TelemetryApiVersion::V1_01,
                c"Euro Truck Simulator 2",
                c"eut2",
                scs_sdk::game::ets2::V1_11,
                OptionalSchemaCapabilitiesPlugin,
            ),
            sys::SCS_RESULT_OK,
        );
        assert!(harness().events.is_empty());
        assert_eq!(harness().channels.len(), 1);
        assert_eq!(harness().channels[0].name, b"truck.speed");
        optional_runtime.shutdown();
        assert!(harness().channels.is_empty());
        harness().reset();
        drop(optional_owner);

        let required_ats_owner = TestRuntimeOwner::new();
        assert_eq!(
            initialize_without_counts_for_game(
                required_ats_owner.runtime(),
                TelemetryApiVersion::V1_01,
                c"American Truck Simulator",
                c"ats",
                scs_sdk::game::ats::V1_05,
                AtsAdbluePlugin {
                    requirement: crate::SubscriptionRequirement::Required,
                },
            ),
            sys::SCS_RESULT_UNSUPPORTED,
        );
        assert!(harness().channels.is_empty());
        harness().reset();
        drop(required_ats_owner);

        let optional_ats_owner = TestRuntimeOwner::new();
        let optional_ats_runtime = optional_ats_owner.runtime();
        assert_eq!(
            initialize_without_counts_for_game(
                optional_ats_runtime,
                TelemetryApiVersion::V1_01,
                c"American Truck Simulator",
                c"ats",
                scs_sdk::game::ats::V1_05,
                AtsAdbluePlugin {
                    requirement: crate::SubscriptionRequirement::Optional,
                },
            ),
            sys::SCS_RESULT_OK,
        );
        assert_eq!(harness().channels.len(), 1);
        assert_eq!(harness().channels[0].name, b"truck.speed");
        optional_ats_runtime.shutdown();
        harness().reset();
        drop(optional_ats_owner);
    }

    #[test]
    fn optional_channels_skip_only_expected_capability_failures() {
        let _serial_guard = serial_guard();

        for tolerated in [sys::SCS_RESULT_NOT_FOUND, sys::SCS_RESULT_UNSUPPORTED_TYPE] {
            harness().reset();
            harness().selective_channel_failure = Some((b"truck.engine.rpm".to_vec(), tolerated));

            let owner = TestRuntimeOwner::new();
            let runtime = owner.runtime();
            assert_eq!(
                initialize_without_counts_at(
                    runtime,
                    TelemetryApiVersion::V1_00,
                    OptionalChannelPlugin,
                ),
                sys::SCS_RESULT_OK
            );

            // Both required channels commit across the missing optional RPM
            // declaration. The optional i64 request is skipped locally because
            // API 1.00 predates that representation.
            assert_eq!(harness().channels.len(), 2);
            assert_eq!(harness().channels[0].name, b"truck.speed");
            assert_eq!(harness().channels[0].value_type, sys::SCS_VALUE_TYPE_FLOAT);
            assert_eq!(harness().channels[1].name, b"truck.engine.gear");
            assert_eq!(harness().channels[1].value_type, sys::SCS_VALUE_TYPE_S32);

            runtime.shutdown();
            assert!(harness().channels.is_empty());
            harness().reset();
            drop(owner);
        }

        harness().reset();
        harness().selective_channel_failure =
            Some((b"truck.engine.rpm".to_vec(), sys::SCS_RESULT_GENERIC_ERROR));
        let owner = TestRuntimeOwner::new();
        assert_eq!(
            initialize_without_counts_at(
                owner.runtime(),
                TelemetryApiVersion::V1_00,
                OptionalChannelPlugin,
            ),
            sys::SCS_RESULT_GENERIC_ERROR
        );
        assert!(harness().channels.is_empty());

        harness().reset();
        drop(owner);
    }

    #[test]
    fn optional_events_skip_api_and_expected_registration_absence() {
        let _serial_guard = serial_guard();

        harness().reset();
        let v100_owner = TestRuntimeOwner::new();
        let v100_runtime = v100_owner.runtime();
        assert_eq!(
            initialize_without_counts_at(
                v100_runtime,
                TelemetryApiVersion::V1_00,
                OptionalGameplayPlugin,
            ),
            sys::SCS_RESULT_OK
        );
        assert_eq!(harness().events.len(), 2);
        assert_eq!(harness().events[0].event, Event::Started.raw());
        assert_eq!(harness().events[1].event, Event::FrameEnd.raw());
        v100_runtime.shutdown();
        harness().reset();
        drop(v100_owner);

        for tolerated in [sys::SCS_RESULT_UNSUPPORTED, sys::SCS_RESULT_NOT_FOUND] {
            harness().reset();
            harness().selective_event_failure = Some((Event::Gameplay.raw(), tolerated));
            let owner = TestRuntimeOwner::new();
            let runtime = owner.runtime();
            assert_eq!(
                initialize_without_counts_at(
                    runtime,
                    TelemetryApiVersion::V1_01,
                    OptionalGameplayPlugin,
                ),
                sys::SCS_RESULT_OK
            );
            assert_eq!(harness().events.len(), 2);
            assert_eq!(harness().events[0].event, Event::Started.raw());
            assert_eq!(harness().events[1].event, Event::FrameEnd.raw());
            runtime.shutdown();
            harness().reset();
            drop(owner);
        }

        harness().reset();
        harness().selective_event_failure =
            Some((Event::Gameplay.raw(), sys::SCS_RESULT_GENERIC_ERROR));
        let owner = TestRuntimeOwner::new();
        assert_eq!(
            initialize_without_counts_at(
                owner.runtime(),
                TelemetryApiVersion::V1_01,
                OptionalGameplayPlugin,
            ),
            sys::SCS_RESULT_GENERIC_ERROR
        );
        assert!(harness().events.is_empty());
        harness().reset();
        drop(owner);
    }
}
