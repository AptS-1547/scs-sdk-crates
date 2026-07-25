//! Safe application-boundary fixture for an SCS semantical input device.
//!
//! Semantical inputs map directly to game mixes with the same configuration
//! name. This fixture follows the official SDK 1.14 example by exposing one
//! bool input named `light`, which a fresh ETS2 or ATS controls file references
//! as `semantical.light?0`. Unlike a generic device, it requires no binding in
//! the game's controller UI.

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

use scs_sdk_plugin::sdk::{InputApiVersion, input};
use scs_sdk_plugin::{
    Game, InputDeviceId, InputDeviceSpec, InputDeviceType, InputEvent, InputEventRequest,
    InputGameCompatibility, InputIndex, InputPlugin, InputPluginCompatibility, InputPluginContext,
    InputSpec, InputValue, InputValueType, PluginMetadata, PluginResult,
};

/// Official semantical mix used by the SDK 1.14 sample.
static INPUTS: [InputSpec; 1] = [InputSpec::new("light", "Lights", InputValueType::Bool)];

static SUPPORTED_GAMES: [InputGameCompatibility; 2] = [
    InputGameCompatibility::new(Game::EuroTruckSimulator2, input::game::ets2::V1_00),
    InputGameCompatibility::new(Game::AmericanTruckSimulator, input::game::ats::V1_00),
];

/// A one-second transition at the game's usual 60 input frames per second is
/// slow enough to observe and fast enough for a short manual E2E run.
const TOGGLE_INTERVAL: u32 = 60;

#[derive(Default)]
struct Plugin {
    device: Option<InputDeviceId>,
    active: bool,
    next_input: u32,
    light: bool,
    frame: u32,
    logged_event_mask: u8,
}

impl Plugin {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn update_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.frame % TOGGLE_INTERVAL == 0 {
            self.light = !self.light;
        }
        self.next_input = 0;
    }

    fn event_for_current_position(&self) -> Option<InputEvent> {
        match self.next_input {
            0 => {
                InputIndex::new(0).map(|index| InputEvent::new(index, InputValue::Bool(self.light)))
            }
            _ => None,
        }
    }
}

impl InputPlugin for Plugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(
            "SCS SDK Semantical Input Example",
            env!("CARGO_PKG_VERSION"),
        )
    }

    fn compatibility(&self) -> InputPluginCompatibility {
        InputPluginCompatibility::new(InputApiVersion::V1_00, &SUPPORTED_GAMES)
    }

    fn initialize(&mut self, context: &mut InputPluginContext<'_>) -> PluginResult {
        self.reset();
        let device = context.register_device(
            InputDeviceSpec::new(
                "scs_sdk_input_semantical_example",
                "SCS SDK Semantical Input Example",
                InputDeviceType::Semantical,
                &INPUTS,
            )
            .with_activity_notifications(),
        )?;
        self.device = Some(device);
        context.message(format_args!(
            "[scs-sdk-input-semantical-example] registered device_type=semantical inputs={} mix=light value_type=bool toggle_interval_frames={TOGGLE_INTERVAL}",
            INPUTS.len(),
        ));
        Ok(())
    }

    fn device_active(
        &mut self,
        context: &mut InputPluginContext<'_>,
        device: InputDeviceId,
        active: bool,
    ) {
        if Some(device) != self.device {
            context.warning(format_args!(
                "[scs-sdk-input-semantical-example] ignored activity for unknown device {}",
                device.ordinal()
            ));
            return;
        }

        self.active = active;
        if active {
            // Each activation starts a new bounded evidence window while the
            // deterministic value itself continues from the plugin state.
            self.logged_event_mask = 0;
        }
        context.message(format_args!(
            "[scs-sdk-input-semantical-example] device active={active}"
        ));
    }

    fn next_input_event(
        &mut self,
        context: &mut InputPluginContext<'_>,
        request: InputEventRequest,
    ) -> Option<InputEvent> {
        if Some(request.device()) != self.device || !self.active {
            return None;
        }

        if request.flags().first_after_activation() {
            context.message(format_args!(
                "[scs-sdk-input-semantical-example] first poll after activation"
            ));
        }
        if request.flags().first_in_frame() {
            self.update_frame();
        }

        let event = self.event_for_current_position();
        if event.is_some() {
            if self.light && self.logged_event_mask & 0b010 == 0 {
                context.message(format_args!(
                    "[scs-sdk-input-semantical-example] emitted index=0 mix=light type=bool value=true"
                ));
                self.logged_event_mask |= 0b010;
            } else if !self.light && self.logged_event_mask & 0b001 == 0 {
                context.message(format_args!(
                    "[scs-sdk-input-semantical-example] emitted index=0 mix=light type=bool value=false"
                ));
                self.logged_event_mask |= 0b001;
            }
        } else if self.logged_event_mask & 0b100 == 0 {
            context.message(format_args!(
                "[scs-sdk-input-semantical-example] event sequence exhausted"
            ));
            self.logged_event_mask |= 0b100;
        }

        self.next_input = self.next_input.saturating_add(1);
        event
    }

    fn shutdown(&mut self, context: &mut InputPluginContext<'_>) {
        context.message(format_args!("[scs-sdk-input-semantical-example] shutdown"));
        self.reset();
    }
}

scs_sdk_plugin::export_input_plugin!(Plugin::default());

#[cfg(test)]
mod tests {
    use super::{Plugin, TOGGLE_INTERVAL};
    use scs_sdk_plugin::InputValue;

    #[test]
    fn light_starts_false_and_toggles_at_the_fixed_interval() {
        let mut plugin = Plugin::default();
        assert!(!plugin.light);

        for _ in 0..(TOGGLE_INTERVAL - 1) {
            plugin.update_frame();
        }
        assert!(!plugin.light);

        plugin.update_frame();
        assert!(plugin.light);

        for _ in 0..TOGGLE_INTERVAL {
            plugin.update_frame();
        }
        assert!(!plugin.light);
    }

    #[test]
    fn each_frame_starts_one_single_event_sequence() {
        let mut plugin = Plugin::default();
        plugin.update_frame();

        let first = plugin
            .event_for_current_position()
            .expect("position zero should produce the declared light event");
        assert_eq!(first.index().raw(), 0);
        assert_eq!(first.value(), InputValue::Bool(false));

        plugin.next_input = plugin.next_input.saturating_add(1);
        assert!(plugin.event_for_current_position().is_none());

        plugin.update_frame();
        assert!(plugin.event_for_current_position().is_some());
    }

    #[test]
    fn reset_discards_device_and_e2e_state() {
        let mut plugin = Plugin {
            active: true,
            next_input: 1,
            light: true,
            frame: 42,
            logged_event_mask: 0b111,
            ..Plugin::default()
        };

        plugin.reset();

        assert!(plugin.device.is_none());
        assert!(!plugin.active);
        assert_eq!(plugin.next_input, 0);
        assert!(!plugin.light);
        assert_eq!(plugin.frame, 0);
        assert_eq!(plugin.logged_event_mask, 0);
    }
}
