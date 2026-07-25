//! End-to-end ETS2 telemetry example built on the safe plugin framework.
//!
//! This crate intentionally contains only product-owned state and behavior:
//! which channels are interesting, how a snapshot is accumulated, and what is
//! written to the game log. ABI exports, SDK callback pointers, lifecycle
//! synchronization, registration rollback, and foreign value decoding belong
//! to `scs-sdk-plugin` and its lower layers.

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

use scs_sdk_plugin::sdk::{
    DPlacement, TelemetryApiVersion, channels, configuration, game, gameplay,
};
use scs_sdk_plugin::{
    ChannelUpdate, ConfigurationEvent, Game, GameCompatibility, GameplayEvent, PluginCompatibility,
    PluginContext, PluginMetadata, PluginResult, TelemetryEvent, TelemetryEventKind,
    TelemetryPlugin, export_plugin,
};

/// Probe output is rate-limited to one line per real-time second.
///
/// SCS render timestamps are expressed in microseconds, so this value is used
/// directly without a floating-point duration conversion.
const LOG_INTERVAL_MICROSECONDS: u64 = 1_000_000;

/// Conversion factor from the SDK's SI speed unit to the unit displayed by the
/// current diagnostic log.
const METRES_PER_SECOND_TO_KILOMETRES_PER_HOUR: f32 = 3.6;

/// Compatibility requirements derived from capabilities used by this example.
///
/// Gameplay callbacks require Telemetry API 1.01. ETS2 schema 1.14 is the first
/// version documented by SCS with gameplay-event support; the navigation
/// channels used below arrived earlier in schema 1.12. Later schema minors are
/// compatible additions and are accepted within major version 1.
static SUPPORTED_GAMES: [GameCompatibility; 1] = [GameCompatibility::new(
    Game::EuroTruckSimulator2,
    game::ets2::V1_14,
)];

/// Last values received from the selected telemetry channels.
///
/// Channel callbacks are independent; a frame-end event therefore observes a
/// coherent product snapshot assembled from the most recent value of each
/// channel rather than assuming a particular callback order inside the frame.
#[derive(Clone, Copy, Debug, Default)]
struct Snapshot {
    placement: DPlacement,
    speed_metres_per_second: f32,
    engine_rpm: f32,
    gear: i32,
    navigation_distance_metres: f32,
    navigation_time_seconds: f32,
    navigation_speed_limit_metres_per_second: f32,
    cargo_damage: f32,
}

/// Product-owned plugin state.
///
/// The framework invokes every hook synchronously under the SDK lifecycle, so
/// this state needs no global variable or application-side mutex. Resetting the
/// value on initialization and shutdown also prevents one game session from
/// leaking data into a later reinitialization of the same loaded library.
#[derive(Debug)]
struct TelemetryExample {
    snapshot: Snapshot,
    current_render_time: u64,
    last_probe_log_time: Option<u64>,
    paused: bool,
}

impl Default for TelemetryExample {
    fn default() -> Self {
        Self {
            snapshot: Snapshot::default(),
            current_render_time: 0,
            last_probe_log_time: None,
            // No driving snapshot should be emitted until SCS reports Started.
            paused: true,
        }
    }
}

impl TelemetryExample {
    /// Applies one type-checked channel callback to the accumulated snapshot.
    ///
    /// `ChannelUpdate::value` verifies both the registered descriptor and the
    /// SCS tagged-union discriminator. A mismatched or absent value therefore
    /// leaves the previous snapshot field unchanged instead of inventing zero.
    fn update_snapshot(&mut self, update: ChannelUpdate<'_>) {
        if let Some(value) = update.value(channels::truck::WORLD_PLACEMENT) {
            self.snapshot.placement = value;
        } else if let Some(value) = update.value(channels::truck::SPEED) {
            self.snapshot.speed_metres_per_second = value;
        } else if let Some(value) = update.value(channels::truck::ENGINE_RPM) {
            self.snapshot.engine_rpm = value;
        } else if let Some(value) = update.value(channels::truck::ENGINE_GEAR) {
            self.snapshot.gear = value;
        } else if let Some(value) = update.value(channels::truck::NAVIGATION_DISTANCE) {
            self.snapshot.navigation_distance_metres = value;
        } else if let Some(value) = update.value(channels::truck::NAVIGATION_TIME) {
            self.snapshot.navigation_time_seconds = value;
        } else if let Some(value) = update.value(channels::truck::NAVIGATION_SPEED_LIMIT) {
            self.snapshot.navigation_speed_limit_metres_per_second = value;
        } else if let Some(value) = update.value(channels::job::CARGO_DAMAGE) {
            self.snapshot.cargo_damage = value;
        }
    }

    /// Emits the current probe after enforcing pause and one-second throttling.
    fn log_probe(&mut self, context: &PluginContext<'_>) {
        if self.paused {
            return;
        }

        if let Some(previous) = self.last_probe_log_time {
            let elapsed = self.current_render_time.checked_sub(previous);
            if elapsed.is_some_and(|elapsed| elapsed < LOG_INTERVAL_MICROSECONDS) {
                return;
            }
        }
        self.last_probe_log_time = Some(self.current_render_time);

        // IEEE-754 retains a negative sign on values very close to zero. That
        // is useful for calculations but noisy in a human probe, so only the
        // formatted display is normalized; the stored SDK value stays intact.
        let raw_speed_kmh =
            self.snapshot.speed_metres_per_second * METRES_PER_SECOND_TO_KILOMETRES_PER_HOUR;
        let speed_kmh = if raw_speed_kmh.abs() < 0.05 {
            0.0
        } else {
            raw_speed_kmh
        };

        let speed_limit = self.snapshot.navigation_speed_limit_metres_per_second;
        let speed_limit_text = if speed_limit > 0.0 {
            format!(
                "{:.1}km/h",
                speed_limit * METRES_PER_SECOND_TO_KILOMETRES_PER_HOUR
            )
        } else {
            String::from("n/a")
        };

        context.message(format_args!(
            concat!(
                "[scs-sdk-example] probe speed={:.1}km/h rpm={:.0} gear={} ",
                "position=({:.3},{:.3},{:.3}) heading={:.4} ",
                "navigation_distance={:.2}km navigation_time={:.0}s ",
                "speed_limit={} cargo_damage={:.3}"
            ),
            speed_kmh,
            self.snapshot.engine_rpm,
            self.snapshot.gear,
            self.snapshot.placement.position.x,
            self.snapshot.placement.position.y,
            self.snapshot.placement.position.z,
            self.snapshot.placement.orientation.heading,
            self.snapshot.navigation_distance_metres / 1000.0,
            self.snapshot.navigation_time_seconds,
            speed_limit_text,
            self.snapshot.cargo_damage,
        ));
    }

    /// Logs a complete active-job configuration using the typed attribute map.
    fn log_job_configuration(
        context: &PluginContext<'_>,
        configuration_event: ConfigurationEvent<'_>,
    ) {
        if !configuration_event.is(configuration::ids::JOB) {
            return;
        }
        if !configuration_event.has_attributes() {
            context.message(format_args!("[scs-sdk-example] no active job"));
            return;
        }

        let cargo_name = configuration_event
            .string_owned(configuration::attributes::CARGO)
            .unwrap_or_default();
        let cargo_id = configuration_event
            .string_owned(configuration::attributes::CARGO_ID)
            .unwrap_or_default();
        let source_city = configuration_event
            .string_owned(configuration::attributes::SOURCE_CITY)
            .unwrap_or_default();
        let source_city_id = configuration_event
            .string_owned(configuration::attributes::SOURCE_CITY_ID)
            .unwrap_or_default();
        let source_company = configuration_event
            .string_owned(configuration::attributes::SOURCE_COMPANY)
            .unwrap_or_default();
        let source_company_id = configuration_event
            .string_owned(configuration::attributes::SOURCE_COMPANY_ID)
            .unwrap_or_default();
        let destination_city = configuration_event
            .string_owned(configuration::attributes::DESTINATION_CITY)
            .unwrap_or_default();
        let destination_city_id = configuration_event
            .string_owned(configuration::attributes::DESTINATION_CITY_ID)
            .unwrap_or_default();
        let destination_company = configuration_event
            .string_owned(configuration::attributes::DESTINATION_COMPANY)
            .unwrap_or_default();
        let destination_company_id = configuration_event
            .string_owned(configuration::attributes::DESTINATION_COMPANY_ID)
            .unwrap_or_default();
        // Keep both views deliberately. `job_market` gives application logic a
        // closed, typed SDK 1.14 value, while the generic string accessor keeps
        // a future additive market visible in diagnostics instead of erasing it
        // merely because this build predates that value.
        let market_raw = configuration_event
            .string_owned(configuration::attributes::JOB_MARKET)
            .unwrap_or_default();
        let market_known = configuration_event.job_market();

        let mass = configuration_event
            .get(configuration::attributes::CARGO_MASS)
            .unwrap_or_default();
        let income = configuration_event
            .get(configuration::attributes::INCOME)
            .unwrap_or_default();
        let planned_distance = configuration_event
            .get(configuration::attributes::PLANNED_DISTANCE_KM)
            .unwrap_or_default();
        let delivery_time = configuration_event
            .get(configuration::attributes::DELIVERY_TIME)
            .unwrap_or_default();
        let cargo_loaded = configuration_event
            .get(configuration::attributes::IS_CARGO_LOADED)
            .unwrap_or_default();
        let special_job = configuration_event
            .get(configuration::attributes::SPECIAL_JOB)
            .unwrap_or_default();

        context.message(format_args!(
            concat!(
                "[scs-sdk-example] job cargo={} cargo_id={} mass={:.0}kg ",
                "source={}({})/{}({}) destination={}({})/{}({}) ",
                "market_raw={} market_known={:?} ",
                "income={} planned_distance={}km delivery_time={} ",
                "cargo_loaded={} special_job={}"
            ),
            cargo_name,
            cargo_id,
            mass,
            source_city,
            source_city_id,
            source_company,
            source_company_id,
            destination_city,
            destination_city_id,
            destination_company,
            destination_company_id,
            market_raw,
            market_known,
            income,
            planned_distance,
            delivery_time,
            cargo_loaded,
            special_job,
        ));
    }

    /// Logs a paid ferry or train journey with the complete SDK payload.
    ///
    /// Ferry and train events expose the same five attributes. Keeping their
    /// decoding in one helper makes it easy to compare real-game evidence while
    /// the caller still names each event explicitly in [`Self::log_gameplay`].
    fn log_paid_transport(
        context: &PluginContext<'_>,
        event: GameplayEvent<'_>,
        transport: &'static str,
    ) {
        let amount = event
            .get(gameplay::attributes::PAY_AMOUNT)
            .unwrap_or_default();
        let source_name = event
            .string_owned(gameplay::attributes::SOURCE_NAME)
            .unwrap_or_default();
        let source_id = event
            .string_owned(gameplay::attributes::SOURCE_ID)
            .unwrap_or_default();
        let target_name = event
            .string_owned(gameplay::attributes::TARGET_NAME)
            .unwrap_or_default();
        let target_id = event
            .string_owned(gameplay::attributes::TARGET_ID)
            .unwrap_or_default();

        context.message(format_args!(
            concat!(
                "[scs-sdk-example] {} used amount={} ",
                "source={}({}) target={}({})"
            ),
            transport, amount, source_name, source_id, target_name, target_id,
        ));
    }

    /// Logs all six gameplay events defined by the SDK 1.14 header.
    ///
    /// The example keeps every branch explicit even though SCS delivers them
    /// through one generic gameplay callback. Besides making the application
    /// boundary readable, this gives real ETS2 tests a distinct marker for
    /// every official payload shape.
    fn log_gameplay(context: &PluginContext<'_>, event: GameplayEvent<'_>) {
        if event.is(gameplay::events::JOB_DELIVERED) {
            let revenue = event.get(gameplay::attributes::REVENUE).unwrap_or_default();
            let earned_xp = event
                .get(gameplay::attributes::EARNED_XP)
                .unwrap_or_default();
            let cargo_damage = event
                .get(gameplay::attributes::CARGO_DAMAGE)
                .unwrap_or_default();
            let distance_km = event
                .get(gameplay::attributes::DISTANCE_KM)
                .unwrap_or_default();
            let delivery_time = event
                .get(gameplay::attributes::DELIVERY_TIME)
                .unwrap_or_default();
            let auto_park_used = event
                .get(gameplay::attributes::AUTO_PARK_USED)
                .unwrap_or_default();
            let auto_load_used = event
                .get(gameplay::attributes::AUTO_LOAD_USED)
                .unwrap_or_default();

            context.message(format_args!(
                concat!(
                    "[scs-sdk-example] job delivered revenue={} xp={} ",
                    "cargo_damage={:.3} distance={:.1}km delivery_time={}min ",
                    "auto_park={} auto_load={}"
                ),
                revenue,
                earned_xp,
                cargo_damage,
                distance_km,
                delivery_time,
                auto_park_used,
                auto_load_used,
            ));
        } else if event.is(gameplay::events::JOB_CANCELLED) {
            let penalty = event
                .get(gameplay::attributes::CANCEL_PENALTY)
                .unwrap_or_default();
            context.message(format_args!(
                "[scs-sdk-example] job cancelled penalty={penalty}"
            ));
        } else if event.is(gameplay::events::PLAYER_FINED) {
            // As with job markets, retain the original SDK string beside the
            // typed value so a newer game can add an offence without making an
            // older plugin print a misleading known classification.
            let offence_raw = event
                .string_owned(gameplay::attributes::FINE_OFFENCE)
                .unwrap_or_default();
            let offence_known = event.fine_offence();
            let amount = event
                .get(gameplay::attributes::FINE_AMOUNT)
                .unwrap_or_default();
            context.message(format_args!(
                concat!(
                    "[scs-sdk-example] player fined offence_raw={} ",
                    "offence_known={:?} amount={}"
                ),
                offence_raw, offence_known, amount,
            ));
        } else if event.is(gameplay::events::PLAYER_TOLLGATE_PAID) {
            let amount = event
                .get(gameplay::attributes::PAY_AMOUNT)
                .unwrap_or_default();
            context.message(format_args!(
                "[scs-sdk-example] tollgate paid amount={amount}"
            ));
        } else if event.is(gameplay::events::PLAYER_USE_FERRY) {
            Self::log_paid_transport(context, event, "ferry");
        } else if event.is(gameplay::events::PLAYER_USE_TRAIN) {
            Self::log_paid_transport(context, event, "train");
        }
    }
}

impl TelemetryPlugin for TelemetryExample {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("SCS SDK Telemetry Example", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> PluginCompatibility {
        PluginCompatibility::new(TelemetryApiVersion::V1_01, &SUPPORTED_GAMES)
    }

    fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
        *self = Self::default();

        // Event capabilities are deliberately listed rather than inferred from
        // the `event` hook. Removing one line here removes the corresponding SDK
        // registration and makes the plugin's complete callback surface visible
        // during code review.
        context.subscribe_event(TelemetryEventKind::FrameStart)?;
        context.subscribe_event(TelemetryEventKind::FrameEnd)?;
        context.subscribe_event(TelemetryEventKind::Paused)?;
        context.subscribe_event(TelemetryEventKind::Started)?;
        context.subscribe_event(TelemetryEventKind::Configuration)?;
        context.subscribe_event(TelemetryEventKind::Gameplay)?;

        // Scalar channel subscriptions use change-driven delivery, which is
        // sufficient because the frame-end hook reads the last complete
        // snapshot. The framework retains each descriptor's expected union type
        // and rejects duplicate or indexed misuse before invoking the SDK.
        context.subscribe(channels::truck::WORLD_PLACEMENT)?;
        context.subscribe(channels::truck::SPEED)?;
        context.subscribe(channels::truck::ENGINE_RPM)?;
        context.subscribe(channels::truck::ENGINE_GEAR)?;
        context.subscribe(channels::truck::NAVIGATION_DISTANCE)?;
        context.subscribe(channels::truck::NAVIGATION_TIME)?;
        context.subscribe(channels::truck::NAVIGATION_SPEED_LIMIT)?;
        context.subscribe(channels::job::CARGO_DAMAGE)?;

        context.message(format_args!("[scs-sdk-example] example state initialized"));
        Ok(())
    }

    fn channel(&mut self, _context: &mut PluginContext<'_>, update: ChannelUpdate<'_>) {
        self.update_snapshot(update);
    }

    fn event(&mut self, context: &mut PluginContext<'_>, event: TelemetryEvent<'_>) {
        match event {
            TelemetryEvent::FrameStart(frame) => {
                self.current_render_time = frame.render_time();
                if frame.timer_restarted() {
                    // A timer restart can move the render timestamp backwards;
                    // discard the old throttle anchor before the next frame end.
                    self.last_probe_log_time = None;
                }
            }
            TelemetryEvent::FrameEnd => self.log_probe(context),
            TelemetryEvent::Paused => {
                self.paused = true;
                context.message(format_args!("[scs-sdk-example] telemetry paused"));
            }
            TelemetryEvent::Started => {
                self.paused = false;
                context.message(format_args!("[scs-sdk-example] telemetry started"));
            }
            TelemetryEvent::Configuration(configuration_event) => {
                Self::log_job_configuration(context, configuration_event);
            }
            TelemetryEvent::Gameplay(gameplay_event) => {
                Self::log_gameplay(context, gameplay_event);
            }
        }
    }

    fn shutdown(&mut self, context: &mut PluginContext<'_>) {
        context.message(format_args!("[scs-sdk-example] example state shutdown"));
        *self = Self::default();
    }
}

export_plugin!(TelemetryExample::default());
