//! End-to-end ETS2 and ATS telemetry example built on the safe plugin framework.
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
/// Gameplay callbacks require Telemetry API 1.01. ETS2 schema 1.14 and ATS
/// schema 1.01 are the first versions documented by SCS with gameplay-event
/// support; the navigation channels used below arrived earlier (ETS2 1.12,
/// ATS 1.00). Later schema minors are compatible additions and are accepted
/// within major version 1.
///
/// ATS was audited before being listed here: every subscribed channel exists
/// in ATS, the example uses no `AdBlue` channel (which ATS does not provide),
/// distances and speeds are SI in both games, and money values are logged
/// verbatim in the game's native currency (EUR for ETS2, USD for ATS) without
/// conversion.
static SUPPORTED_GAMES: [GameCompatibility; 2] = [
    GameCompatibility::new(Game::EuroTruckSimulator2, game::ets2::V1_14),
    GameCompatibility::new(Game::AmericanTruckSimulator, game::ats::V1_01),
];

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
    /// In-game minutes until the next mandatory break (SDK 1.16).
    ///
    /// `None` until SCS delivers a value: the channel is optional, older
    /// schemas do not provide it, and the game may omit it when the mandatory
    /// break simulation is disabled. A fabricated zero would look like an
    /// authoritative "break due now".
    next_mandatory_break_minutes: Option<i32>,
}

/// Renders a value SCS may omit, printing `n/a` when it was not sent.
///
/// Real games do omit documented attributes: ATS 1.62 sends no `earned.xp`
/// with `car_job.delivered`. Printing a type default such as `0` would look
/// like an authoritative value, so absence stays visible in the log.
fn shown<T: core::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| String::from("n/a"), |value| value.to_string())
}

/// Appends a unit to a rendered value, leaving `n/a` without a unit.
fn with_unit(text: String, unit: &str) -> String {
    if text == "n/a" { text } else { text + unit }
}

/// Like [`shown`], with a fixed number of decimals for floating-point values.
fn shown_fixed(value: Option<f32>, decimals: usize) -> String {
    value.map_or_else(
        || String::from("n/a"),
        |value| format!("{value:.decimals$}"),
    )
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
        } else if let Some(value) = update.value(channels::common::NEXT_MANDATORY_BREAK) {
            self.snapshot.next_mandatory_break_minutes = Some(value);
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
        let mandatory_break_text = self
            .snapshot
            .next_mandatory_break_minutes
            .map_or_else(|| String::from("n/a"), |minutes| format!("{minutes}min"));

        context.message(format_args!(
            concat!(
                "[scs-sdk-example] probe speed={:.1}km/h rpm={:.0} gear={} ",
                "position=({:.3},{:.3},{:.3}) heading={:.4} ",
                "navigation_distance={:.2}km navigation_time={:.0}s ",
                "speed_limit={} cargo_damage={:.3} mandatory_break={}"
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
            mandatory_break_text,
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

        let cargo_name = configuration_event.string_owned(configuration::attributes::CARGO);
        let cargo_id = configuration_event.string_owned(configuration::attributes::CARGO_ID);
        let source_city = configuration_event.string_owned(configuration::attributes::SOURCE_CITY);
        let source_city_id =
            configuration_event.string_owned(configuration::attributes::SOURCE_CITY_ID);
        let source_company =
            configuration_event.string_owned(configuration::attributes::SOURCE_COMPANY);
        let source_company_id =
            configuration_event.string_owned(configuration::attributes::SOURCE_COMPANY_ID);
        let destination_city =
            configuration_event.string_owned(configuration::attributes::DESTINATION_CITY);
        let destination_city_id =
            configuration_event.string_owned(configuration::attributes::DESTINATION_CITY_ID);
        let destination_company =
            configuration_event.string_owned(configuration::attributes::DESTINATION_COMPANY);
        let destination_company_id =
            configuration_event.string_owned(configuration::attributes::DESTINATION_COMPANY_ID);
        // Keep both views deliberately. `job_market` gives application logic a
        // closed, typed SDK 1.16 value, while the generic string accessor keeps
        // a future additive market visible in diagnostics instead of erasing it
        // merely because this build predates that value.
        let market_raw = configuration_event.string_owned(configuration::attributes::JOB_MARKET);
        let market_known = configuration_event.job_market();

        let mass = configuration_event.get(configuration::attributes::CARGO_MASS);
        let income = configuration_event.get(configuration::attributes::INCOME);
        let planned_distance =
            configuration_event.get(configuration::attributes::PLANNED_DISTANCE_KM);
        let delivery_time = configuration_event.get(configuration::attributes::DELIVERY_TIME);
        let cargo_loaded = configuration_event.get(configuration::attributes::IS_CARGO_LOADED);
        let special_job = configuration_event.get(configuration::attributes::SPECIAL_JOB);

        context.message(format_args!(
            concat!(
                "[scs-sdk-example] job cargo={} cargo_id={} mass={} ",
                "source={}({})/{}({}) destination={}({})/{}({}) ",
                "market_raw={} market_known={:?} ",
                "income={} planned_distance={} delivery_time={} ",
                "cargo_loaded={} special_job={}"
            ),
            shown(cargo_name),
            shown(cargo_id),
            with_unit(shown_fixed(mass, 0), "kg"),
            shown(source_city),
            shown(source_city_id),
            shown(source_company),
            shown(source_company_id),
            shown(destination_city),
            shown(destination_city_id),
            shown(destination_company),
            shown(destination_company_id),
            shown(market_raw),
            market_known,
            shown(income),
            with_unit(shown(planned_distance), "km"),
            shown(delivery_time),
            shown(cargo_loaded),
            shown(special_job),
        ));
    }

    /// Logs the SDK 1.16 car-job configuration (ATS schema 1.07 and later).
    ///
    /// Car jobs report a unit count and customer priorities instead of the
    /// cargo-market fields of a classic job. As with `job.market`, both the raw
    /// `car_job.market` string and the typed known value are retained.
    fn log_car_job_configuration(
        context: &PluginContext<'_>,
        configuration_event: ConfigurationEvent<'_>,
    ) {
        if !configuration_event.has_attributes() {
            context.message(format_args!("[scs-sdk-example] no active car job"));
            return;
        }

        let text = |attribute| configuration_event.string_owned(attribute);
        let cargo_name = text(configuration::attributes::CARGO);
        let cargo_id = text(configuration::attributes::CARGO_ID);
        let source_city_id = text(configuration::attributes::SOURCE_CITY_ID);
        let source_company_id = text(configuration::attributes::SOURCE_COMPANY_ID);
        let destination_city_id = text(configuration::attributes::DESTINATION_CITY_ID);
        let destination_company_id = text(configuration::attributes::DESTINATION_COMPANY_ID);
        let market_raw = text(configuration::attributes::CAR_JOB_MARKET);
        let market_known = configuration_event.car_job_market();

        let unit_count = configuration_event.get(configuration::attributes::CARGO_UNIT_COUNT);
        let income = configuration_event.get(configuration::attributes::INCOME);
        let planned_distance =
            configuration_event.get(configuration::attributes::PLANNED_DISTANCE_KM);
        let delivery_time = configuration_event.get(configuration::attributes::DELIVERY_TIME);
        let prio_cargo =
            configuration_event.get(configuration::attributes::CUSTOMER_PRIO_CARGO_HANDLING);
        let prio_time = configuration_event.get(configuration::attributes::CUSTOMER_PRIO_TIME);
        let prio_vehicle =
            configuration_event.get(configuration::attributes::CUSTOMER_PRIO_VEHICLE_APPEARANCE);

        context.message(format_args!(
            concat!(
                "[scs-sdk-example] car job cargo={} cargo_id={} units={} ",
                "source={}/{} destination={}/{} ",
                "market_raw={} market_known={:?} ",
                "income={} planned_distance={} delivery_time={} ",
                "prio_cargo={} prio_time={} prio_vehicle={}"
            ),
            shown(cargo_name),
            shown(cargo_id),
            shown(unit_count),
            shown(source_city_id),
            shown(source_company_id),
            shown(destination_city_id),
            shown(destination_company_id),
            shown(market_raw),
            market_known,
            shown(income),
            with_unit(shown(planned_distance), "km"),
            shown(delivery_time),
            shown(prio_cargo),
            shown(prio_time),
            shown(prio_vehicle),
        ));
    }

    /// Logs the job-related configuration groups the example understands.
    ///
    /// SDK 1.16 documents no bus-job attributes yet (SCS will document them
    /// with the bus DLC), so a `bus_job` configuration only reports whether
    /// the game sent any attributes at all.
    fn log_configuration(context: &PluginContext<'_>, configuration_event: ConfigurationEvent<'_>) {
        if configuration_event.is(configuration::ids::JOB) {
            Self::log_job_configuration(context, configuration_event);
        } else if configuration_event.is(configuration::ids::CAR_JOB) {
            Self::log_car_job_configuration(context, configuration_event);
        } else if configuration_event.is(configuration::ids::BUS_JOB) {
            context.message(format_args!(
                "[scs-sdk-example] bus job configuration has_attributes={}",
                configuration_event.has_attributes()
            ));
        }
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
        let amount = event.get(gameplay::attributes::PAY_AMOUNT);
        let source_name = event.string_owned(gameplay::attributes::SOURCE_NAME);
        let source_id = event.string_owned(gameplay::attributes::SOURCE_ID);
        let target_name = event.string_owned(gameplay::attributes::TARGET_NAME);
        let target_id = event.string_owned(gameplay::attributes::TARGET_ID);

        context.message(format_args!(
            concat!(
                "[scs-sdk-example] {} used amount={} ",
                "source={}({}) target={}({})"
            ),
            transport,
            shown(amount),
            shown(source_name),
            shown(source_id),
            shown(target_name),
            shown(target_id),
        ));
    }

    /// Logs all eight gameplay events defined by the SDK 1.16 header.
    ///
    /// The example keeps every branch explicit even though SCS delivers them
    /// through one generic gameplay callback. Besides making the application
    /// boundary readable, this gives real ETS2 tests a distinct marker for
    /// every official payload shape.
    fn log_gameplay(context: &PluginContext<'_>, event: GameplayEvent<'_>) {
        if event.is(gameplay::events::JOB_DELIVERED) {
            let revenue = event.get(gameplay::attributes::REVENUE);
            let earned_xp = event.get(gameplay::attributes::EARNED_XP);
            let cargo_damage = event.get(gameplay::attributes::CARGO_DAMAGE);
            let distance_km = event.get(gameplay::attributes::DISTANCE_KM);
            let delivery_time = event.get(gameplay::attributes::DELIVERY_TIME);
            let auto_park_used = event.get(gameplay::attributes::AUTO_PARK_USED);
            let auto_load_used = event.get(gameplay::attributes::AUTO_LOAD_USED);

            context.message(format_args!(
                concat!(
                    "[scs-sdk-example] job delivered revenue={} xp={} ",
                    "cargo_damage={} distance={} delivery_time={} ",
                    "auto_park={} auto_load={}"
                ),
                shown(revenue),
                shown(earned_xp),
                shown_fixed(cargo_damage, 3),
                with_unit(shown_fixed(distance_km, 1), "km"),
                with_unit(shown(delivery_time), "min"),
                shown(auto_park_used),
                shown(auto_load_used),
            ));
        } else if event.is(gameplay::events::CAR_JOB_DELIVERED) {
            let revenue = event.get(gameplay::attributes::REVENUE);
            let earned_xp = event.get(gameplay::attributes::EARNED_XP);
            let cargo_damage = event.get(gameplay::attributes::CARGO_DAMAGE);
            let vehicle_damage = event.get(gameplay::attributes::VEHICLE_DAMAGE);
            let distance_km = event.get(gameplay::attributes::DISTANCE_KM);
            let delivery_time = event.get(gameplay::attributes::DELIVERY_TIME);

            context.message(format_args!(
                concat!(
                    "[scs-sdk-example] car job delivered revenue={} xp={} ",
                    "cargo_damage={} vehicle_damage={} ",
                    "distance={} delivery_time={}"
                ),
                shown(revenue),
                shown(earned_xp),
                shown_fixed(cargo_damage, 3),
                shown_fixed(vehicle_damage, 3),
                with_unit(shown_fixed(distance_km, 1), "km"),
                with_unit(shown(delivery_time), "min"),
            ));
        } else if event.is(gameplay::events::JOB_CANCELLED) {
            let penalty = event.get(gameplay::attributes::CANCEL_PENALTY);
            context.message(format_args!(
                "[scs-sdk-example] job cancelled penalty={}",
                shown(penalty)
            ));
        } else if event.is(gameplay::events::CAR_JOB_CANCELLED) {
            let penalty = event.get(gameplay::attributes::CANCEL_PENALTY);
            context.message(format_args!(
                "[scs-sdk-example] car job cancelled penalty={}",
                shown(penalty)
            ));
        } else if event.is(gameplay::events::PLAYER_FINED) {
            // As with job markets, retain the original SDK string beside the
            // typed value so a newer game can add an offence without making an
            // older plugin print a misleading known classification.
            let offence_raw = event.string_owned(gameplay::attributes::FINE_OFFENCE);
            let offence_known = event.fine_offence();
            let amount = event.get(gameplay::attributes::FINE_AMOUNT);
            context.message(format_args!(
                concat!(
                    "[scs-sdk-example] player fined offence_raw={} ",
                    "offence_known={:?} amount={}"
                ),
                shown(offence_raw),
                offence_known,
                shown(amount),
            ));
        } else if event.is(gameplay::events::PLAYER_TOLLGATE_PAID) {
            let amount = event.get(gameplay::attributes::PAY_AMOUNT);
            context.message(format_args!(
                "[scs-sdk-example] tollgate paid amount={}",
                shown(amount)
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
        // SDK 1.16 channel, subscribed as optional: schemas before ETS2 1.19 /
        // ATS 1.06 do not declare it, and the framework skips it there instead
        // of rejecting the whole plugin. SCS only started delivering values in
        // ETS2 1.21 / ATS 1.08, so the snapshot keeps `None` until it arrives.
        context.subscribe_optional(channels::common::NEXT_MANDATORY_BREAK)?;

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
                Self::log_configuration(context, configuration_event);
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

#[cfg(test)]
mod tests {
    use super::{shown, shown_fixed, with_unit};

    #[test]
    fn absent_sdk_values_render_as_n_a_without_units() {
        // ATS 1.62 omits `earned.xp` from `car_job.delivered`; it must not
        // appear as an authoritative zero.
        assert_eq!(shown::<i32>(None), "n/a");
        assert_eq!(shown(Some(0_i32)), "0");
        assert_eq!(shown_fixed(None, 3), "n/a");
        assert_eq!(shown_fixed(Some(0.009), 3), "0.009");
        assert_eq!(with_unit(shown_fixed(Some(115.0), 1), "km"), "115.0km");
        assert_eq!(with_unit(shown::<u32>(None), "km"), "n/a");
    }
}
