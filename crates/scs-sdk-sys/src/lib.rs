#![no_std]
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

#[cfg(not(target_pointer_width = "64"))]
compile_error!("scs-sdk-sys currently supports only 64-bit SCS game targets");

pub mod channels;
pub mod configuration;
pub mod constants;
pub mod gameplay;
pub mod games;
pub mod input;
pub mod telemetry;
pub mod value;

pub use constants::*;
pub use input::*;
pub use telemetry::*;
pub use value::*;
