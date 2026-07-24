//! Raw gameplay event and attribute names from the SCS Telemetry SDK.
//!
//! Documentation mirrors `scssdk_telemetry_common_gameplay_events.h`. Every
//! byte string is NUL-terminated for direct use with the C ABI.

/// Gameplay event identifiers.
pub mod events {
    /// Number of constants declared in this group.
    pub const COUNT: usize = 6;

    /// Event called when job is cancelled.
    ///
    /// Attributes:
    /// - `cancel_penalty`
    pub const JOB_CANCELLED: &[u8] = b"job.cancelled\0";

    /// Event called when job is delivered.
    ///
    /// Attributes:
    /// - revenue
    /// - `earned_xp`
    /// - `cargo_damage`
    /// - `distance_km`
    /// - `delivery_time`
    /// - `autopark_used`
    /// - `autoload_used`
    pub const JOB_DELIVERED: &[u8] = b"job.delivered\0";

    /// Event called when player gets fined.
    ///
    /// Attributes:
    /// - `fine_offence`
    /// - `fine_amount`
    pub const PLAYER_FINED: &[u8] = b"player.fined\0";

    /// Event called when player pays for a tollgate.
    ///
    /// Attributes:
    /// - `pay_amount`
    pub const PLAYER_TOLLGATE_PAID: &[u8] = b"player.tollgate.paid\0";

    /// Event called when player uses a ferry.
    ///
    /// Attributes:
    /// - `pay_amount`
    /// - `source_name`
    /// - `target_name`
    /// - `source_id`
    /// - `target_id`
    pub const PLAYER_USE_FERRY: &[u8] = b"player.use.ferry\0";

    /// Event called when player uses a train.
    ///
    /// Attributes:
    /// - `pay_amount`
    /// - `source_name`
    /// - `target_name`
    /// - `source_id`
    /// - `target_id`
    pub const PLAYER_USE_TRAIN: &[u8] = b"player.use.train\0";

    /// All raw gameplay event identifiers in official header order.
    pub const ALL: [&[u8]; COUNT] = [
        JOB_CANCELLED,
        JOB_DELIVERED,
        PLAYER_FINED,
        PLAYER_TOLLGATE_PAID,
        PLAYER_USE_FERRY,
        PLAYER_USE_TRAIN,
    ];
}

/// Attributes carried by gameplay events.
pub mod attributes {
    /// Number of constants declared in this group.
    pub const COUNT: usize = 15;

    /// The penalty for cancelling the job in native game currency. (Can be 0)
    ///
    /// Type: s64
    pub const CANCEL_PENALTY: &[u8] = b"cancel.penalty\0";

    /// The job revenue in native game currency.
    ///
    /// Type: s64
    pub const REVENUE: &[u8] = b"revenue\0";

    /// How much XP player received for the job.
    ///
    /// Type: s32
    pub const EARNED_XP: &[u8] = b"earned.xp\0";

    /// Total cargo damage. (Range <0.0, 1.0>)
    ///
    /// Type: float
    pub const CARGO_DAMAGE: &[u8] = b"cargo.damage\0";

    /// The real distance in km on the job.
    ///
    /// Type: float
    pub const DISTANCE_KM: &[u8] = b"distance.km\0";

    /// Total time spend on the job in game minutes.
    ///
    /// Type: u32
    pub const DELIVERY_TIME: &[u8] = b"delivery.time\0";

    /// Was auto parking used on this job?
    ///
    /// Type: bool
    pub const AUTO_PARK_USED: &[u8] = b"auto.park.used\0";

    /// Was auto loading used on this job? (always `true` for non cargo market jobs)
    ///
    /// Type: bool
    pub const AUTO_LOAD_USED: &[u8] = b"auto.load.used\0";

    /// Fine offence type.
    ///
    /// Possible values:
    /// - crash
    /// - `avoid_sleeping`
    /// - `wrong_way`
    /// - `speeding_camera`
    /// - `no_lights`
    /// - `red_signal`
    /// - speeding
    /// - `avoid_weighing`
    /// - `illegal_trailer`
    /// - `avoid_inspection`
    /// - `illegal_border_crossing`
    /// - `hard_shoulder_violation`
    /// - `damaged_vehicle_usage`
    /// - generic
    ///
    /// Type: string
    pub const FINE_OFFENCE: &[u8] = b"fine.offence\0";

    /// Fine offence amount in native game currency.
    ///
    /// Type: s64
    pub const FINE_AMOUNT: &[u8] = b"fine.amount\0";

    /// How much player was charged for this action (in native game currency)
    ///
    /// Type: s64
    pub const PAY_AMOUNT: &[u8] = b"pay.amount\0";

    /// The name of the transportation source.
    ///
    /// Type: string
    pub const SOURCE_NAME: &[u8] = b"source.name\0";

    /// The name of the transportation target.
    ///
    /// Type: string
    pub const TARGET_NAME: &[u8] = b"target.name\0";

    /// The id of the transportation source.
    ///
    /// Type: string
    pub const SOURCE_ID: &[u8] = b"source.id\0";

    /// The id of the transportation target.
    ///
    /// Type: string
    pub const TARGET_ID: &[u8] = b"target.id\0";

    /// All raw gameplay attribute names in official header order.
    pub const ALL: [&[u8]; COUNT] = [
        CANCEL_PENALTY,
        REVENUE,
        EARNED_XP,
        CARGO_DAMAGE,
        DISTANCE_KM,
        DELIVERY_TIME,
        AUTO_PARK_USED,
        AUTO_LOAD_USED,
        FINE_OFFENCE,
        FINE_AMOUNT,
        PAY_AMOUNT,
        SOURCE_NAME,
        TARGET_NAME,
        SOURCE_ID,
        TARGET_ID,
    ];
}

/// Total number of gameplay string macros covered by this module.
pub const COUNT: usize = events::COUNT + attributes::COUNT;

const _: [(); 21] = [(); COUNT];
