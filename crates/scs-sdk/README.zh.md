# `scs-sdk`

**中文** | [English](README.md)

`scs-sdk` 是建立在 [`scs-sdk-sys`](../scs-sdk-sys/) 之上的安全、typed、
`no_std` **SCS Telemetry SDK 1.14** 公共接口解释层。它把 header 形状的 ABI
转换为 typed Rust value、descriptor、catalog、版本域和 callback scope 内的 SDK
操作，但不接管插件生命周期或产品状态。

> 本 crate 覆盖 SDK 1.14 的公共 **telemetry** 接口。SDK 中的 input-device API
> 目前尚未由本 workspace 实现。

本 crate 是独立社区项目，与 SCS Software 不存在隶属或官方背书关系。

## 分层职责

本层负责：

- 将 raw SCS value 审计并解码为 typed Rust 表示；
- 分离 Telemetry API 与每款游戏 telemetry schema 的强版本类型；
- channel、configuration、gameplay、event 和 attribute descriptor；
- 生命周期不会超过 callback 数据的 borrowed callback view；
- callback scope 内回调 SCS 的注册、注销与日志能力；
- 带官方能力历史、可完整枚举的 catalog。

它刻意**不负责** loader export、进程级 runtime 状态、callback context 分配、注册事务、
panic containment 或产品 telemetry 状态。这些机制属于
[`scs-sdk-plugin`](../scs-sdk-plugin/) 或应用层。

本 crate 保持 `no_std`，不会仅为了 owned string 或 collection 的便利而引入
`std` 或 `alloc`。foreign 数据需要活过直接 callback scope 时，应由更高层完成复制。

## API 与版本模型

`TelemetryApi` 是适配 SCS 初始化结构的唯一 audited 入口。它精确支持：

```text
TelemetryApiVersion::V1_00
TelemetryApiVersion::V1_01
```

未知 raw 版本仍可被表示并用于诊断，但不会被静默解释成最新已知布局。
`TelemetryApiVersion` 描述与 loader 协商的插件 ABI；`GameSchemaVersion` 描述某款
游戏的 telemetry descriptor schema。二者是独立强类型，也都不等于 SDK 压缩包后缀
或公开游戏补丁版本。

关键 scope 类型包括：

- `TelemetryApi<'a>`：经过验证的初始化数据与已选择 ABI adapter；
- `TelemetrySession`：从有效 API 捕获的 callback 注册与日志函数；
- `SdkCall<'scope>`：只在 SCS 直接于游戏主线程调用插件时存在、不可逃逸的调用能力；
- `ScopedLogger<'scope>`：在一次允许的 SDK 调用 scope 内构造临时有界 C 字符串。

`SdkCall` 刻意不可存储，也不是 `Send` 或 `Sync`。安全 API 不会暗示 SDK 调用可以
保留或移动到 worker thread。每个官方非成功 result 都映射为独立 `SdkError` variant，
而不是丢失信息后统一变成 generic failure。

## Typed value 与 callback view

`ValueRef<'a>` 会先检查 raw SCS value tag，再读取对应 C union 成员。未知或不匹配
的 tag 返回 `None`，inactive union member 永远不会被读取。borrowed string 与
named value 受 foreign callback 生命周期约束。

Rust-owned geometry 类型 `FVector`、`DVector`、`Euler`、`FPlacement` 与
`DPlacement` 只复制有意义的字段，不会复制 SCS 没有义务初始化的 ABI padding。

`NamedValues<'a>` 为 configuration 与 gameplay payload 实现 SDK sentinel iteration
契约。它在 foreign sentinel 处终止并逐项验证，不会越界搜索任意内存。

## Descriptor、catalog 与 index

Typed descriptor 保留预期 value representation：

```rust
use scs_sdk::{Attribute, Channel};
```

`Channel<T>` 与 `Attribute<T>` 用于解码和注册；`AnyChannel` 与 `AnyAttribute`
则为枚举与诊断保留名称、value type、index 元数据和 availability。

三个 index 域刻意保持分离：

| 类型 | 含义 |
| --- | --- |
| `SdkIndex` | 附着于 indexed value 的显式 SCS array slot。 |
| `TrailerIndex` | `trailer.0` 等编号 trailer namespace。 |
| `TrailerConfigurationId` | 区分 legacy `trailer` 与编号 trailer configuration 的 callback identity。 |

应用 API 不应拿含义模糊的裸整数替代这些域。

完整 typed telemetry 清单为：

| 接口面 | 数量 |
| --- | ---: |
| Channels | 107 |
| Configuration IDs | 6 |
| Configuration attributes | 60 |
| Configuration-to-attribute associations | 71 |
| Gameplay events | 6 |
| Gameplay attributes | 15 |
| Gameplay-to-attribute associations | 21 |

文档化的 enum-like string 也有 closed known-value catalog：

| Catalog | 已知值数量 |
| --- | ---: |
| `configuration::ShifterType` | 4 |
| `configuration::JobMarket` | 5 |
| `gameplay::FineOffence` | 14 |

每个 closed catalog 都暴露 `ALL`、`COUNT`、`as_str`、`FromStr` 与 per-value
schema availability。known-value 解析失败不会销毁原文：generic string 入口仍然存在，
因此未来新增的 SDK value 依旧可以被记录和保留。

## 能力历史

每个内置 channel、configuration descriptor、gameplay descriptor 和 association
都携带 `GameSchemaAvailability`，分别记录经过考证的 ETS2 与 ATS 最低版本。这些值来自
官方历史 SDK header 及其 changelog 注释，不来自 SDK 压缩包编号或 Telemetry API。

representation-level 与 event-level API 历史同样保持分离。例如 signed 64-bit value
与 gameplay event 要求 Telemetry API 1.01，而具体 descriptor 还可能要求更晚的游戏
schema。

插件 framework 会把这些元数据与检测到的 `GameInfo` 组合，然后才请求 SCS 注册能力。

## Raw escape hatch

底层 ABI crate 被刻意暴露为：

```rust
pub use scs_sdk_sys as sys;
```

这是给 wrapper 与 runtime 开发使用的 escape hatch，不是普通应用路径。产品插件应使用
安全 descriptor 和 `scs-sdk-plugin::sdk` re-export，而不是导入 `sys` 类型。

## 验证

在仓库根目录运行：

```bash
cargo fmt --all -- --check
cargo test --locked -p scs-sdk
cargo clippy --locked -p scs-sdk --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk --no-deps
cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
```

修改公共 API、descriptor、注册、版本或 value 后，还必须证明 framework 仍能在不泄漏
raw 细节的前提下暴露 wrapper：

```bash
cargo test --locked -p scs-sdk-plugin
scripts/check-plugin-boundary.sh
```

## 许可证

Workspace 自有 Rust 代码可由你选择使用
[Apache License 2.0](LICENSE-APACHE) 或 [MIT](LICENSE-MIT)。

来源于 SDK 1.0 到 1.14 的 typed descriptor、常量、标识符、catalog、
schema-history 元数据与相关文档保留两份原始 SCS Software 声明：SDK 1.0-1.5 见
[LICENSE-SCS-SDK-2013](LICENSE-SCS-SDK-2013)，SDK 1.6-1.14 见
[LICENSE-SCS-SDK-2016](LICENSE-SCS-SDK-2016)。
[官方 SDK 压缩包](https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip)
仍是第三方材料，不会被重新许可为 workspace license。
