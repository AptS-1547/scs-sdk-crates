//! Independent compile-pass fixture for the input export macro.

#![forbid(unsafe_code)]
#![deny(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![deny(
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::unwrap_used
)]

use scs_sdk_plugin::sdk::{InputApiVersion, input};
use scs_sdk_plugin::{
    Game, InputDeviceSpec, InputDeviceType, InputGameCompatibility, InputPlugin,
    InputPluginCompatibility, InputPluginContext, InputSpec, InputValueType, PluginMetadata,
    PluginResult,
};

static INPUTS: [InputSpec; 1] = [InputSpec::new("button", "Button", InputValueType::Bool)];
static GAMES: [InputGameCompatibility; 1] = [InputGameCompatibility::new(
    Game::EuroTruckSimulator2,
    input::game::ets2::V1_00,
)];

#[derive(Default)]
struct Plugin;

impl InputPlugin for Plugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Input export fixture", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> InputPluginCompatibility {
        InputPluginCompatibility::new(InputApiVersion::V1_00, &GAMES)
    }

    fn initialize(&mut self, context: &mut InputPluginContext<'_>) -> PluginResult {
        context.register_device(InputDeviceSpec::new(
            "fixture_device",
            "Fixture Device",
            InputDeviceType::Generic,
            &INPUTS,
        ))?;
        Ok(())
    }
}

scs_sdk_plugin::export_input_plugin!(Plugin);
