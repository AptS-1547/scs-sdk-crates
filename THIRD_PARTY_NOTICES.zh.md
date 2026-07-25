# 第三方声明

**简体中文** | [English](THIRD_PARTY_NOTICES.md)

## SCS SDK 1.0 到 1.14

本仓库与官方 SCS SDK 1.0 到 1.14 历史压缩包系列互操作，部分 ABI 声明、常量、
标识符、catalog 和相关文档来源于这些 SDK。历史 header 用于推导 per-game schema
availability；SDK 1.14 仍是当前 ABI 与常量的权威来源。

- SDK 1.0 到 1.5 版权所有：`Copyright (C) 2013 SCS Software`
- SDK 1.6 到 1.14 版权所有：`Copyright (C) 2016 SCS Software`
- 压缩包 URL、checksum 与许可证映射：
  [`third-party/scs_sdk_history/`](third-party/scs_sdk_history/)
- 当前 vendored SDK：
  [`third-party/scs_sdk_1_14/`](third-party/scs_sdk_1_14/)
- 用于分发的两份原始声明：
  [`LICENSE-SCS-SDK-2013`](LICENSE-SCS-SDK-2013) 与
  [`LICENSE-SCS-SDK-2016`](LICENSE-SCS-SDK-2016)

SCS SDK 许可证允许使用、修改、发布、分发、再许可和销售，但在复制或分发 SDK
材料的实质性部分时，需要保留 SCS Software 的版权与许可声明。完整上游声明未经修改地
保存在 [`LICENSE-SCS-SDK-2013`](LICENSE-SCS-SDK-2013) 与
[`LICENSE-SCS-SDK-2016`](LICENSE-SCS-SDK-2016) 中。

Workspace 自行编写的 Rust 代码另行采用 [MIT](LICENSE-MIT) 或
[Apache-2.0](LICENSE-APACHE)，由接收者任选其一。该 workspace 许可声明不会重新许可
官方 SDK 文件，也不会移除其归属要求。

本项目是独立社区项目，与 SCS Software 不存在隶属或官方背书关系。文中出现的
SCS Software、Euro Truck Simulator 2、American Truck Simulator、ETS2 与 ATS
名称仅用于标识兼容的软件和接口。
