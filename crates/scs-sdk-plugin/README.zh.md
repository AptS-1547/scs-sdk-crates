# `scs-sdk-plugin`

**中文** | [English](README.md)

`scs-sdk-plugin` 是 native SCS telemetry 插件的安全应用 framework 与 audited
runtime 边界。它把 typed [`scs-sdk`](../scs-sdk/) 与生命周期管理、事务式注册、
callback dispatch、panic containment 和稳定 foreign context ownership 组合起来，
让普通插件源码保持 safe Rust。

本 crate 刻意 re-export 应用需要的两个依赖：

```rust
pub use scs_sdk as sdk;
pub use scs_sdk_plugin_macros::export_plugin;
```

产品插件通常只需要在 manifest 中依赖 `scs-sdk-plugin`。

> Framework 覆盖公共 **SCS Telemetry SDK 1.14** 接口。SDK 中的 input-device
> API 目前尚未由本 workspace 实现。

本 crate 是独立社区项目，与 SCS Software 不存在隶属或官方背书关系。

## 应用契约

应用通过实现 `TelemetryPlugin` 显式处理四类职责：

1. `metadata` 返回稳定 `PluginMetadata`，其中包含生命周期日志使用的产品名称与
   build version；
2. `compatibility` 返回 `PluginCompatibility`，声明最低 Telemetry API 与每款游戏
   的 `GameCompatibility` schema 要求；
3. `initialize` 初始化产品状态，并逐个显式声明 event 和 channel subscription；
4. `channel`、`event` 与 `shutdown` 处理已送达数据和清理。

runtime 负责 ABI entry point 与 foreign callback plumbing。应用源码不需要原始指针、
C 字符串、手写 `extern` 函数、raw SCS union、直接访问 `scs-sdk-sys` 或使用
`unsafe` block。

## 最小安全插件

```rust
#![forbid(unsafe_code)]

use scs_sdk_plugin::sdk::{TelemetryApiVersion, channels, game};
use scs_sdk_plugin::{
    Game, GameCompatibility, PluginCompatibility, PluginContext,
    PluginMetadata, PluginResult, TelemetryPlugin,
};

static SUPPORTED_GAMES: [GameCompatibility; 1] = [GameCompatibility::new(
    Game::EuroTruckSimulator2,
    game::ets2::V1_00,
)];

#[derive(Default)]
struct Plugin;

impl TelemetryPlugin for Plugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("My telemetry plugin", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> PluginCompatibility {
        PluginCompatibility::new(TelemetryApiVersion::V1_00, &SUPPORTED_GAMES)
    }

    fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
        context.subscribe(channels::truck::SPEED)
    }
}

scs_sdk_plugin::export_plugin!(Plugin::default());
```

应用传入普通 constructor expression。proc macro 生成 SCS loader 要求的恰好两个符号，
并将二者委托给本 crate 的 runtime。

## 显式兼容性与游戏身份

`PluginCompatibility` 将 wrapper 能力与产品策略分开。runtime 会在产品初始化之前，
验证声明的最低 Telemetry API 与每款支持游戏的最低 schema。只有同一 major
兼容族内更高的 schema minor 会被接受；过低版本、不同 major、重复声明、未声明游戏
与含义模糊的 `Game::Other` 要求都会被明确拒绝。

`GameInfo` 会把只在 SCS 初始化期间有效的字符串复制为 owned Rust value，并把
identifier 分类为：

```text
Game::EuroTruckSimulator2
Game::AmericanTruckSimulator
Game::Other
```

未知 ID 会保留其原始文本 ID 与展示名称。应用可通过
`GameInfo::minimum_schema_for` 与 `GameInfo::supports` 查询 descriptor 历史，
无需自己再造一套 ETS2/ATS 版本策略。

Telemetry API version、游戏 telemetry schema 与公开游戏版本在初始化和日志中始终
保持为独立版本域。

## 显式订阅

`PluginContext` 不会从已实现 hook 推断 subscription，也不会自动订阅整个 catalog。
产品必须在 `initialize` 中逐项请求能力。

API 保留不同 registration domain：

- scalar channel：`subscribe` 与 `subscribe_with_flags`；
- SDK-indexed channel：使用 `SdkIndex` 的 `subscribe_at` 与
  `subscribe_at_with_flags`；
- 编号 trailer channel：使用 `TrailerIndex` 的 `subscribe_trailer`；
- 同时含编号 trailer 与 SDK index 的 channel：使用两个强 index 类型的
  `subscribe_trailer_at`。

每个 family 都有名称明确的 optional 形式。required 与 optional registration 具有
严格且不同的错误规则：

- optional channel 只容忍 `NotFound` 和 `UnsupportedType`；
- optional event 只容忍 `Unsupported` 和 `NotFound`；
- 其他任何失败都会中止初始化并保持事务 rollback。

framework 会在调用 SDK 前检查 Telemetry API 与 per-game schema availability。
更新的 optional 能力会被跳过；更新的 required 能力会拒绝初始化。commit 后报告的
数量只包含 SCS 实际接受的注册。

## 安全 callback 接口

Callback 接收 framework 提供的安全 view：

| 类型 | 用途 |
| --- | --- |
| `ChannelUpdate<'a>` | 已注册 channel identity、flag、强 index 与 typed value 解码。 |
| `TelemetryEvent<'a>` | frame、pause/start、configuration 与 gameplay callback payload。 |
| `ConfigurationEvent<'a>` | Typed configuration attribute 与 legacy/编号 trailer identity。 |
| `GameplayEvent<'a>` | Typed gameplay attribute 与 known-value helper。 |

borrowed payload 不会活过 hook invocation。generic `string` 与 `string_owned` 入口
和 `shifter_type`、`job_market`、`fine_offence` 等 helper 同时存在，因此未来未知
SDK 字符串不会被静默折叠为某个已知值。

`TelemetryEventKind` 是 `scs_sdk::Event` 的 re-export，不是重复的 event catalog。
registration identity 与 callback payload 因而保持不同类型，又无需维护两套 raw
discriminator mapping。

## Runtime 保证

audited runtime 提供产品插件不应重复实现的共享生命周期机制：

- 确定性的 idle、initializing、active 与 shutdown 状态转换；
- 注册作为事务执行，部分失败时按完成顺序的逆序 rollback；
- 正常 shutdown 按逆序 unregister；
- active teardown 与被拒绝的部分初始化都会调用插件 shutdown；
- 每个 foreign ABI 边界之前都有 panic containment；
- 显式恢复 poisoned mutex，而不是使用 `unwrap` 或 `expect`；
- 为 foreign-visible callback context 提供地址稳定的 `Arc<Registration>` allocation；
- generation check 隔离上一 session 延迟到达的 callback；
- SDK unregistration 失败时保留 runtime backlink 与 callback context；
- callback scope SDK 调用留在 SCS 串行化的游戏主线程。

仅仅在逻辑上 inactive，并不等于 SCS 已经丢弃某个 pointer。这个区别是 context
provenance 与 stale-callback safety 的关键。

Runtime 日志会标识插件名称和版本、framework 版本、检测到的游戏展示名称和 ID、
协商的 Telemetry API、游戏 schema、已 commit 的 event/channel 数量、初始化失败与
干净 shutdown。产品 probe line 和业务诊断仍由应用负责。

## 范围边界

此 crate 是可复用基础设施，不实现 web transport、dispatch 行为、shared-memory
protocol、save-game 解析、持久化或其他产品功能。产品仓库应在安全 hook 之上实现这些
内容，而不是把它们塞进 framework。

真实应用边界 fixture 见
[`examples/telemetry-plugin`](../../examples/telemetry-plugin/)，独立 consumer macro
fixture 位于 [`tests/fixtures/export-plugin`](tests/fixtures/export-plugin/)。

## 验证

在仓库根目录运行：

```bash
cargo fmt --all -- --check
scripts/check-plugin-boundary.sh
scripts/check-plugin-macro-fixtures.sh
cargo test --locked -p scs-sdk-plugin
cargo clippy --locked -p scs-sdk-plugin --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk-plugin --no-deps
MIRIFLAGS=-Zmiri-strict-provenance \
  cargo +nightly-2026-04-12 miri test --locked -p scs-sdk-plugin
```

修改 export、macro integration、package 名称或 release linkage 时，还需要运行仓库的
Windows、Linux、macOS release artifact 与 symbol verification 脚本。

## 许可证

Workspace 自有 Rust 代码可由你选择使用
[Apache License 2.0](LICENSE-APACHE) 或 [MIT](LICENSE-MIT)。

来源于 SDK 1.0 到 1.14 的 lifecycle contract、ABI 标识符、compatibility
元数据与相关文档保留两份原始 SCS Software 声明：SDK 1.0-1.5 见
[LICENSE-SCS-SDK-2013](LICENSE-SCS-SDK-2013)，SDK 1.6-1.14 见
[LICENSE-SCS-SDK-2016](LICENSE-SCS-SDK-2016)。
[官方 SDK 压缩包](https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip)
仍是第三方材料，不会被重新许可为 workspace license。
