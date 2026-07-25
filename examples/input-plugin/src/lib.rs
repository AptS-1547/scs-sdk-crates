//! Safe application-boundary example for the SCS Input Device API.
//!
//! The example exposes one generic device with a float axis and a bool button.
//! It intentionally uses deterministic in-process state rather than real
//! hardware so the same artifact can be loaded by ETS2 or ATS as an end-to-end
//! framework fixture.

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
    Game, InputAxisValue, InputAxisValueError, InputDeviceId, InputDeviceSpec, InputDeviceType,
    InputEvent, InputEventRequest, InputGameCompatibility, InputIndex, InputPlugin,
    InputPluginCompatibility, InputPluginContext, InputSpec, InputValue, InputValueType,
    PluginMetadata, PluginResult,
};

static INPUTS: [InputSpec; 2] = [
    InputSpec::new("example_axis", "Example Axis", InputValueType::Float),
    InputSpec::new("example_button", "Example Button", InputValueType::Bool),
];

static SUPPORTED_GAMES: [InputGameCompatibility; 2] = [
    InputGameCompatibility::new(Game::EuroTruckSimulator2, input::game::ets2::V1_00),
    InputGameCompatibility::new(Game::AmericanTruckSimulator, input::game::ats::V1_00),
];

const AXIS_MIN: InputAxisValue = InputAxisValue::MIN;
const AXIS_MAX: InputAxisValue = InputAxisValue::MAX;
#[cfg(test)]
const AXIS_STEP_COUNT: u32 = 16;

const AXIS_STEP: f32 = 0.125;
const AXIS_UPDATE_INTERVAL: u32 = 60;

struct Plugin {
    device: Option<InputDeviceId>,
    active: bool,
    next_input: u32,
    axis: InputAxisValue,
    button: bool,
    frame: u32,
    logged_event_mask: u8,
    logged_exhaustion: bool,
}

impl Default for Plugin {
    fn default() -> Self {
        Self {
            device: None,
            active: false,
            next_input: 0,
            axis: AXIS_MIN,
            button: false,
            frame: 0,
            logged_event_mask: 0,
            logged_exhaustion: false,
        }
    }
}

impl Plugin {
    fn reset(&mut self) {
        self.device = None;
        self.active = false;
        self.next_input = 0;
        self.axis = AXIS_MIN;
        self.button = false;
        self.frame = 0;
        self.logged_event_mask = 0;
        self.logged_exhaustion = false;
    }

    fn update_frame(&mut self) -> Result<(), InputAxisValueError> {
        self.frame = self.frame.wrapping_add(1);
        // Retain the official sample's exact 0.125 step and low update rate,
        // but extend this fixture across the complete normalized signed range
        // so real-game E2E exercises negative values as well. Constructing the
        // next position through `InputAxisValue` keeps the application's own
        // state under the same contract as the value returned to the runtime.
        if self.frame % AXIS_UPDATE_INTERVAL == 0 {
            if self.axis == AXIS_MAX {
                self.axis = AXIS_MIN;
            } else {
                self.axis = InputAxisValue::new(self.axis.get() + AXIS_STEP)?;
            }
        }
        if self.frame % 120 == 0 {
            self.button = !self.button;
        }
        self.next_input = 0;
        Ok(())
    }
}

impl InputPlugin for Plugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("SCS SDK Input Example", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> InputPluginCompatibility {
        InputPluginCompatibility::new(InputApiVersion::V1_00, &SUPPORTED_GAMES)
    }

    fn initialize(&mut self, context: &mut InputPluginContext<'_>) -> PluginResult {
        self.reset();
        let device = context.register_device(
            InputDeviceSpec::new(
                "scs_sdk_input_example",
                "SCS SDK Input Example",
                InputDeviceType::Generic,
                &INPUTS,
            )
            .with_activity_notifications(),
        )?;
        self.device = Some(device);
        context.message(format_args!(
            "[scs-sdk-input-example] registered generic device with {} inputs axis_range=[{AXIS_MIN:.3}, {AXIS_MAX:.3}]",
            INPUTS.len(),
            AXIS_MIN = AXIS_MIN.get(),
            AXIS_MAX = AXIS_MAX.get(),
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
                "[scs-sdk-input-example] ignored activity for unknown device {}",
                device.ordinal()
            ));
            return;
        }
        self.active = active;
        if active {
            // Keep the real-game evidence bounded to one complete event
            // sequence per activation. A later deactivation/activation cycle
            // deliberately produces a fresh sequence for repeatable E2E runs.
            self.logged_event_mask = 0;
            self.logged_exhaustion = false;
        }
        context.message(format_args!(
            "[scs-sdk-input-example] device active={active}"
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
                "[scs-sdk-input-example] first poll after activation"
            ));
        }
        if request.flags().first_in_frame() {
            if let Err(error) = self.update_frame() {
                context.error(format_args!(
                    "[scs-sdk-input-example] failed to advance normalized axis: {error}"
                ));
                return None;
            }
        }

        let event_position = self.next_input;
        let event = match event_position {
            0 => {
                InputIndex::new(0).map(|index| InputEvent::new(index, InputValue::Float(self.axis)))
            }
            1 => InputIndex::new(1)
                .map(|index| InputEvent::new(index, InputValue::Bool(self.button))),
            _ => None,
        };
        match event_position {
            0 if self.logged_event_mask & 0b01 == 0 => {
                context.message(format_args!(
                    "[scs-sdk-input-example] emitted index=0 type=float value={:.3}",
                    self.axis.get()
                ));
                self.logged_event_mask |= 0b01;
            }
            1 if self.logged_event_mask & 0b10 == 0 => {
                context.message(format_args!(
                    "[scs-sdk-input-example] emitted index=1 type=bool value={}",
                    self.button
                ));
                self.logged_event_mask |= 0b10;
            }
            _ if event.is_none() && !self.logged_exhaustion => {
                context.message(format_args!(
                    "[scs-sdk-input-example] event sequence exhausted"
                ));
                self.logged_exhaustion = true;
            }
            _ => {}
        }
        self.next_input = self.next_input.saturating_add(1);
        event
    }

    fn shutdown(&mut self, context: &mut InputPluginContext<'_>) {
        context.message(format_args!("[scs-sdk-input-example] shutdown"));
        self.reset();
    }
}

scs_sdk_plugin::export_input_plugin!(Plugin::default());

#[cfg(test)]
mod tests {
    use super::{AXIS_MAX, AXIS_MIN, AXIS_STEP, AXIS_STEP_COUNT, AXIS_UPDATE_INTERVAL, Plugin};

    #[test]
    fn synthetic_axis_starts_at_minimum_and_stays_in_the_selected_probe_range() {
        let mut plugin = Plugin::default();
        assert_eq!(plugin.axis.get().to_bits(), AXIS_MIN.get().to_bits());

        for _ in 0..10_000 {
            plugin
                .update_frame()
                .expect("fixture step should stay normalized");
            assert!(plugin.axis.get().is_finite());
            assert!((AXIS_MIN.get()..=AXIS_MAX.get()).contains(&plugin.axis.get()));
        }
    }

    #[test]
    fn synthetic_axis_advances_slowly_and_resets_after_the_maximum() {
        let mut plugin = Plugin::default();

        for _ in 0..(AXIS_UPDATE_INTERVAL - 1) {
            plugin
                .update_frame()
                .expect("fixture step should stay normalized");
        }
        assert_eq!(plugin.axis.get().to_bits(), AXIS_MIN.get().to_bits());

        plugin
            .update_frame()
            .expect("fixture step should stay normalized");
        assert_eq!(
            plugin.axis.get().to_bits(),
            (AXIS_MIN.get() + AXIS_STEP).to_bits()
        );

        for _ in 0..(AXIS_UPDATE_INTERVAL * (AXIS_STEP_COUNT - 1)) {
            plugin
                .update_frame()
                .expect("fixture step should stay normalized");
        }
        assert_eq!(plugin.axis.get().to_bits(), AXIS_MAX.get().to_bits());

        for _ in 0..AXIS_UPDATE_INTERVAL {
            plugin
                .update_frame()
                .expect("fixture step should stay normalized");
        }
        assert_eq!(plugin.axis.get().to_bits(), AXIS_MIN.get().to_bits());
    }

    #[test]
    fn reset_restores_the_initial_input_state() {
        let mut plugin = Plugin::default();
        for _ in 0..120 {
            plugin
                .update_frame()
                .expect("fixture step should stay normalized");
        }
        assert_ne!(plugin.axis.get().to_bits(), AXIS_MIN.get().to_bits());
        assert!(plugin.button);

        plugin.reset();

        assert_eq!(plugin.axis.get().to_bits(), AXIS_MIN.get().to_bits());
        assert!(!plugin.button);
        assert_eq!(plugin.frame, 0);
        assert_eq!(plugin.next_input, 0);
    }
}
