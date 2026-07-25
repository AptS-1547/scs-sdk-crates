# SCS SDK historical-source provenance

[简体中文](README.zh.md) | **English**

The typed descriptor availability metadata in this repository was derived from
the official SCS SDK archives from version 1.0 through 1.14. The historical
headers were used as evidence for the first per-game telemetry schema which
contained each descriptor, association, or documented enum-like value.

The extracted archives are research inputs rather than build dependencies. The
current ABI and constant source of truth remains
[`../scs_sdk_1_14/`](../scs_sdk_1_14/).

## License history

All inspected archives use the same MIT-style permission text, but the retained
SCS Software copyright notice has two distinct forms:

| SDK archives | Copyright notice | Preserved text |
| --- | --- | --- |
| 1.0 through 1.5 | `Copyright (C) 2013 SCS Software` | [`licenses/LICENSE-SCS-SDK-2013`](licenses/LICENSE-SCS-SDK-2013) |
| 1.6 through 1.14 | `Copyright (C) 2016 SCS Software` | [`licenses/LICENSE-SCS-SDK-2016`](licenses/LICENSE-SCS-SDK-2016) |

Both original notices are distributed with every public Rust crate because its
schema-history metadata can be traced across the complete 1.0 through 1.14
archive series. The two texts differ only in the copyright year; retaining both
avoids silently replacing the notice attached to the earlier SDK archives.

## Official archive inventory

The following SHA-256 values record the official archives used for the history
audit. Each URL follows SCS Software's official download naming convention.

| SDK | Official archive | SHA-256 |
| --- | --- | --- |
| 1.0 | <https://download.eurotrucksimulator2.com/scs_sdk_1_0.zip> | `04d742e628c22c2d6bf0a7ebe03e04450024ed2c514365595d84e87bfa33462b` |
| 1.1 | <https://download.eurotrucksimulator2.com/scs_sdk_1_1.zip> | `0357a786f6a1ecfd419be580f7e88167694a26b7aef087936021d3bcc840748d` |
| 1.2 | <https://download.eurotrucksimulator2.com/scs_sdk_1_2.zip> | `4ce37d135c599e8a9e96abb6b38a3a7b3521b86b2a62d285a2b88894ccf0d023` |
| 1.3 | <https://download.eurotrucksimulator2.com/scs_sdk_1_3.zip> | `f1aef9699f519a7aac3aff1bc53615eccf654c60b625fb836c7c60d77f563f64` |
| 1.4 | <https://download.eurotrucksimulator2.com/scs_sdk_1_4.zip> | `0eff0be84b43b50cb667556a980ce6a8b2bc13a0c6741363485409ec230b5beb` |
| 1.5 | <https://download.eurotrucksimulator2.com/scs_sdk_1_5.zip> | `addb9fec9d0851db4ba64c3012905db2e31c9e96ff4e97f5ede163a36dabb073` |
| 1.6 | <https://download.eurotrucksimulator2.com/scs_sdk_1_6.zip> | `8e42b67819acf241813530ef522ecbd494f3c84a05fcd924d1a1f87f6befcef6` |
| 1.7 | <https://download.eurotrucksimulator2.com/scs_sdk_1_7.zip> | `ede8191858dec8888ce09c70549bdd18637fd16e1f7cab22ee8b00d43e30d2a9` |
| 1.8 | <https://download.eurotrucksimulator2.com/scs_sdk_1_8.zip> | `426d2dbad31676d55fa9af5296d2720aff336480eeaf876c0663a61a02e78d1f` |
| 1.9 | <https://download.eurotrucksimulator2.com/scs_sdk_1_9.zip> | `dbb5d3d6382a4cd090090ec71222c2ca28811269c96e3be65e2139e5418f784a` |
| 1.10 | <https://download.eurotrucksimulator2.com/scs_sdk_1_10.zip> | `ae7571330a4d098ab2fd185977b41a04aeeb9a55deb2b558cc2da8ce81cb0b1c` |
| 1.11 | <https://download.eurotrucksimulator2.com/scs_sdk_1_11.zip> | `5b48998c473ba0e2e1a6b6610024fed0d1b9f2e5b0e194495d412e6c8be6a33f` |
| 1.12 | <https://download.eurotrucksimulator2.com/scs_sdk_1_12.zip> | `2afb46c1c946b39a5ea16e043d4b95eef90911a2c0399ab9c0ae73326d0a6076` |
| 1.13 | <https://download.eurotrucksimulator2.com/scs_sdk_1_13.zip> | `fd4a0baac5f94f4a2a2167067f68aeaaf0fe54daa597bdb9ae91b2934c5ed6a6` |
| 1.14 | <https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip> | `c6c1f7376b7324994d9f9c567f3c4141fbbf305b6bf803bc4cfeef2437b2023a` |

The archive version is not a Telemetry API version or a game telemetry schema
version. This inventory records source provenance only; compatibility metadata
continues to use the distinct typed version domains implemented by `scs-sdk`.
