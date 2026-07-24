# ETS2 Dispatch

[English](README.md) | **简体中文**

ETS2 Dispatch 的长期目标是为 Euro Truck Simulator 2 提供外置网页仪表、导航数据和调度能力。仓库当前只推进 **SCS Telemetry SDK 基座**：先把官方 SDK 1.14 的 ABI、类型化 wrapper、插件生命周期和跨平台构建做完整，再讨论 bridge、网页或调度产品。

当前基座使用纯 Rust 实现，不需要 C++ shim、CMake 或 bindgen。官方 SDK 原始分发仍保留在 `third-party/scs_sdk_1_14/`，它是 ABI 与常量的权威来源。

## 基座目标

当前设计坚持以下边界：

1. **完整覆盖 SDK 1.14**：公开 telemetry headers 中的 ABI、channel、configuration、gameplay event、游戏标识和版本常量均进入 Rust 层。
2. **产品意图必须显式**：框架不会因为插件实现了某个回调，就猜测它想订阅对应事件，也不会自动订阅整个 channel catalog。
3. **应用插件只写 safe Rust**：裸指针、C 字符串、FFI callback、导出符号与 `unsafe` 全部收口在框架及更低层。
4. **正确性事务由框架负责**：注册成功后的逆序回滚、shutdown 注销、panic containment、stale callback 隔离和 context 保活属于生命周期机制，不要求每个产品插件重复实现。
5. **跨平台产物可验证**：Windows DLL 和 Linux shared object 都在构建后检查架构与 SCS 必需导出，而不是只看文件扩展名。

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
| H-shifter values | 4 | `ShifterType` |
| Gameplay events | 6 | `gameplay::events::*` 与 `events::ALL` |
| Gameplay attributes | 15 | `gameplay::attributes::*` 与 `attributes::ALL` |
| Fine offence values | 14 | `FineOffence` |

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
apps/plugin-rust
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
- `Channel<T>`、`AnyChannel`、`ChannelFlags`；
- `Attribute<T>`、`AnyAttribute`、`ConfigurationId`、`GameplayEventId`；
- `ValueRef` 对 tagged union 先验证 tag，再读取对应活跃成员；
- Rust-owned 几何值 `FVector`、`DVector`、`Euler`、`FPlacement`、`DPlacement`；
- `NamedValues` 哨兵数组迭代与 typed attribute lookup；
- 107 个 typed channel、60 个 configuration attribute 和 15 个 gameplay attribute 的可枚举 catalog。

`DPlacement` 的高层值不携带 ABI padding。它可被复制和长期保存，而 wrapper 在解码时不会读取 SDK 未初始化的对齐字节。

SDK 规定调回游戏的 API 只能在主线程，并且只能发生在游戏直接调用插件的 init、event callback 或 shutdown 作用域中。`SdkCall` 使用 higher-ranked lifetime 限制，safe 代码不能把它返回、保存到全局状态或发送到其他线程。裸 callback/context 注册函数仍位于本层的受审计 `unsafe` 边界，供上层 runtime 使用。

### `scs-sdk-plugin`

`crates/scs-sdk-plugin/` 把底层能力组合成应用可用的 safe framework：

- `TelemetryPlugin` 生命周期；
- `PluginContext` 显式 event/channel subscription；
- owned `GameInfo` 与 `Game::{EuroTruckSimulator2, AmericanTruckSimulator, Other}` 类型化游戏判断；
- `ChannelUpdate` 的 descriptor、SDK index、trailer index 与 typed value 解码；
- `TelemetryEvent`、`ConfigurationEvent`、`GameplayEvent`；
- Rust `str`/`String` 形式的游戏信息、configuration string 和 gameplay string；
- init/reinit/shutdown 状态机；
- 注册失败后的逆序事务回滚；
- callback 与 shutdown 中的 panic containment；
- mutex poison 恢复；
- 旧 session callback 的 generation 隔离；
- 注销失败时的 foreign context 保活。

注册 context 使用 `Arc<Registration>` 持有稳定 pointee，并使用 `AtomicBool` 表示 SDK 侧是否仍注册。active/retired 集合只移动 `Arc` handle，不移动 allocation，也不通过重新创建独占借用破坏 foreign pointer provenance。每次 session 还有独立 generation，旧 session 即使延迟触发 callback，也不会进入新插件实例。该模型已经使用 Miri strict provenance 验证。

### `scs-sdk-plugin-macros`

`crates/scs-sdk-plugin-macros/` 提供：

```rust
scs_sdk_plugin::export_plugin!(DispatchPlugin::default());
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

正向 fixture 使用 `#![forbid(unsafe_code)]`，同时接受和应用插件一致的源码边界审计。Windows PE 与 Linux ELF 构建还会在 LTO 和 symbol stripping 后检查 `scs_telemetry_init`、`scs_telemetry_shutdown` 的动态导出，避免把“宏语法展开成功”误当成“游戏 loader 确实可见符号”。proc-macro rustdoc 示例继续标记为 ignored，是因为 macro crate 反向 dev-depend framework 会形成 Cargo 依赖环；独立 fixture 正是这个依赖边界的长期测试入口。

## 显式订阅

插件必须在 `initialize` 中逐项声明意图：

```rust
use scs_sdk_plugin::sdk::{ChannelFlags, channels};
use scs_sdk_plugin::{
    PluginContext, PluginResult, TelemetryEventKind, TelemetryPlugin,
};

struct Plugin;

impl TelemetryPlugin for Plugin {
    fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
        context.subscribe_event(TelemetryEventKind::Started)?;
        context.subscribe_event(TelemetryEventKind::FrameEnd)?;

        context.subscribe(channels::truck::SPEED)?;
        context.subscribe_with_flags(
            channels::truck::ENGINE_RPM,
            ChannelFlags::EACH_FRAME,
        )?;
        context.subscribe_at(channels::truck::WHEEL_ROTATION, 0)?;
        context.subscribe_trailer(channels::trailer::CONNECTED, 1)?;

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
- `Channel::requesting<U>()` 显式选择 SDK 转换后的 value representation，兼容性最终由游戏注册函数裁定。

`TelemetryPlugin::initialize` 没有默认实现。空插件若没有显式订阅，runtime 对 SDK 发起的 event/channel 注册次数就是零。重复订阅会在调用 SDK 之前返回 `AlreadyRegistered`；在 callback 或 shutdown 阶段订阅会返回 `NotNow`。

显式意图不等于把资源管理推给产品。插件成功返回后，runtime 才提交注册；任一 SDK 调用失败时，之前已注册项目会按相反顺序注销。正常 shutdown 使用相同的逆序规则。

## 应用插件边界

`apps/plugin-rust/` 是当前实机探针，也是 framework 的边界样例。它只依赖 `scs-sdk-plugin`，显式订阅当前需要的 6 类 event 和 8 个 channel，然后使用 typed callback 更新 snapshot、记录任务配置和 gameplay event。

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

## Workspace

```text
apps/plugin-rust/               safe Rust 实机探针 cdylib
crates/scs-sdk-sys/             SDK 1.14 raw x86-64 ABI
crates/scs-sdk/                 no_std typed wrapper 与完整 catalog
crates/scs-sdk-plugin/          safe plugin 生命周期框架
crates/scs-sdk-plugin-macros/   SCS entry-point proc macro
  tests/fixtures/export-plugin/ 独立宏 compile-pass/fail cdylib workspace
scripts/                        Windows/Linux 构建与产物验证
third-party/scs_sdk_1_14/       官方 SDK 原始分发与许可证
tmp/                            本地调查、日志结论和设计笔记
```

`apps/bridge/`、`apps/web/`、`crates/dispatcher/`、`crates/protocol/`、`crates/savegame/`、`crates/telemetry/`、`assets/` 和 `fixtures/` 是后续产品阶段的保留目录，当前不属于 SDK 基座 workspace。

## 开发环境

仓库通过 `rust-toolchain.toml` 固定 Rust `1.85.0`，声明 `rustfmt`、`clippy`、Windows GNU 与 Linux GNU targets。

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

- 所有 SDK result code 与 channel flag；
- 107/60/15 catalog 的逐项 raw 名称比对、顺序、索引模式和重复项；
- 每种 primitive/geometry tagged-union 解码；
- 错误或未知 tag 不读取 inactive union member；
- 未初始化 ABI padding 不被读取；
- `SdkCall` 不可逃逸且不实现 `Send`/`Sync` 的 compile-fail doctest；
- proc-macro 正向 consumer 独立编译、严格 Clippy 和 safe-source 审计；
- 缺少 `TelemetryPlugin` 实现时精确产生 E0277 trait-bound failure；
- Windows PE 与 Linux ELF fixture 均保留两个 loader-visible SCS exports；
- 全 workspace rustdoc 在 `-Dwarnings` 下没有失效的 intra-doc links；
- owned game metadata 与 Rust string boundary；
- scalar、indexed 和 multi-trailer 订阅命名；
- 显式 event subscription 与空插件零注册；
- duplicate/invalid-phase subscription 拦截；
- channel/event dispatch；
- partial-init 逆序回滚与 shutdown 逆序注销；
- stale generation callback 拦截；
- stable context provenance 与无泄漏销毁。

workspace 使用严格 Clippy 配置，尤其拒绝可能截断或丢失符号位的 cast；非测试构建同时拒绝 `unwrap`、`expect`、`panic`、`todo`、`unimplemented` 和 `unreachable`。

## 持续集成

`.github/workflows/rust.yml` 参考 AsterDrive 的 Rust workflow 组织方式，保留只读仓库权限、相同分支新提交的并发取消、路径过滤、固定超时和独立 Rust cache。ETS2 Dispatch 根据自身基座边界拆成六个并行 gate：

| Job | 验证内容 |
| --- | --- |
| `Format, Clippy, and boundaries` | rustfmt、shell 语法、应用 safe boundary、宏 compile-pass/fail fixture、全 workspace Clippy 与严格 rustdoc |
| `Workspace tests` | 全 workspace unit tests 和 doctests |
| `Miri (scs-sdk)` | typed value、union、padding、scope 与 catalog 的 Miri 验证 |
| `Miri (scs-sdk-plugin)` | runtime strict provenance、context 生命周期和 stale generation 验证 |
| `Windows x86-64 plugin` | 产品与独立宏 fixture 的 MinGW release DLL、PE32+/x86-64 和两个 SCS dynamic exports |
| `Linux x86-64 plugin (glibc 2.17)` | 产品与独立宏 fixture 的 Zig release shared object、ELF/x86-64 和两个 SCS dynamic exports |

CI 固定使用：

```text
Rust/MSRV:          1.85.0
Miri:               nightly-2026-04-12
Zig:                0.16.0
cargo-zigbuild:     0.23.0
Linux glibc floor:  2.17
```

Windows 和 Linux job 会上传已经通过格式及导出检查的插件产物，保留 7 天。workflow 支持 `master` push、面向 `master` 的 pull request 和手动触发；只有 SDK 基座、构建脚本、工具链或 workflow 自身变化时才运行，README 与后续网页目录里的独立改动不会触发整套 Miri 和跨平台构建。

## 构建与验证

### Windows x64

```bash
scripts/build-windows-plugin.sh
```

产物：

```text
target/x86_64-pc-windows-gnu/release/ets2_dispatch_telemetry_rust.dll
```

脚本会检查：

- PE32+ DLL；
- x86-64 architecture；
- `scs_telemetry_init` dynamic export；
- `scs_telemetry_shutdown` dynamic export。

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
target/x86_64-unknown-linux-gnu/release/libets2_dispatch_telemetry_rust.so
```

脚本会检查：

- ELF 64-bit LSB shared object；
- x86-64 architecture；
- dynamic symbol table 中的 `scs_telemetry_init`；
- dynamic symbol table 中的 `scs_telemetry_shutdown`。

也可单独验证已有产物：

```bash
scripts/verify-linux-plugin.sh PATH_TO_SHARED_OBJECT
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

fixture 产物位于 `target/plugin-macro-fixtures/`，仅用于验证 proc-macro 的 consumer contract，不作为 ETS2 Dispatch 产品插件分发。游戏安装仍使用本节前面列出的 `ets2_dispatch_telemetry_rust.dll` 或 `libets2_dispatch_telemetry_rust.so`。

## 安装到 ETS2

Windows DLL 放入：

```text
bin/win_x64/plugins/ets2_dispatch_telemetry_rust.dll
```

Linux shared object 放入：

```text
bin/linux_x64/plugins/libets2_dispatch_telemetry_rust.so
```

不要同时放置多个实现相同探针的 telemetry plugin，否则它们会分别注册 channel 并产生重复日志。首次加载第三方 SDK plugin 时，ETS2 可能显示确认提示。

Windows 游戏日志通常位于：

```text
Documents/Euro Truck Simulator 2/game.log.txt
```

当前探针日志前缀：

```text
[ets2-dispatch-rust]
```

## 后续阶段

SDK 基座完成并稳定后，产品层再依次处理：

1. 本地 bridge 与 IPC；
2. telemetry protocol；
3. 浏览器/PWA 仪表盘；
4. 调度、行程记录与本地数据库；
5. 存档任务同步；
6. 独立路线与地图能力。

这些目录目前只是保留边界，不应反向把网络、数据库或产品状态塞回游戏进程内的 telemetry callback。

## 许可证

本项目自行编写的 Rust 代码由用户任选以下许可证之一：

- [Apache License, Version 2.0](LICENSE-APACHE)；
- [MIT License](LICENSE-MIT)。

`third-party/scs_sdk_1_14/` 来自 SCS Software，受其随 SDK 提供的独立授权文本约束。该文本允许使用、修改和分发，但要求在软件副本或实质性部分中保留 SCS Software 的版权与许可声明。
