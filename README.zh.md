# SCS SDK Rust Crates

[English](README.md) | **简体中文**

SCS Software SDK 的类型化 Rust bindings 与 safe plugin framework。当前 workspace 完整覆盖 Telemetry SDK 1.14 的公开接口，并保留一个真实 ETS2 plugin example，用于验证应用边界与跨平台 loader 产物。具体产品插件放在独立仓库中。

当前基座使用纯 Rust 实现，不需要 C++ shim、CMake 或 bindgen。官方 SDK 原始分发仍保留在 `third-party/scs_sdk_1_14/`，它是 ABI 与常量的权威来源。

## 基座目标

当前设计坚持以下边界：

1. **完整覆盖 SDK 1.14**：公开 telemetry headers 中的 ABI、channel、configuration、gameplay event、游戏标识和版本常量均进入 Rust 层。
2. **产品意图必须显式**：框架不会因为插件实现了某个回调，就猜测它想订阅对应事件，也不会自动订阅整个 channel catalog。
3. **应用插件只写 safe Rust**：裸指针、C 字符串、FFI callback、导出符号与 `unsafe` 全部收口在框架及更低层。
4. **正确性事务由框架负责**：注册成功后的逆序回滚、shutdown 注销、panic containment、stale callback 隔离和 context 保活属于生命周期机制，不要求每个产品插件重复实现。
5. **跨平台产物可验证**：Windows DLL、Linux shared object 与 macOS dynamic library 都在构建后检查架构与 SCS 必需导出，而不是只看文件扩展名。

## SDK 1.14 覆盖范围

| 目录 | 数量 | 高层表示 |
| --- | ---: | --- |
| Common channels | 4 | `channels::common::*` 与 `channels::common::ALL` |
| Truck channels | 84 | `channels::truck::*` 与 `channels::truck::ALL` |
| Trailer channels | 18 | `channels::trailer::*` 与 `channels::trailer::ALL` |
| Job channels | 1 | `channels::job::*` 与 `channels::job::ALL` |
| **Channels 合计** | **107** | `channels::ALL` |
| Configuration IDs | 6 | `configuration::ids::*` 与 `ids::ALL` |
| Configuration attributes | 60 | `configuration::attributes::*` 与 `attributes::ALL` |
| Configuration associations | 71 | `configuration::associations::*` 与 `associations::ALL` |
| H-shifter values | 4 | `ShifterType::ALL` |
| Job market values | 5 | `JobMarket::ALL` |
| Gameplay events | 6 | `gameplay::events::*` 与 `events::ALL` |
| Gameplay attributes | 15 | `gameplay::attributes::*` 与 `attributes::ALL` |
| Gameplay associations | 21 | `gameplay::associations::*` 与 `associations::ALL` |
| Fine offence values | 14 | `FineOffence::ALL` |

除此之外，raw ABI 还覆盖：

- Telemetry API 版本、初始化参数和函数表；
- event/channel callback ABI；
- SDK result code、delivery flags 和 `SCS_U32_NIL`；
- 所有公开 tagged-union value 类型；
- `fvector`、`dvector`、Euler、单/双精度 placement；
- frame-start、configuration 与 gameplay callback payload；
- ETS2 的 telemetry game version 1.00–1.18；
- ATS 的 telemetry game version 1.00–1.05；
- ETS2、ATS 游戏 ID 和游戏版本拆分函数。

`ALL` catalog 不是另一套手抄字符串。`scs-sdk-sys` 暴露 header 顺序下的 raw 名称数组，`scs-sdk` 暴露保留 value type 与 indexed/scalar 元数据的 type-erased catalog；测试会逐项比对名称、数量、分组顺序和重复项。具体解码仍使用 `Channel<T>` 与 `Attribute<T>`，不会因为 catalog 可枚举而丢失类型信息。

## 四层结构

```text
examples/telemetry-plugin
        |
        | safe TelemetryPlugin API
        v
scs-sdk-plugin          lifecycle/runtime/framework
        |
        | typed SDK operations
        v
scs-sdk                 safe typed wrapper
        |
        | raw ABI
        v
scs-sdk-sys             no_std x86-64 ABI definitions

scs-sdk-plugin-macros   generates the two exported SCS entry points
```

从概念边界看，层级为：

```text
scs-sdk-sys <- scs-sdk <- scs-sdk-plugin <- scs-sdk-plugin-macros / application
```

实际 Cargo 依赖中，`scs-sdk-plugin` re-export `scs-sdk-plugin-macros`。proc macro 展开后引用调用方已依赖的 `scs_sdk_plugin`，从而避免两个 crate 形成循环依赖。

### `scs-sdk-sys`

`crates/scs-sdk-sys/` 是手写的 x86-64 C ABI 层：

- `no_std`；
- 零第三方 Rust 依赖；
- 不运行 bindgen，不要求构建机安装 Clang；
- 对照官方 SDK 1.14 header 定义函数指针、结构体、union、常量和 raw catalog；
- 对官方仅用于 ABI 对齐、数值不保证初始化的字段使用 `MaybeUninit<u32>`；
- 使用编译期断言检查关键结构体大小、对齐和字段 offset；
- 只声明支持 SCS 游戏使用的 64 位 ABI，不对 32 位目标作保证。

这一层允许出现原始指针与外部 ABI，因为它的职责就是准确描述 C 接口；它不负责提供应用层安全抽象。

### `scs-sdk`

`crates/scs-sdk/` 是 `no_std` 的类型化 wrapper：

- `TelemetryApi`、`TelemetrySession` 和不可逃逸的 `SdkCall` scope；
- `ScopedLogger` 与封闭的 `LogLevel`；
- SDK result code 的完整映射；
- 相互独立的 `TelemetryApiVersion`、`GameSchemaVersion` 强类型，以及直接从 raw header 投影的 `game::ets2::*`、`game::ats::*` typed schema 历史常量；
- `Channel<T>`、`AnyChannel`、`ChannelFlags`；
- `Attribute<T>`、`AnyAttribute`、`ConfigurationId`、`GameplayEventId`；
- 相互独立的 `SdkIndex`、`TrailerIndex` 与 `TrailerConfigurationId` 域，避免
  SDK indexed value、编号式 trailer namespace 与 legacy 无编号 `trailer`
  configuration 被静默混用；
- `ConfigurationAttributeAssociation` 与 `GameplayAttributeAssociation`，
  完整保留每个共享 attribute 由哪个 configuration group 或 event 携带；
- 每个内置 channel、configuration、gameplay descriptor 及 descriptor association
  都保留 `GameSchemaAvailability` 元数据；ETS2 与 ATS 的最低 schema 分开记录，
  证据来自官方 SDK 1.0 到 1.14 的历史 header；
- `ShifterType`、`JobMarket`、`FineOffence` 封闭字符串 value catalog，提供
  `ALL`、`COUNT`、`as_str`、`FromStr` 与 value-level schema availability；
- 零分配 parse failure 类型 `UnknownStringValue`；generic string API 同时保留
  future unknown value 的原文，供向前兼容诊断；
- `ValueRef` 对 tagged union 先验证 tag，再读取对应活跃成员；
- `ValueType` capability 元数据，包括 signed 64-bit value 需要 Telemetry API 1.01 的官方版本下限；
- Rust-owned 几何值 `FVector`、`DVector`、`Euler`、`FPlacement`、`DPlacement`；
- `NamedValues` 哨兵数组迭代与 typed attribute lookup；
- 107 个 typed channel、60 个 configuration attribute、71 条 configuration
  association、15 个 gameplay attribute 与 21 条 gameplay association 的可枚举
  catalog。

`DPlacement` 的高层值不携带 ABI padding。它可被复制和长期保存，而 wrapper 在解码时不会读取 SDK 未初始化的对齐字节。

SDK 规定调回游戏的 API 只能在主线程，并且只能发生在游戏直接调用插件的 init、event callback 或 shutdown 作用域中。`SdkCall` 使用 higher-ranked lifetime 限制，safe 代码不能把它返回、保存到全局状态或发送到其他线程。裸 callback/context 注册函数仍位于本层的受审计 `unsafe` 边界，供上层 runtime 使用。

### `scs-sdk-plugin`

`crates/scs-sdk-plugin/` 把底层能力组合成应用可用的 safe framework：

- `TelemetryPlugin` 生命周期；
- 通过必需的 `PluginMetadata` 显式声明产品身份；
- 通过必需的 `PluginCompatibility` 显式声明产品兼容要求；
- `PluginContext` 显式 event/channel subscription；
- owned `GameInfo`、`Game::{EuroTruckSimulator2, AmericanTruckSimulator, Other}`
  类型化游戏判断，以及 descriptor、association、value capability 共用的
  canonical `minimum_schema_for` / `supports` 查询；
- `ChannelUpdate` 的 descriptor、SDK index、trailer index 与 typed value 解码；
- `TelemetryEvent`、`ConfigurationEvent`、`GameplayEvent`，包括 typed trailer
  configuration identity，以及高层 `shifter_type`、`job_market`、`fine_offence`
  value decoder；
- Rust `str`/`String` 形式的游戏信息、configuration string 和 gameplay string；
- init/reinit/shutdown 状态机；
- 注册失败后的逆序事务回滚；
- required/optional descriptor 的 game-schema 预检，包括单独演进的编号式
  multi-trailer namespace；
- callback 与 shutdown 中的 panic containment；
- mutex poison 恢复；
- 旧 session callback 的 generation 隔离；
- 注销失败时的 foreign context 保活。

注册 context 使用 `Arc<Registration>` 持有稳定 pointee，并使用 `AtomicBool` 表示 SDK 侧是否仍注册。active/retired 集合只移动 `Arc` handle，不移动 allocation，也不通过重新创建独占借用破坏 foreign pointer provenance。每次 session 还有独立 generation，旧 session 即使延迟触发 callback，也不会进入新插件实例。该模型已经使用 Miri strict provenance 验证。

runtime 会在产品初始化前记录产品与兼容性身份，并在注册提交后输出实际 event/channel 数量。`game_display_name` 是 SCS 提供的完整展示字符串，API 与 schema 版本则保持为独立的强类型字段：

```text
[scs-sdk-plugin] starting plugin name="SCS SDK Telemetry Example" version="0.1.0" framework_version="0.1.0"
[scs-sdk-plugin] detected game_display_name="Euro Truck Simulator 2 1.60.1.7s" game_id="eut2" telemetry_api=1.1 telemetry_schema=1.19
[scs-sdk-plugin] initialized plugin name="SCS SDK Telemetry Example" version="0.1.0" events=6 channels=8
```

API 支持能力只有一个权威来源：`scs-sdk::TelemetryApi` 列出已经审计 foreign
初始化布局 adapter 的版本，framework 直接消费该结果，不再维护第二份版本白名单。
产品的 `PluginCompatibility` 是另一层独立声明：它描述产品实际需要的最低 API
能力以及每款支持游戏的最低 schema。runtime 在同一 major 内接受更高 schema minor，
遇到不同 major 时等待显式审计，并在产品初始化和 SDK 注册之前完成全部验证。

下载压缩包末尾的版本、协商得到的 Telemetry API、每款游戏的 telemetry schema
是三个不同版本域。官方 SDK 1.0 到 1.14 的历史压缩包给出了以下对应关系：

| SDK 压缩包 | Telemetry API `CURRENT` | ETS2 schema `CURRENT` | ATS schema `CURRENT` |
| --- | --- | --- | --- |
| 1.0 | 1.00 | 1.05 | - |
| 1.1 | 1.00 | 1.07 | - |
| 1.2 | 1.00 | 1.08 | - |
| 1.3 | 1.00 | 1.09 | - |
| 1.4 | 1.00 | 1.10 | - |
| 1.5 | 1.00 | 1.12 | - |
| 1.6-1.8 | 1.00 | 1.12 | 1.00 |
| 1.9 | 1.00 | 1.13 | 1.00 |
| 1.10 | 1.01 | 1.14 | 1.01 |
| 1.11 | 1.01 | 1.15 | 1.02 |
| 1.12 | 1.01 | 1.16 | 1.03 |
| 1.13 | 1.01 | 1.17 | 1.04 |
| 1.14 | 1.01 | 1.18 | 1.05 |

其中，官方 SDK 1.10 到 1.14 的 `scssdk_telemetry.h` 逐字节相同，但游戏专属
header 仍在继续增加 descriptor。wrapper 已把这些新增历史作为 per-game schema
availability 记录到 `Channel`、`Attribute`、`ConfigurationId`、
`GameplayEventId` 与 `Event`。另外，71 条 configuration attribute 关系与 21 条
gameplay attribute 关系单独记录，因为一个 attribute 可能在自身 descriptor 出现很久后
才加入第二个 group；`FineOffence` 在 value level 也保留同样的历史差异。编号式 trailer
namespace 与 gameplay payload 的共同版本边界统一由 `game::capabilities` 提供，不在各个
catalog 重复写 schema 数字。loading schema 太旧时，required registration 会在本地直接
失败，optional registration 则在 SDK 收到不存在的名称之前本地跳过；channel-specific
value conversion 和运行时缺失仍由 SCS 最终判定。因此插件不会要求用户选择某个 SDK
压缩包。

### `scs-sdk-plugin-macros`

`crates/scs-sdk-plugin-macros/` 提供：

```rust
scs_sdk_plugin::export_plugin!(TelemetryExample::default());
```

宏生成 SCS loader 查找的：

```text
scs_telemetry_init
scs_telemetry_shutdown
```

同时生成进程内稳定 runtime storage、ABI 参数转换与 unwind boundary。应用 crate 不手写 `extern "system"`、`no_mangle`、raw pointer 或全局 runtime。

宏不是只靠一个 `ignore` rustdoc 示例撑场面。`crates/scs-sdk-plugin/tests/fixtures/export-plugin/` 是独立于主 workspace 的 consumer workspace，只有公开的 `scs-sdk-plugin` path dependency，并包含两个 package：

- `pass`：实现 `TelemetryPlugin`，以 `export_plugin!(Plugin::default())` 构建真实 `cdylib`；
- `missing-trait`：保留相同构造表达式但省略 trait 实现，必须以 E0277 和 `TelemetryPlugin` trait-bound 失败。

正向 fixture 使用 `#![forbid(unsafe_code)]`，同时接受和应用插件一致的源码边界审计。Windows PE、Linux ELF 与 macOS Mach-O 构建还会在 LTO 和 symbol stripping 后检查 `scs_telemetry_init`、`scs_telemetry_shutdown` 的外部导出，避免把“宏语法展开成功”误当成“游戏 loader 确实可见符号”。proc-macro rustdoc 示例继续标记为 ignored，是因为 macro crate 反向 dev-depend framework 会形成 Cargo 依赖环；独立 fixture 正是这个依赖边界的长期测试入口。

## 显式订阅

插件必须在 `initialize` 中逐项声明意图：

```rust
use scs_sdk_plugin::sdk::{
    ChannelFlags, SdkIndex, TelemetryApiVersion, TrailerIndex, channels, game,
};
use scs_sdk_plugin::{
    Game, GameCompatibility, PluginCompatibility, PluginContext, PluginMetadata,
    PluginResult, TelemetryEventKind, TelemetryPlugin,
};

struct Plugin;

static SUPPORTED_GAMES: [GameCompatibility; 1] = [GameCompatibility::new(
    Game::EuroTruckSimulator2,
    game::ets2::V1_00,
)];

impl TelemetryPlugin for Plugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("My Telemetry Plugin", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> PluginCompatibility {
        PluginCompatibility::new(TelemetryApiVersion::V1_00, &SUPPORTED_GAMES)
    }

    fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
        context.subscribe_event(TelemetryEventKind::Started)?;
        context.subscribe_event(TelemetryEventKind::FrameEnd)?;
        context.subscribe_event_optional(TelemetryEventKind::Gameplay)?;

        context.subscribe(channels::truck::SPEED)?;
        context.subscribe_optional(channels::truck::NAVIGATION_SPEED_LIMIT)?;
        context.subscribe_with_flags(
            channels::truck::ENGINE_RPM,
            ChannelFlags::EACH_FRAME,
        )?;
        context.subscribe_at(channels::truck::WHEEL_ROTATION, SdkIndex::ZERO)?;
        context.subscribe_trailer(
            channels::trailer::CONNECTED,
            TrailerIndex::ALL[1],
        )?;

        Ok(())
    }
}
```

几个索引概念不会被混在一起：

- `subscribe(channel)`：scalar channel；
- `subscribe_at(channel, sdk_index)`：wheel、selector 等 SDK 数组下标；
- `subscribe_trailer(channel, trailer_index)`：`trailer.0.*` 至 `trailer.9.*` 名称中的挂车编号；
- `subscribe_trailer_at(channel, trailer_index, sdk_index)`：指定挂车的 indexed channel；
- 每组都有 `_with_flags` 版本，用于显式选择 `EACH_FRAME`、`NO_VALUE` 等 delivery flags；
- 每个 channel index domain 都有对应的显式 `subscribe*_optional` 方法族；optional 声明只容忍 `NotFound` 和 `UnsupportedType`，跳过时保留产品侧默认值，并且不会放宽 descriptor 形状、重复声明、生命周期或其他 SDK 错误；
- `subscribe_event_optional(event)` 会跳过晚于当前协商 API 才出现的 event，并且在 event 注册阶段只容忍 `Unsupported` 或 `NotFound`；
- `Channel::requesting<U>()` 显式选择 SDK 转换后的 value representation。required representation 若晚于当前协商 API，framework 会在注册前拒绝（`i64` 需要 Telemetry API 1.01）；具体 channel conversion 仍由 SCS 最终判定。optional 的新版 representation 会被跳过。

`SdkIndex::new` 只排除表示 scalar 的 `SCS_U32_NIL` sentinel。
`TrailerIndex::new` 验证 SDK 1.14 官方规定的 `0..10` 范围，
`TrailerIndex::ALL` 则提供十个静态有效值。Configuration callback 通过
`TrailerConfigurationId` 保留 legacy `trailer` 与编号式 `trailer.0` 的差异，
不会把 legacy identity 静默重写成 `TrailerIndex::ZERO`。

SDK 中形似 enum 的字符串使用“typed known + raw unknown”成对 API。
`ConfigurationEvent::shifter_type`、`ConfigurationEvent::job_market` 与
`GameplayEvent::fine_offence` 会在值属于 SDK 1.14 已知集合时返回 enum；generic
`string` / `string_owned` accessor 仍同时可用，因此未来游戏新增的原始值不会丢失。
这些 catalog enum 也实现了 `FromStr`；`GameInfo::supports` 则根据检测到的游戏类型
与 schema，统一判断任意 `GameSchemaAvailability`。

`TelemetryPlugin::initialize` 没有默认实现。空插件若没有显式订阅，runtime 对 SDK 发起的 event/channel 注册次数就是零。重复订阅会在调用 SDK 之前返回 `AlreadyRegistered`；在 callback 或 shutdown 阶段订阅会返回 `NotNow`。

显式意图不等于把资源管理推给产品。插件成功返回后，runtime 才提交注册；预期的 capability 缺失只跳过对应 optional 声明，required 失败以及 optional 的非 capability 错误都会让已完成前缀按相反顺序回滚。正常 shutdown 使用相同的逆序规则，committed count 只统计实际注册成功的项目。

## 示例插件边界

`examples/telemetry-plugin/` 是真实实机探针，也是 framework 的应用边界示例。它只依赖 `scs-sdk-plugin`，显式订阅 6 类 event 和 8 个 channel，然后使用 typed callback 更新 snapshot、记录任务配置和 gameplay event。

该目录的源码目标是：

- 零 `unsafe`；
- 零裸指针；
- 零手写外部 ABI；
- 零 C 字符串类型或字面量；
- 零 `scs-sdk-sys` / `::sys` 访问；
- 零 `scs_sdk_plugin::__private` 宏卫生实现访问。

可使用仓库内与 CI 共用的检查确认边界没有回退：

```bash
scripts/check-plugin-boundary.sh
```

该脚本同时审计 Rust 源码和 `Cargo.toml`，防止应用直接依赖 `scs-sdk-sys`，也防止手写代码借宏专用的 doc-hidden `__private` 模块绕回 raw ABI。wrapper/runtime 中仍会保留必要且有 Safety contract 的 `unsafe`；要求它们表面上也完全消失，只会把 FFI 前提藏起来，而不是让边界更可靠。

## Loader fallback E2E 示例

`examples/telemetry-fallback-plugin/` 是独立的真实 ETS 手动探针，专门验证
SCS 文档规定的 loader 规则：游戏从最新到最旧尝试 Telemetry API，且仅当
`scs_telemetry_init` 返回 `SCS_RESULT_unsupported` 时继续重试旧版本。

该探针把 API 1.00 声明为 compatibility minimum，使 SDK 1.14 的两次尝试都能
进入产品初始化；随后故意以 `SdkError::Unsupported` 拒绝 1.01，只接受精确的
1.00，并且只注册两个 API 1.00 event 与 scalar truck-speed channel。这是测试行为，
不是普通产品插件的兼容策略。

`game.log.txt` 应按顺序出现：

1. API 1.01 的 `[scs-sdk-fallback-example] requesting loader retry`，并带有
   `result=unsupported`；
2. API 1.01 的 `[scs-sdk-fallback-example] rejected attempt cleaned`；
3. API 1.00 的 `[scs-sdk-fallback-example] accepted loader fallback`；
4. framework 初始化结果 `events=2 channels=1`；
5. API 1.00 下的 `[scs-sdk-fallback-example] fallback callbacks confirmed`，且
   同时带有 `frame_end_seen=true` 与已经解码的 `speed_metres_per_second`，分别
   证明 event 和 channel delivery；
6. clean fallback-session shutdown。

普通示例与 fallback 示例一次只安装一个。两个 macOS installer 都会先验证新
产物，再移除另一个示例和 legacy 示例的精确文件名，使日志中只保留一条 negotiation
序列。

## Workspace

```text
crates/scs-sdk-sys/             SDK 1.14 raw x86-64 ABI
crates/scs-sdk/                 no_std typed wrapper 与完整 catalog
crates/scs-sdk-plugin/          safe plugin 生命周期框架
crates/scs-sdk-plugin-macros/   SCS entry-point proc macro
  tests/fixtures/export-plugin/ 独立宏 compile-pass/fail cdylib workspace
examples/telemetry-plugin/      safe Rust 实机示例 cdylib
examples/telemetry-fallback-plugin/
                                真实 ETS API fallback 手动 E2E cdylib
scripts/                        Windows/Linux/macOS 构建与产物验证
third-party/scs_sdk_1_14/       官方 SDK 原始分发与许可证
tmp/                            本地调查、日志结论和设计笔记
```

具体产品应用、bridge、网页、调度逻辑、存档集成与其他最终用户组件均刻意放在该 SDK workspace 之外。

## 开发环境

仓库通过 `rust-toolchain.toml` 固定 Rust `1.85.0`，声明 `rustfmt`、`clippy`、Windows GNU、Linux GNU 与 macOS x86-64 targets。

基础要求：

- rustup；
- Cargo；
- Bash；
- `file`；
- nightly Miri，用于 provenance 与生命周期验证。

安装 Miri：

```bash
rustup toolchain install nightly-2026-04-12 \
  --profile minimal \
  --component miri \
  --component rust-src
```

`rust-src` 用于构建 Miri sysroot；固定日期与 CI 一致，同时由 rustup 自动选择本机 host triple。

Windows x64 交叉编译需要 MinGW-w64：

```text
x86_64-w64-mingw32-gcc
x86_64-w64-mingw32-objdump
```

Linux x86-64 交叉编译使用 Zig 与 `cargo-zigbuild`：

```bash
cargo install cargo-zigbuild --version 0.23.0 --locked
```

Linux 产物验证还需要一个能读取 ELF dynamic symbol table 的 `nm`。脚本按顺序查找：

1. `$NM` 显式路径；
2. `x86_64-linux-gnu-nm`；
3. `llvm-nm`；
4. Linux 主机上的原生 `nm`。

例如 Homebrew 环境可安装：

```bash
brew install mingw-w64 zig
```

## 质量门禁

完整本地检查：

```bash
cargo fmt --all -- --check
scripts/check-license-copies.sh
scripts/check-plugin-boundary.sh
scripts/check-plugin-macro-fixtures.sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --locked
cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
MIRIFLAGS=-Zmiri-strict-provenance \
  cargo +nightly-2026-04-12 miri test --locked -p scs-sdk-plugin
```

测试覆盖包括：

- 每个可发布 crate 内与仓库根目录逐字节一致的 Apache-2.0 与 MIT 许可证副本；
- 所有 SDK result code 与 channel flag；
- 107/60/15 catalog 的逐项 raw 名称比对、顺序、索引模式和重复项；
- 每种 primitive/geometry tagged-union 解码；
- 错误或未知 tag 不读取 inactive union member；
- 未初始化 ABI padding 不被读取；
- `SdkCall` 不可逃逸且不实现 `Send`/`Sync` 的 compile-fail doctest；
- proc-macro 正向 consumer 独立编译、严格 Clippy 和 safe-source 审计；
- 缺少 `TelemetryPlugin` 实现时精确产生 E0277 trait-bound failure；
- Windows PE、Linux ELF 与 macOS Mach-O fixture 均保留两个 loader-visible SCS exports；
- 全 workspace rustdoc 在 `-Dwarnings` 下没有失效的 intra-doc links；
- owned game metadata 与 Rust string boundary；
- scalar、indexed 和 multi-trailer 订阅命名；
- 显式 event subscription 与空插件零注册；
- duplicate/invalid-phase subscription 拦截；
- channel/event dispatch；
- partial-init 逆序回滚与 shutdown 逆序注销；
- stale generation callback 拦截；
- stable context provenance 与无泄漏销毁。
- fallback probe 精确策略：拒绝 API 1.01、接受 API 1.00，并保持 accepted
  subscription surface 与 API 1.00 兼容。

workspace 使用严格 Clippy 配置，尤其拒绝可能截断或丢失符号位的 cast；非测试构建同时拒绝 `unwrap`、`expect`、`panic`、`todo`、`unimplemented` 和 `unreachable`。

## 持续集成

`.github/workflows/rust.yml` 保留只读仓库权限、相同分支新提交的并发取消、路径过滤、固定超时和独立 Rust cache。workspace 根据自身基座边界拆成七个并行 gate：

| Job | 验证内容 |
| --- | --- |
| `Format, Clippy, and boundaries` | rustfmt、shell 语法、应用 safe boundary、宏 compile-pass/fail fixture、全 workspace Clippy 与严格 rustdoc |
| `Workspace tests` | 全 workspace unit tests 和 doctests |
| `Miri (scs-sdk)` | typed value、union、padding、scope 与 catalog 的 Miri 验证 |
| `Miri (scs-sdk-plugin)` | runtime strict provenance、context 生命周期和 stale generation 验证 |
| `Windows x86-64 plugin` | 示例与独立宏 fixture 的 MinGW release DLL、PE32+/x86-64 和两个 SCS dynamic exports |
| `Linux x86-64 plugin (glibc 2.17)` | 示例与独立宏 fixture 的 Zig release shared object、ELF/x86-64 和两个 SCS dynamic exports |
| `macOS x86-64 plugin` | 普通示例、fallback E2E probe 与独立宏 fixture 的 release dynamic library；Mach-O/x86-64、签名和精确 SCS export 集合 |

CI 固定使用：

```text
Rust/MSRV:          1.85.0
Miri:               nightly-2026-04-12
Zig:                0.16.0
cargo-zigbuild:     0.23.0
Linux glibc floor:  2.17
```

Windows、Linux 与 macOS job 会上传已经通过格式及导出检查的插件产物，保留 7 天；macOS job 还会用独立名称上传手动 fallback E2E artifact。workflow 支持 `master` push、面向 `master` 的 pull request 和手动触发；只有 SDK 基座、example、构建脚本、工具链或 workflow 自身变化时才运行，README 与后续网页目录里的独立改动不会触发整套 Miri 和跨平台构建。

## 构建与验证

### Windows x64

```bash
scripts/build-windows-plugin.sh
```

产物：

```text
target/x86_64-pc-windows-gnu/release/scs_sdk_telemetry_example.dll
```

脚本会检查：

- PE32+ DLL；
- x86-64 architecture；
- PE export table 恰好只包含 `scs_telemetry_init` 与
  `scs_telemetry_shutdown`，不存在额外 named 或 ordinal-only export。

也可单独验证已有产物：

```bash
scripts/verify-windows-plugin.sh PATH_TO_DLL
```

### Linux x86-64

```bash
scripts/build-linux-plugin.sh
```

构建使用 glibc 2.17 baseline：

```text
x86_64-unknown-linux-gnu.2.17
```

产物：

```text
target/x86_64-unknown-linux-gnu/release/libscs_sdk_telemetry_example.so
```

脚本会检查：

- ELF 64-bit LSB shared object；
- x86-64 architecture；
- defined dynamic export 集合恰好为 `scs_telemetry_init` 与
  `scs_telemetry_shutdown`。

也可单独验证已有产物：

```bash
scripts/verify-linux-plugin.sh PATH_TO_SHARED_OBJECT
```

### macOS x86-64

```bash
scripts/build-macos-plugin.sh
```

当前 macOS 版 ETS2 可执行文件仍为 x86-64；在 Apple Silicon 上也通过 Rosetta 运行。因此脚本显式选择 `x86_64-apple-darwin`，不会跟随构建机生成 arm64 插件。

产物：

```text
target/x86_64-apple-darwin/release/libscs_sdk_telemetry_example.dylib
```

脚本会检查：

- Mach-O 64-bit dynamically linked shared library；
- x86-64 architecture；
- 有效的 embedded code signature；本地与 CI 构建使用 ad-hoc identity；
- defined external symbol 集合恰好为 `_scs_telemetry_init` 与
  `_scs_telemetry_shutdown`；前导下划线来自 Mach-O C ABI 拼写。

ad-hoc signature 能为本地构建提供可验证的 code directory，但它不等于 Developer ID signing 或 notarization。公开 release pipeline 后续应替换为项目自己的 Developer ID 签名与 notarized 分发包。

也可单独验证已有产物：

```bash
scripts/verify-macos-plugin.sh PATH_TO_DYNAMIC_LIBRARY
```

### macOS loader fallback E2E

与普通示例分开构建这个故意拒绝新版 API 的探针：

```bash
scripts/build-macos-fallback-plugin.sh
```

产物：

```text
target/x86_64-apple-darwin/release/libscs_sdk_telemetry_fallback_example.dylib
```

构建脚本使用和普通插件相同的 x86-64、code signing 与精确 export 验证。将它作为
唯一示例探针安装：

```bash
scripts/install-macos-fallback-plugin.sh
```

切回普通 6-event/8-channel 示例：

```bash
scripts/install-macos-plugin.sh
```

### Proc-macro 独立 fixture

只检查正向/负向编译契约、格式、Clippy 和 safe source：

```bash
scripts/check-plugin-macro-fixtures.sh
```

构建并验证 Windows fixture 的真实 PE exports：

```bash
scripts/build-windows-plugin-macro-fixture.sh
```

构建并验证 Linux glibc 2.17 fixture 的真实 ELF exports：

```bash
scripts/build-linux-plugin-macro-fixture.sh
```

构建 macOS x86-64 fixture 并验证真实 Mach-O exports：

```bash
scripts/build-macos-plugin-macro-fixture.sh
```

fixture 产物位于 `target/plugin-macro-fixtures/`，仅用于验证 proc-macro 的 consumer contract，不作为示例插件分发。游戏安装使用本节前面列出的对应平台示例产物。

## 安装到 ETS2

Windows DLL 放入：

```text
bin/win_x64/plugins/scs_sdk_telemetry_example.dll
```

Linux shared object 放入：

```text
bin/linux_x64/plugins/libscs_sdk_telemetry_example.so
```

macOS dynamic library 放入：

```text
<ETS2 安装目录>/Euro Truck Simulator 2.app/Contents/MacOS/plugins/libscs_sdk_telemetry_example.dylib
```

Steam library 位于当前用户默认位置时，`<ETS2 安装目录>` 通常是 `~/Library/Application Support/Steam/steamapps/common/Euro Truck Simulator 2`。SCS 从游戏可执行文件旁的 `plugins` 目录发现插件；这里与保存 profile 和日志的用户数据目录不是同一个位置。

仓库安装脚本会清除下载产物的 quarantine 属性，对私有副本应用 ad-hoc 签名，完成验证后再写入该目录：

```bash
scripts/install-macos-plugin.sh
```

macOS App Management 会控制对其他 application bundle 的写入。如果安装时报 `Operation not permitted`，需要在 **系统设置 -> 隐私与安全性 -> App Management** 中允许当前终端，重启终端后重新执行。安装脚本不会重签 ETS2 application；那样会替换 SCS Software 的 Developer ID 与已公证应用签名。

不要同时放置多个实现相同探针的 telemetry plugin，否则它们会分别注册 channel 并产生重复日志。首次加载第三方 SDK plugin 时，ETS2 可能显示确认提示。

Windows 游戏日志通常位于：

```text
Documents/Euro Truck Simulator 2/game.log.txt
```

macOS 游戏日志通常位于：

```text
~/Library/Application Support/Euro Truck Simulator 2/game.log.txt
```

当前探针日志前缀：

```text
[scs-sdk-example]
```


## 许可证

本项目自行编写的 Rust 代码由用户任选以下许可证之一：

- [Apache License, Version 2.0](LICENSE-APACHE)；
- [MIT License](LICENSE-MIT)。

`third-party/scs_sdk_1_14/` 来自 SCS Software，受其随 SDK 提供的独立授权文本约束。该文本允许使用、修改和分发，但要求在软件副本或实质性部分中保留 SCS Software 的版权与许可声明。
