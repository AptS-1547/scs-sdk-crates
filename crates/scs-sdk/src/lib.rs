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

mod api;
mod channel;
pub mod configuration;
mod descriptor;
mod error;
mod event;
pub mod game;
pub mod gameplay;
mod index;
mod value;
mod version;

pub use api::{LogLevel, ScopedLogger, SdkCall, TelemetryApi, TelemetrySession};
pub use channel::{AnyChannel, Channel, ChannelFlags, ChannelValue, channels};
pub use descriptor::{
    AnyAttribute, Attribute, ConfigurationAttributeAssociation, ConfigurationId,
    GameSchemaAvailability, GameplayAttributeAssociation, GameplayEventId,
};
pub use error::{SdkError, SdkResult};
pub use event::{
    ConfigurationRef, Event, FrameStartRef, GameplayEventRef, NamedValueRef, NamedValues,
};
pub use index::{SdkIndex, TrailerConfigurationId, TrailerIndex};
pub use value::{
    DPlacement, DVector, Euler, FPlacement, FVector, SdkValue, StringValue, UnknownStringValue,
    ValueRef, ValueType,
};
pub use version::{GameSchemaVersion, TelemetryApiVersion};

pub use scs_sdk_sys as sys;
