//! Real-game probe for the SCS telemetry loader's API fallback sequence.
//!
//! The official loader starts with its newest Telemetry API and retries older
//! versions only when `scs_telemetry_init` reports `Unsupported`. This example
//! deliberately rejects every negotiated API except 1.00, then registers a
//! small 1.00-compatible callback surface. Its game-log markers distinguish a
//! genuine loader retry from an ordinary successful initialization.

#![forbid(unsafe_code)]
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

use scs_sdk_plugin::sdk::{SdkError, TelemetryApiVersion, channels, game};
use scs_sdk_plugin::{
    ChannelUpdate, Game, GameCompatibility, PluginCompatibility, PluginContext, PluginError,
    PluginMetadata, PluginResult, TelemetryEvent, TelemetryEventKind, TelemetryPlugin,
    export_plugin,
};

/// The deliberately selected fallback endpoint.
///
/// SDK 1.14 defines both API 1.00 and 1.01. A current game should first call the
/// export with 1.01, receive `Unsupported`, and then retry with this version.
const ACCEPTED_FALLBACK_API: TelemetryApiVersion = TelemetryApiVersion::V1_00;

/// The probe uses only descriptors present in the initial ETS2 telemetry
/// schema, keeping API fallback independent from later per-game schema changes.
static SUPPORTED_GAMES: [GameCompatibility; 1] = [GameCompatibility::new(
    Game::EuroTruckSimulator2,
    game::ets2::V1_00,
)];

/// Returns whether this exact loader attempt is the endpoint under test.
///
/// Equality is intentional. Accepting every later minor would make the first
/// 1.01 attempt succeed and would therefore bypass the behavior this artifact
/// exists to exercise.
const fn accepts_loader_attempt(version: TelemetryApiVersion) -> bool {
    version.raw() == ACCEPTED_FALLBACK_API.raw()
}

/// Minimal state proving that the accepted 1.00 session reaches both callback
/// classes registered by this probe.
///
/// SCS does not promise that a changed channel value arrives before the first
/// frame-end event. The probe therefore records the two observations
/// independently and emits its confirmation only after both are present.
#[derive(Debug, Default)]
struct TelemetryFallbackExample {
    accepted_api: Option<TelemetryApiVersion>,
    latest_speed_metres_per_second: Option<f32>,
    frame_end_seen: bool,
    callback_confirmation_logged: bool,
}

impl TelemetryFallbackExample {
    /// Returns whether the log can prove both registered callback paths.
    ///
    /// A speed of zero is still a real channel value, so readiness depends on
    /// `Option::is_some` rather than the numeric value. The logged flag keeps
    /// the proof marker one-shot across every later frame and speed update.
    fn callbacks_ready_for_confirmation(&self) -> bool {
        !self.callback_confirmation_logged
            && self.frame_end_seen
            && self.latest_speed_metres_per_second.is_some()
    }

    /// Emits the callback proof once event and channel delivery are both known.
    ///
    /// This helper is called after either observation so it remains correct for
    /// both legal orders: channel-before-frame-end and frame-end-before-channel.
    fn confirm_callbacks_if_ready(&mut self, context: &PluginContext<'_>) {
        if !self.callbacks_ready_for_confirmation() {
            return;
        }

        let Some(speed_metres_per_second) = self.latest_speed_metres_per_second else {
            // The readiness predicate above established this invariant. Keep
            // the branch explicit instead of relying on unwrap in application
            // code, preserving the example's non-panicking boundary.
            return;
        };

        self.callback_confirmation_logged = true;
        context.message(format_args!(
            concat!(
                "[scs-sdk-fallback-example] fallback callbacks confirmed ",
                "telemetry_api={} frame_end_seen=true ",
                "speed_metres_per_second={}"
            ),
            context.telemetry_api_version(),
            speed_metres_per_second,
        ));
    }
}

impl TelemetryPlugin for TelemetryFallbackExample {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("SCS SDK Telemetry Fallback E2E", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> PluginCompatibility {
        // The framework must admit both 1.00 and later compatible attempts so
        // product initialization can deliberately return `Unsupported` for the
        // first one. Declaring 1.01 here would reject the eventual 1.00 retry
        // before the probe receives it.
        PluginCompatibility::new(TelemetryApiVersion::V1_00, &SUPPORTED_GAMES)
    }

    fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
        *self = Self::default();
        let negotiated_api = context.telemetry_api_version();

        if !accepts_loader_attempt(negotiated_api) {
            context.message(format_args!(
                concat!(
                    "[scs-sdk-fallback-example] requesting loader retry ",
                    "rejected_telemetry_api={} accepted_telemetry_api={} ",
                    "result=unsupported"
                ),
                negotiated_api, ACCEPTED_FALLBACK_API,
            ));
            return Err(PluginError::new(
                SdkError::Unsupported,
                format!(
                    "fallback E2E intentionally rejects telemetry API {negotiated_api}; retry {ACCEPTED_FALLBACK_API}"
                ),
            ));
        }

        self.accepted_api = Some(negotiated_api);

        // Every declaration below is valid in Telemetry API 1.00. Gameplay and
        // signed-64-bit capabilities stay absent so success proves the loader
        // actually retried instead of relying on a 1.01-only representation.
        context.subscribe_event(TelemetryEventKind::Started)?;
        context.subscribe_event(TelemetryEventKind::FrameEnd)?;
        context.subscribe(channels::truck::SPEED)?;

        context.message(format_args!(
            concat!(
                "[scs-sdk-fallback-example] accepted loader fallback ",
                "telemetry_api={} expected_telemetry_api={}"
            ),
            negotiated_api, ACCEPTED_FALLBACK_API,
        ));
        Ok(())
    }

    fn channel(&mut self, context: &mut PluginContext<'_>, update: ChannelUpdate<'_>) {
        if let Some(speed) = update.value(channels::truck::SPEED) {
            self.latest_speed_metres_per_second = Some(speed);
            self.confirm_callbacks_if_ready(context);
        }
    }

    fn event(&mut self, context: &mut PluginContext<'_>, event: TelemetryEvent<'_>) {
        match event {
            TelemetryEvent::Started => context.message(format_args!(
                "[scs-sdk-fallback-example] fallback session started telemetry_api={}",
                context.telemetry_api_version(),
            )),
            TelemetryEvent::FrameEnd => {
                self.frame_end_seen = true;
                self.confirm_callbacks_if_ready(context);
            }
            TelemetryEvent::FrameStart(_)
            | TelemetryEvent::Paused
            | TelemetryEvent::Configuration(_)
            | TelemetryEvent::Gameplay(_) => {}
        }
    }

    fn shutdown(&mut self, context: &mut PluginContext<'_>) {
        if let Some(accepted_api) = self.accepted_api {
            context.message(format_args!(
                concat!(
                    "[scs-sdk-fallback-example] fallback session shutdown ",
                    "telemetry_api={} callbacks_confirmed={}"
                ),
                accepted_api, self.callback_confirmation_logged,
            ));
        } else {
            // The framework invokes product shutdown after a rejected
            // initialization so attempt-local state can be cleaned before the
            // game retries. This marker proves that path completed as well.
            context.message(format_args!(
                concat!(
                    "[scs-sdk-fallback-example] rejected attempt cleaned ",
                    "telemetry_api={}"
                ),
                context.telemetry_api_version(),
            ));
        }
        *self = Self::default();
    }
}

export_plugin!(TelemetryFallbackExample::default());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_api_v100_is_the_probe_endpoint() {
        assert!(accepts_loader_attempt(TelemetryApiVersion::V1_00));
        assert!(!accepts_loader_attempt(TelemetryApiVersion::V1_01));
        assert!(!accepts_loader_attempt(TelemetryApiVersion::new(1, 2)));
        assert!(!accepts_loader_attempt(TelemetryApiVersion::new(2, 0)));
    }

    #[test]
    fn callback_confirmation_requires_both_delivery_paths_and_is_one_shot() {
        let mut probe = TelemetryFallbackExample::default();
        assert!(!probe.callbacks_ready_for_confirmation());

        // Either callback is insufficient by itself.
        probe.frame_end_seen = true;
        assert!(!probe.callbacks_ready_for_confirmation());
        probe.frame_end_seen = false;
        probe.latest_speed_metres_per_second = Some(0.0);
        assert!(!probe.callbacks_ready_for_confirmation());

        // Zero is a valid stopped-truck sample and therefore proves channel
        // delivery just as strongly as a nonzero speed.
        probe.frame_end_seen = true;
        assert!(probe.callbacks_ready_for_confirmation());

        // Once the marker has been emitted, later callbacks must not duplicate
        // the E2E evidence line.
        probe.callback_confirmation_logged = true;
        assert!(!probe.callbacks_ready_for_confirmation());
    }
}
