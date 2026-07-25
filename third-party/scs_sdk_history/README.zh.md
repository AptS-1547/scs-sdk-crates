# SCS SDK 历史来源追踪

**简体中文** | [English](README.md)

本仓库的 typed descriptor availability 元数据来源于官方 SCS SDK 1.0 到 1.14
压缩包。历史 header 用于确认每个 descriptor、association 或文档化 enum-like value
第一次出现时对应的 per-game telemetry schema。

解压后的历史压缩包是研究输入，不是构建依赖。当前 ABI 与常量的权威来源仍是
[`../scs_sdk_1_14/`](../scs_sdk_1_14/)。

## 许可证历史

所有已检查压缩包都使用相同的 MIT 风格许可正文，但保留了两种不同的 SCS Software
版权声明：

| SDK 压缩包 | 版权声明 | 保留文本 |
| --- | --- | --- |
| 1.0 到 1.5 | `Copyright (C) 2013 SCS Software` | [`licenses/LICENSE-SCS-SDK-2013`](licenses/LICENSE-SCS-SDK-2013) |
| 1.6 到 1.14 | `Copyright (C) 2016 SCS Software` | [`licenses/LICENSE-SCS-SDK-2016`](licenses/LICENSE-SCS-SDK-2016) |

每个公共 Rust crate 都会同时分发两份原始声明，因为其 schema-history 元数据可以追溯到
完整的 1.0 到 1.14 压缩包系列。两份文本只有版权年份不同；同时保留二者可以避免用
新版声明静默替换早期 SDK header 随附的声明。

## 官方压缩包清单

以下 SHA-256 记录历史审计使用的官方压缩包。所有 URL 均遵循 SCS Software 的官方
下载命名规则。

| SDK | 官方压缩包 | SHA-256 |
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

压缩包版本既不是 Telemetry API version，也不是游戏 telemetry schema version。
此清单只记录来源追踪；兼容性元数据继续使用 `scs-sdk` 实现的独立强版本域。
