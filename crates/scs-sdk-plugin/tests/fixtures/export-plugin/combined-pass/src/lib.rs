//! Independent fixture proving both SCS API macros coexist in one cdylib.

#![forbid(unsafe_code)]

use scs_sdk_plugin::sdk::{InputApiVersion, TelemetryApiVersion, game, input};
use scs_sdk_plugin::{
    Game, GameCompatibility, InputGameCompatibility, InputPlugin, InputPluginCompatibility,
    InputPluginContext, PluginCompatibility, PluginContext, PluginMetadata, PluginResult,
    TelemetryPlugin,
};

static TELEMETRY_GAMES: [GameCompatibility; 1] = [GameCompatibility::new(
    Game::EuroTruckSimulator2,
    game::ets2::V1_00,
)];
static INPUT_GAMES: [InputGameCompatibility; 1] = [InputGameCompatibility::new(
    Game::EuroTruckSimulator2,
    input::game::ets2::V1_00,
)];

#[derive(Default)]
struct TelemetryFixture;

impl TelemetryPlugin for TelemetryFixture {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Combined telemetry fixture", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> PluginCompatibility {
        PluginCompatibility::new(TelemetryApiVersion::V1_00, &TELEMETRY_GAMES)
    }

    fn initialize(&mut self, _context: &mut PluginContext<'_>) -> PluginResult {
        Ok(())
    }
}

#[derive(Default)]
struct InputFixture;

impl InputPlugin for InputFixture {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Combined input fixture", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> InputPluginCompatibility {
        InputPluginCompatibility::new(InputApiVersion::V1_00, &INPUT_GAMES)
    }

    fn initialize(&mut self, _context: &mut InputPluginContext<'_>) -> PluginResult {
        Ok(())
    }
}

scs_sdk_plugin::export_plugin!(TelemetryFixture);
scs_sdk_plugin::export_input_plugin!(InputFixture);
