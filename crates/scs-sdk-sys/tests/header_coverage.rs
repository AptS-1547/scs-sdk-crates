//! Mechanical coverage audit against the vendored official SDK headers.
//!
//! Every string macro and game-version macro declared by the official SDK
//! 1.16 headers under `third-party/scs_sdk_1_16/include` must appear in the
//! matching raw catalog, in official header order, and no catalog may contain
//! a name that the headers do not declare.
//!
//! The headers are workspace-only third-party material, so this test is
//! excluded from the published package (see this crate's `Cargo.toml`).

use scs_sdk_sys::{channels, configuration, gameplay, games};

macro_rules! header {
    ($path:literal) => {
        (
            $path,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../third-party/scs_sdk_1_16/include/",
                $path
            )),
        )
    };
}

const HEADERS: [(&str, &str); 11] = [
    header!("scssdk.h"),
    header!("eurotrucks2/scssdk_eut2.h"),
    header!("amtrucks/scssdk_ats.h"),
    header!("common/scssdk_telemetry_common_channels.h"),
    header!("common/scssdk_telemetry_truck_common_channels.h"),
    header!("common/scssdk_telemetry_trailer_common_channels.h"),
    header!("common/scssdk_telemetry_job_common_channels.h"),
    header!("common/scssdk_telemetry_common_configs.h"),
    header!("common/scssdk_telemetry_common_gameplay_events.h"),
    header!("eurotrucks2/scssdk_telemetry_eut2.h"),
    header!("amtrucks/scssdk_telemetry_ats.h"),
];

/// One `#define NAME VALUE` line, in header order.
struct Define<'a> {
    header: &'a str,
    name: &'a str,
    value: &'a str,
}

fn defines() -> Vec<Define<'static>> {
    let mut found = Vec::new();
    for (header, text) in HEADERS {
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("#define") else {
                continue;
            };
            let mut parts = rest.trim_start().splitn(2, char::is_whitespace);
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                continue;
            };
            let value = value.split("//").next().unwrap_or_default().trim();
            found.push(Define {
                header,
                name,
                value,
            });
        }
    }
    found
}

/// Returns the unquoted string values of macros selected by `select`.
fn header_strings(select: impl Fn(&str) -> bool) -> Vec<&'static str> {
    defines()
        .into_iter()
        .filter(|define| select(define.name))
        .map(|define| {
            define
                .value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .unwrap_or_else(|| panic!("{} is not a string macro", define.name))
        })
        .collect()
}

fn catalog(raw: &[&[u8]]) -> Vec<&'static str> {
    raw.iter()
        .map(|name| {
            let bytes = name
                .strip_suffix(b"\0")
                .unwrap_or_else(|| panic!("{name:?} is not NUL-terminated"));
            let text = core::str::from_utf8(bytes).expect("catalog names are UTF-8");
            // Leak once per test run so the comparison can use one string type.
            &*Box::leak(text.to_owned().into_boxed_str())
        })
        .collect()
}

fn assert_catalog(group: &str, select: impl Fn(&str) -> bool, raw: &[&[u8]]) {
    let expected = header_strings(select);
    let actual = catalog(raw);
    assert!(
        !expected.is_empty(),
        "{group}: header selection matched nothing"
    );
    assert_eq!(
        actual, expected,
        "{group}: raw catalog must match the SDK 1.16 header exactly and in order"
    );
}

#[test]
fn channel_catalogs_match_the_headers() {
    assert_catalog(
        "common channels",
        |name| name.starts_with("SCS_TELEMETRY_CHANNEL_"),
        &channels::common::ALL,
    );
    assert_catalog(
        "truck channels",
        |name| name.starts_with("SCS_TELEMETRY_TRUCK_CHANNEL_"),
        &channels::truck::ALL,
    );
    assert_catalog(
        "trailer channels",
        |name| name.starts_with("SCS_TELEMETRY_TRAILER_CHANNEL_"),
        &channels::trailer::ALL,
    );
    assert_catalog(
        "job channels",
        |name| name.starts_with("SCS_TELEMETRY_JOB_CHANNEL_"),
        &channels::job::ALL,
    );
}

#[test]
fn configuration_catalogs_match_the_headers() {
    assert_catalog(
        "configuration ids",
        |name| {
            name.starts_with("SCS_TELEMETRY_CONFIG_")
                && !name.starts_with("SCS_TELEMETRY_CONFIG_ATTRIBUTE_")
        },
        &configuration::ids::ALL,
    );
    assert_catalog(
        "configuration attributes",
        |name| name.starts_with("SCS_TELEMETRY_CONFIG_ATTRIBUTE_"),
        &configuration::attributes::ALL,
    );
    assert_catalog(
        "shifter types",
        |name| name.starts_with("SCS_SHIFTER_TYPE_"),
        &configuration::shifter_types::ALL,
    );
}

#[test]
fn gameplay_catalogs_match_the_headers() {
    assert_catalog(
        "gameplay events",
        |name| {
            name.starts_with("SCS_TELEMETRY_GAMEPLAY_EVENT_")
                && !name.starts_with("SCS_TELEMETRY_GAMEPLAY_EVENT_ATTRIBUTE_")
        },
        &gameplay::events::ALL,
    );
    assert_catalog(
        "gameplay attributes",
        |name| name.starts_with("SCS_TELEMETRY_GAMEPLAY_EVENT_ATTRIBUTE_"),
        &gameplay::attributes::ALL,
    );
}

#[test]
fn every_header_string_macro_is_audited() {
    // Printf helpers are the only string macros without a runtime meaning.
    let audited_prefixes = [
        "SCS_TELEMETRY_CHANNEL_",
        "SCS_TELEMETRY_TRUCK_CHANNEL_",
        "SCS_TELEMETRY_TRAILER_CHANNEL_",
        "SCS_TELEMETRY_JOB_CHANNEL_",
        "SCS_TELEMETRY_CONFIG_",
        "SCS_SHIFTER_TYPE_",
        "SCS_TELEMETRY_GAMEPLAY_EVENT_",
        "SCS_GAME_ID_",
        "SCS_PF_",
    ];
    for define in defines() {
        if define.value.starts_with('"') {
            assert!(
                audited_prefixes
                    .iter()
                    .any(|prefix| define.name.starts_with(prefix)),
                "{} in {} is a string macro outside every audited catalog",
                define.name,
                define.header
            );
        }
    }
    assert_eq!(
        header_strings(|name| name == "SCS_GAME_ID_EUT2"),
        catalog(&[scs_sdk_sys::SCS_GAME_ID_EUT2])
    );
    assert_eq!(
        header_strings(|name| name == "SCS_GAME_ID_ATS"),
        catalog(&[scs_sdk_sys::SCS_GAME_ID_ATS])
    );
}

/// Parses `SCS_MAKE_VERSION(major, minor)` into the packed SDK value.
fn make_version(value: &str) -> u32 {
    let inner = value
        .strip_prefix("SCS_MAKE_VERSION(")
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or_else(|| panic!("{value} is not SCS_MAKE_VERSION"));
    let (major, minor) = inner.split_once(',').expect("major and minor");
    let major: u32 = major.trim().parse().expect("numeric major");
    let minor: u32 = minor.trim().parse().expect("numeric minor");
    scs_sdk_sys::make_version(major, minor)
}

/// Returns `(declared versions, resolved CURRENT)` for one game-version prefix.
fn game_versions(prefix: &str) -> (Vec<u32>, u32) {
    let all = defines();
    let current_name = format!("{prefix}CURRENT");
    let versions: Vec<u32> = all
        .iter()
        .filter(|define| define.name.starts_with(prefix) && define.name != current_name)
        .map(|define| make_version(define.value))
        .collect();
    let current_alias = all
        .iter()
        .find(|define| define.name == current_name)
        .map_or_else(
            || panic!("{current_name} is missing"),
            |define| define.value,
        );
    let current = all
        .iter()
        .find(|define| define.name == current_alias)
        .map_or_else(
            || panic!("{current_alias} is missing"),
            |define| make_version(define.value),
        );
    (versions, current)
}

#[test]
fn game_versions_match_the_headers() {
    let (ets2, ets2_current) = game_versions("SCS_TELEMETRY_EUT2_GAME_VERSION_");
    assert_eq!(ets2.len(), 22, "ETS2 declares versions 1.00 through 1.21");
    assert_eq!(games::ets2::VERSION_CURRENT, ets2_current);
    assert_eq!(ets2.iter().max(), Some(&games::ets2::VERSION_CURRENT));

    let (ats, ats_current) = game_versions("SCS_TELEMETRY_ATS_GAME_VERSION_");
    assert_eq!(ats.len(), 9, "ATS declares versions 1.00 through 1.08");
    assert_eq!(games::ats::VERSION_CURRENT, ats_current);
    assert_eq!(ats.iter().max(), Some(&games::ats::VERSION_CURRENT));
}
