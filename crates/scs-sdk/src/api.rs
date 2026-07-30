use core::ffi::{CStr, c_void};
use core::marker::PhantomData;

use crate::{Event, GameSchemaVersion, SdkError, SdkResult, TelemetryApiVersion, sys};

/// An inert handle to the game's logger.
///
/// The handle can be stored as part of a plugin session, but it deliberately
/// has no logging methods. Logging is only available through [`SdkCall`], which
/// is scoped to one direct call from the game.
#[derive(Clone, Copy)]
pub(crate) struct LoggerHandle(sys::ScsLog);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum LogLevel {
    Message = sys::SCS_LOG_TYPE_MESSAGE,
    Warning = sys::SCS_LOG_TYPE_WARNING,
    Error = sys::SCS_LOG_TYPE_ERROR,
}

#[derive(Clone, Copy)]
pub(crate) struct ApiTable {
    pub(crate) version: TelemetryApiVersion,
    pub(crate) logger: LoggerHandle,
    pub(crate) register_for_event: sys::ScsTelemetryRegisterForEvent,
    pub(crate) unregister_from_event: sys::ScsTelemetryUnregisterFromEvent,
    pub(crate) register_for_channel: sys::ScsTelemetryRegisterForChannel,
    pub(crate) unregister_from_channel: sys::ScsTelemetryUnregisterFromChannel,
}

impl ApiTable {
    const fn from_raw(version: TelemetryApiVersion, raw: &sys::ScsTelemetryInitParamsV101) -> Self {
        Self {
            version,
            logger: LoggerHandle(raw.common.log),
            register_for_event: raw.register_for_event,
            unregister_from_event: raw.unregister_from_event,
            register_for_channel: raw.register_for_channel,
            unregister_from_channel: raw.unregister_from_channel,
        }
    }
}

/// A typed view over the initialization parameters supplied by SCS.
pub struct TelemetryApi<'a> {
    raw: &'a sys::ScsTelemetryInitParamsV101,
    table: ApiTable,
    // The SDK API is main-thread-only. Do not allow this view to cross threads.
    not_send_sync: PhantomData<*mut ()>,
}

/// A copyable, inert session handle used by callbacks after initialization.
///
/// Calling [`TelemetrySession::with_call`] is unsafe because the caller must
/// prove that the current stack is executing as a direct call from SCS on the
/// game's main thread.
///
/// The scoped capability cannot be returned from the higher-ranked closure:
///
/// ```compile_fail
/// use scs_sdk::{SdkCall, TelemetrySession};
///
/// fn leak(session: TelemetrySession) -> &'static SdkCall<'static> {
///     unsafe { session.with_call(|call| call) }
/// }
/// ```
#[derive(Clone, Copy)]
pub struct TelemetrySession {
    pub(crate) table: ApiTable,
}

/// Capabilities available during one direct call from the game.
///
/// The lifetime is higher-ranked by the constructors in this module, so a
/// scope cannot be returned, stored, or moved into another thread by safe code.
///
/// `SdkCall` is deliberately neither `Send` nor `Sync`:
///
/// ```compile_fail
/// use scs_sdk::SdkCall;
///
/// fn assert_send<T: Send>() {}
/// fn assert_sync<T: Sync>() {}
///
/// assert_send::<SdkCall<'static>>();
/// assert_sync::<SdkCall<'static>>();
/// ```
pub struct SdkCall<'scope> {
    pub(crate) table: ApiTable,
    scope: PhantomData<&'scope mut ()>,
    not_send_sync: PhantomData<*mut ()>,
}

/// Logger access tied to the current [`SdkCall`] scope.
#[derive(Clone, Copy)]
pub struct ScopedLogger<'scope> {
    raw: sys::ScsLog,
    scope: PhantomData<&'scope SdkCall<'scope>>,
}

impl ScopedLogger<'_> {
    pub(crate) const fn from_raw(raw: sys::ScsLog) -> Self {
        Self {
            raw,
            scope: PhantomData,
        }
    }
}

/// Initialization-parameter layout selected for one audited API version.
///
/// SCS SDK 1.14 defines separate names for the 1.00 and 1.01 parameter types,
/// but the latter is a typedef of the former. Keeping the layout selection as
/// an explicit internal enum gives a future API version a mandatory review
/// point: it must either reuse an audited layout deliberately or add a new
/// match arm with its own structure before any foreign pointer is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TelemetryInitLayout {
    V100,
}

/// One exact mapping from an accepted API version to its audited foreign
/// initialization layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TelemetryApiAdapter {
    version: TelemetryApiVersion,
    layout: TelemetryInitLayout,
}

/// Sole version-support table for the typed wrapper.
///
/// Version discovery and layout selection both derive from these entries. A
/// future API therefore cannot be advertised without simultaneously choosing
/// an audited adapter, and runtime code never carries a parallel whitelist.
const TELEMETRY_API_ADAPTERS: [TelemetryApiAdapter; 2] = [
    TelemetryApiAdapter {
        version: TelemetryApiVersion::V1_00,
        layout: TelemetryInitLayout::V100,
    },
    TelemetryApiAdapter {
        version: TelemetryApiVersion::V1_01,
        layout: TelemetryInitLayout::V100,
    },
];

const fn adapter_versions() -> [TelemetryApiVersion; TELEMETRY_API_ADAPTERS.len()] {
    let mut versions = [TelemetryApiVersion::from_raw(0); TELEMETRY_API_ADAPTERS.len()];
    let mut index = 0;
    while index < TELEMETRY_API_ADAPTERS.len() {
        versions[index] = TELEMETRY_API_ADAPTERS[index].version;
        index += 1;
    }
    versions
}

const SUPPORTED_TELEMETRY_API_VERSIONS: [TelemetryApiVersion; TELEMETRY_API_ADAPTERS.len()] =
    adapter_versions();

impl<'a> TelemetryApi<'a> {
    /// Telemetry API versions with initialization layouts audited by this crate.
    ///
    /// This list describes wrapper capability, not a product plugin's minimum
    /// requirements. A framework may understand API 1.00 while a particular
    /// plugin still requires API 1.01 for gameplay events or signed 64-bit
    /// values.
    pub const SUPPORTED_VERSIONS: &'static [TelemetryApiVersion] =
        &SUPPORTED_TELEMETRY_API_VERSIONS;

    /// Returns whether this wrapper has an audited initialization adapter for
    /// the supplied version.
    ///
    /// The check is an exact whitelist. It deliberately does not accept an
    /// unknown minor version merely because SCS normally uses minor increments
    /// for additive changes: the initialization structure still needs to be
    /// compared with the new official header first.
    #[must_use]
    pub const fn supports_version(version: TelemetryApiVersion) -> bool {
        Self::init_layout(version).is_some()
    }

    const fn init_layout(version: TelemetryApiVersion) -> Option<TelemetryInitLayout> {
        let mut index = 0;
        while index < TELEMETRY_API_ADAPTERS.len() {
            let adapter = TELEMETRY_API_ADAPTERS[index];
            if adapter.version.raw() == version.raw() {
                return Some(adapter.layout);
            }
            index += 1;
        }
        None
    }

    /// Creates a typed view over the initialization parameters supplied by SCS.
    ///
    /// # Safety
    ///
    /// For a supported `version`, `params` must point to the live matching
    /// telemetry initialization structure supplied by the game. An unsupported
    /// version is rejected before `params` is inspected. The returned view may
    /// only be used synchronously during the direct initialization call from
    /// SCS, on the game's main thread. The pointed-to function table and strings
    /// must remain valid for that use.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::Unsupported`] before reading `params` when `version`
    /// has no audited adapter. Returns [`SdkError::InvalidParameter`] when the
    /// version is supported but `params` is null.
    pub unsafe fn from_raw(
        version: TelemetryApiVersion,
        params: *const sys::ScsTelemetryInitParams,
    ) -> SdkResult<Self> {
        // Select an exact audited layout before inspecting params. Both
        // versions currently select V100 because the SDK declares V101 as its
        // typedef. A future layout gets a separate arm rather than falling
        // through a numeric-range compatibility guess.
        let Some(layout) = Self::init_layout(version) else {
            return Err(SdkError::Unsupported);
        };
        let raw = match layout {
            TelemetryInitLayout::V100 => {
                // SAFETY: The caller guarantees that params matches the
                // supported version; init_layout establishes that V100 is the
                // audited concrete structure for that version.
                unsafe { params.cast::<sys::ScsTelemetryInitParamsV100>().as_ref() }
                    .ok_or(SdkError::InvalidParameter)?
            }
        };
        Ok(Self {
            raw,
            table: ApiTable::from_raw(version, raw),
            not_send_sync: PhantomData,
        })
    }

    /// Telemetry API version represented by this initialized view.
    #[must_use]
    pub const fn version(&self) -> TelemetryApiVersion {
        self.table.version
    }

    #[must_use]
    pub const fn raw(&self) -> &sys::ScsTelemetryInitParamsV101 {
        self.raw
    }

    #[must_use]
    pub fn game_name(&self) -> &'a CStr {
        // SAFETY: SCS guarantees a non-null, NUL-terminated string for the
        // duration of the initialization call.
        unsafe { CStr::from_ptr(self.raw.common.game_name) }
    }

    #[must_use]
    pub fn game_id(&self) -> &'a CStr {
        // SAFETY: SCS guarantees a non-null, NUL-terminated string for the
        // duration of the initialization call.
        unsafe { CStr::from_ptr(self.raw.common.game_id) }
    }

    #[must_use]
    pub const fn game_schema_version(&self) -> GameSchemaVersion {
        GameSchemaVersion::from_raw(self.raw.common.game_version)
    }

    /// Returns an inert session handle for later callback entry points.
    #[must_use]
    pub const fn session(&self) -> TelemetrySession {
        TelemetrySession { table: self.table }
    }

    /// Executes an operation with the capabilities valid during initialization.
    ///
    /// The higher-ranked lifetime prevents the operation from returning or
    /// storing the scoped capability.
    pub fn with_call<R>(&self, operation: impl for<'scope> FnOnce(&SdkCall<'scope>) -> R) -> R {
        let call = SdkCall {
            table: self.table,
            scope: PhantomData,
            not_send_sync: PhantomData,
        };
        operation(&call)
    }
}

impl TelemetrySession {
    /// Executes an operation with the capabilities valid during a direct SCS
    /// callback, initialization call, or shutdown call.
    ///
    /// # Safety
    ///
    /// The caller must be executing synchronously on the game's main thread as
    /// a direct call from SCS, while the telemetry session is active. The
    /// operation must not let the scoped capability escape its closure and must
    /// not unwind across the SDK ABI boundary.
    pub unsafe fn with_call<R>(
        self,
        operation: impl for<'scope> FnOnce(&SdkCall<'scope>) -> R,
    ) -> R {
        let call = SdkCall {
            table: self.table,
            scope: PhantomData,
            not_send_sync: PhantomData,
        };
        operation(&call)
    }
}

impl SdkCall<'_> {
    /// Telemetry API version negotiated for the current plugin session.
    ///
    /// The value travels with the copied function table, so upper layers do
    /// not need a second runtime field which could drift from the adapter that
    /// produced this call capability.
    #[must_use]
    pub const fn telemetry_api_version(&self) -> TelemetryApiVersion {
        self.table.version
    }

    #[must_use]
    pub const fn logger(&self) -> ScopedLogger<'_> {
        ScopedLogger::from_raw(self.table.logger.0)
    }

    /// Registers an SDK event callback.
    ///
    /// # Safety
    ///
    /// `callback` must implement the exact SDK ABI, must uphold the validity
    /// requirements for the event-specific `event_info` pointer, and must not
    /// unwind across the ABI boundary. `context` must remain valid for every
    /// callback invocation. This method must be called from initialization or
    /// an SCS event callback other than the callback for `event`, never from a
    /// worker thread or a channel callback.
    ///
    /// # Errors
    ///
    /// Returns the result reported by the game's registration function.
    pub unsafe fn register_event(
        &self,
        event: Event,
        callback: sys::ScsTelemetryEventCallback,
        context: *mut c_void,
    ) -> SdkResult {
        // SAFETY: The caller upholds the SDK callback and context contract.
        let result = unsafe { (self.table.register_for_event)(event.raw(), callback, context) };
        SdkError::from_code(result)
    }

    /// Unregisters an SDK event callback.
    ///
    /// # Safety
    ///
    /// Must be called from initialization, shutdown, or an SCS event callback.
    /// Context storage associated with the old registration must remain alive
    /// until this function succeeds and no invocation of that callback is active.
    ///
    /// # Errors
    ///
    /// Returns the result reported by the game's unregistration function.
    pub unsafe fn unregister_event(&self, event: Event) -> SdkResult {
        // SAFETY: The caller upholds the SDK call-site restriction.
        let result = unsafe { (self.table.unregister_from_event)(event.raw()) };
        SdkError::from_code(result)
    }
}

impl ScopedLogger<'_> {
    pub fn log(self, level: LogLevel, message: &CStr) {
        // SAFETY: The scope proves that this call is made during a direct SDK
        // invocation, and the C string remains alive for the duration of it.
        unsafe { (self.raw)(level as sys::ScsLogType, message.as_ptr()) };
    }

    pub fn message(self, message: &CStr) {
        self.log(LogLevel::Message, message);
    }

    pub fn warning(self, message: &CStr) {
        self.log(LogLevel::Warning, message);
    }

    pub fn error(self, message: &CStr) {
        self.log(LogLevel::Error, message);
    }
}

#[cfg(test)]
mod tests {
    use core::ffi::c_void;
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static LOG_CALLS: AtomicUsize = AtomicUsize::new(0);
    static EVENT_REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "system" fn fake_log(_level: sys::ScsLogType, _message: sys::ScsString) {
        LOG_CALLS.fetch_add(1, Ordering::Relaxed);
    }

    unsafe extern "system" fn fake_event_callback(
        _event: sys::ScsEvent,
        _event_info: *const c_void,
        _context: sys::ScsContext,
    ) {
    }

    unsafe extern "system" fn fake_register_event(
        _event: sys::ScsEvent,
        _callback: sys::ScsTelemetryEventCallback,
        _context: sys::ScsContext,
    ) -> sys::ScsResult {
        EVENT_REGISTRATIONS.fetch_add(1, Ordering::Relaxed);
        sys::SCS_RESULT_OK
    }

    unsafe extern "system" fn fake_unregister_event(_event: sys::ScsEvent) -> sys::ScsResult {
        sys::SCS_RESULT_OK
    }

    unsafe extern "system" fn fake_register_channel(
        _name: sys::ScsString,
        _index: sys::ScsU32,
        _type: sys::ScsValueType,
        _flags: sys::ScsU32,
        _callback: sys::ScsTelemetryChannelCallback,
        _context: sys::ScsContext,
    ) -> sys::ScsResult {
        sys::SCS_RESULT_OK
    }

    unsafe extern "system" fn fake_unregister_channel(
        _name: sys::ScsString,
        _index: sys::ScsU32,
        _type: sys::ScsValueType,
    ) -> sys::ScsResult {
        sys::SCS_RESULT_OK
    }

    fn parameters() -> sys::ScsTelemetryInitParamsV101 {
        sys::ScsTelemetryInitParamsV101 {
            common: sys::ScsSdkInitParamsV100 {
                game_name: c"Euro Truck Simulator 2".as_ptr(),
                game_id: c"eut2".as_ptr(),
                game_version: 0x0001_003c,
                padding: sys::ScsPadding::uninit(),
                log: fake_log,
            },
            register_for_event: fake_register_event,
            unregister_from_event: fake_unregister_event,
            register_for_channel: fake_register_channel,
            unregister_from_channel: fake_unregister_channel,
        }
    }

    #[test]
    fn logging_and_registration_require_a_scoped_call() {
        LOG_CALLS.store(0, Ordering::Relaxed);
        EVENT_REGISTRATIONS.store(0, Ordering::Relaxed);
        let parameters = parameters();
        let pointer = (&raw const parameters).cast::<sys::ScsTelemetryInitParams>();
        // SAFETY: `pointer` comes from the live, correctly aligned V1.01
        // fixture above, whose strings and function table outlive `api`.
        let api = unsafe { TelemetryApi::from_raw(TelemetryApiVersion::V1_01, pointer) }
            .expect("valid function table");

        assert_eq!(api.game_id(), c"eut2");
        assert_eq!(api.game_schema_version(), GameSchemaVersion::new(1, 60));
        api.with_call(|call| {
            call.logger().message(c"initializing");
            // SAFETY: The fixture callback has the exact SDK ABI, contains no
            // unwinding path, and the null context is never dereferenced.
            unsafe {
                call.register_event(Event::Started, fake_event_callback, core::ptr::null_mut())
            }
            .expect("event registration");
        });

        let session = api.session();
        // SAFETY: This synthetic call models a direct main-thread callback
        // while `api` and its function table remain live, and the closure does
        // not let the scoped call capability escape.
        unsafe {
            session.with_call(|call| call.logger().message(c"callback"));
        }

        assert_eq!(LOG_CALLS.load(Ordering::Relaxed), 2);
        assert_eq!(EVENT_REGISTRATIONS.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn unaudited_api_version_is_rejected_before_reading_parameters() {
        assert_eq!(
            TelemetryApi::SUPPORTED_VERSIONS,
            &[TelemetryApiVersion::V1_00, TelemetryApiVersion::V1_01],
        );
        assert!(TelemetryApi::supports_version(TelemetryApiVersion::V1_00));
        assert!(TelemetryApi::supports_version(TelemetryApiVersion::V1_01));
        assert!(!TelemetryApi::supports_version(TelemetryApiVersion::new(
            1, 2
        )));

        // SAFETY: An unsupported version must be rejected before the null
        // parameter pointer is inspected; this call exercises that contract.
        let result = unsafe {
            TelemetryApi::from_raw(
                TelemetryApiVersion::new(1, 2),
                core::ptr::null::<sys::ScsTelemetryInitParams>(),
            )
        };

        assert!(matches!(result, Err(SdkError::Unsupported)));
    }
}
