<div align="center">

# SCS SDK Rust Crates

**用安全、强类型的 Rust 构建原生 SCS telemetry 与 input-device 插件。**

完整覆盖 SDK 1.14 的公开 Telemetry 与 Input 接口 · 审计过的 FFI 与生命周期边界 · 已验证三平台产物

[English](README.md) · **简体中文**

</div>

`scs-sdk-crates` 是面向 **SCS SDK 1.14** 公开 telemetry 与 input-device
接口的可复用 Rust 基座。它把官方 C ABI 转化为强类型 `no_std` binding，
提供彼此独立的安全 plugin runtime，并通过真实示例验证应用边界和最终产物。

整个 workspace 由纯 Rust 实现。插件作者无需引入 C/C++ shim、CMake、bindgen、
裸指针、手写导出符号或应用层 `unsafe`。

> [!IMPORTANT]
> 本仓库完整覆盖 SCS SDK 1.14 中的公开 **Telemetry API 1.00/1.01** 与
> **Input API 1.00**。覆盖声明严格限定在这些已审计接口，不外推到未来尚未出现
> 的 SDK surface。

> [!NOTE]
> 这是独立的社区项目，与 SCS Software 不存在隶属或背书关系。
> [`third-party/scs_sdk_1_14/`](third-party/scs_sdk_1_14/) 中的官方文件始终是
> ABI 与常量的事实来源。完整说明见[第三方声明](THIRD_PARTY_NOTICES.zh.md)。

## 为什么需要这个 workspace

| 需求 | 本仓库提供的能力 |
| --- | --- |
| 可审计的 SDK 覆盖 | 按 header 顺序保存 raw catalog 与 typed catalog，覆盖全部 107 个 channel、6 个 configuration ID、60 个 configuration attribute、6 个 gameplay event 与 15 个 gameplay attribute。 |
| 安全的应用代码 | 独立的 `TelemetryPlugin` 与 `InputPlugin` API 提供 typed value、index、game identity、compatibility 与显式 registration。 |
| 统一的 runtime 正确性 | Telemetry 事务与 Input device lifetime、panic containment、稳定 callback context、poison recovery 和 stale-callback isolation。 |
| 可信的产物验证 | Release 脚本会在链接与符号裁剪后检查 PE、ELF、Mach-O、x86-64 架构以及精确的 API-specific export set。 |
| 真实游戏证据 | 安全示例在 ETS2 中接收 6 类 event 与 8 个 channel；独立 probe 验证 loader 文档规定的 API fallback 顺序。 |

它刻意只做基座，而不是产品插件。Web bridge、调度逻辑、持久化、存档处理与用户
界面应当位于下游产品仓库。

## 从这里开始

这些 crate 目前在同一个 workspace 中协同开发，仓库内的 example 是标准集成入口：

```bash
git clone https://github.com/AptS-1547/scs-sdk-crates.git
cd scs-sdk-crates

cargo test --workspace --locked
scripts/check-plugin-boundary.sh
```

最小插件只依赖 `scs-sdk-plugin`。手写的应用源码可以直接禁止 unsafe code：

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

宏最终只生成两个 loader 可见的入口：

```text
scs_telemetry_init
scs_telemetry_shutdown
```

接下来可以阅读 [safe plugin framework 指南](crates/scs-sdk-plugin/)、
[真实 telemetry example](examples/telemetry-plugin/)或
[generic input-device example](examples/input-plugin/)。独立的
[semantical input fixture](examples/input-semantical-plugin/README.zh.md) 会展示无需 binding
UI 的直接 game-mix routing。

## 先看证据，再看承诺

主 example 既不是藏起来的产品 crate，也不是只做编译演示的片段。它是安全的应用
边界 fixture，会被构建为真实 `cdylib`、通过 CI 检查，并在 ETS2 进程内运行。

启动日志会同时报告 framework 身份、协商版本、检测到的游戏身份和已经提交的订阅：

```text
[scs-sdk-plugin] starting plugin name="SCS SDK Telemetry Example" version="0.1.0" framework_version="0.1.0"
[scs-sdk-plugin] detected game_display_name="Euro Truck Simulator 2 1.60.1.7s" game_id="eut2" telemetry_api=1.1 telemetry_schema=1.19
[scs-sdk-plugin] initialized plugin name="SCS SDK Telemetry Example" version="0.1.0" events=6 channels=8
```

进入游戏后，它会解码 typed snapshot、任务配置以及 SDK 1.14 的全部 6 种 gameplay
payload。下面是一条真实 snapshot 的形式：

```text
[scs-sdk-example] probe speed=85.3km/h rpm=1485 gear=16 \
position=(27077.895,2.983,-8572.040) heading=0.7134 \
navigation_distance=705.64km navigation_time=39887s \
speed_limit=80.0km/h cargo_damage=0.000
```

独立的 [`telemetry-fallback-plugin`](examples/telemetry-fallback-plugin/) 会故意以
`Unsupported` 拒绝 Telemetry API 1.01，只接受精确的 1.00，并证明 SCS loader
会先重试旧版 API，再投递 event 与 channel callback。它与普通 example 隔离，避免
混淆两套 negotiation contract。

## 四层结构，单向依赖

```text
examples/telemetry-plugin
        │ safe TelemetryPlugin API
        ▼
scs-sdk-plugin          lifecycle、registration、callback、runtime
        │ typed SDK operations
        ▼
scs-sdk                 no_std value、descriptor、catalog、decoding
        │ raw ABI
        ▼
scs-sdk-sys             no_std x86-64 C ABI definitions

examples/input-plugin 与 examples/input-semantical-plugin 通过独立 InputPlugin runtime
使用相同分层。scs-sdk-plugin-macros 为每个选定 API 生成对应的两个 export。
```

| 层 | 负责 | 不负责 |
| --- | --- | --- |
| [`scs-sdk-sys`](crates/scs-sdk-sys/) | Raw function pointer、union、structure、constant、catalog、layout assertion 与 ABI 所需的 `unsafe`。 | Typed application policy 或 plugin lifecycle。 |
| [`scs-sdk`](crates/scs-sdk/) | Typed value 与 descriptor、版本域、catalog enumeration、tagged-union decoding，以及 callback scope 内对 SCS 的调用。 | 全局 runtime state、product state 或导出符号。 |
| [`scs-sdk-plugin`](crates/scs-sdk-plugin/) | 安全的 plugin lifecycle、显式 registration、compatibility check、callback dispatch、rollback、shutdown 与 foreign-context ownership。 | 网络、存储、调度、UI 或存档功能。 |
| [`scs-sdk-plugin-macros`](crates/scs-sdk-plugin-macros/) | 把安全构造表达式展开成 telemetry 和/或 input loader export。 | Runtime policy 或产品行为。 |
| [`examples/telemetry-plugin`](examples/telemetry-plugin/) | 真实的安全插件与端到端边界 fixture。 | 产品功能。 |
| [`examples/input-plugin`](examples/input-plugin/) | 使用 typed bool/float event 的安全 generic input device。 | 硬件集成或产品功能。 |
| [`examples/input-semantical-plugin`](examples/input-semantical-plugin/) | 直接驱动官方 `light` bool mix 的独立安全 semantical device。 | Generic binding 或产品功能。 |

Cargo 依赖方向保持为：

```text
scs-sdk-sys ← scs-sdk ← scs-sdk-plugin ← application
                                     ↖ scs-sdk-plugin-macros
```

## Telemetry SDK 1.14 的强类型覆盖

清单直接源自官方 header，并通过测试检查名称、数量、顺序、重复项、value type、
association 以及 indexed/scalar 行为。

| Surface | 数量 | Typed 入口 |
| --- | ---: | --- |
| Channels | **107** | `channels::ALL` |
| Configuration IDs | **6** | `configuration::ids::ALL` |
| Configuration attributes | **60** | `configuration::attributes::ALL` |
| Configuration associations | **71** | `configuration::associations::ALL` |
| Gameplay events | **6** | `gameplay::events::ALL` |
| Gameplay attributes | **15** | `gameplay::attributes::ALL` |
| Gameplay associations | **21** | `gameplay::associations::ALL` |
| H-shifter values | **4** | `ShifterType::ALL` |
| Job-market values | **5** | `JobMarket::ALL` |
| Fine-offence values | **14** | `FineOffence::ALL` |

覆盖范围还包括 SDK 1.14 中所有公开 telemetry ABI value type、result code、delivery
flag、initialization structure、callback payload、game ID、Telemetry API version，以及
ETS2/ATS telemetry game-version constant。

Typed access 不会丢失未来数据。已知的 enum-like string 具有包含 `ALL`、`COUNT`、
`as_str`、`FromStr` 和 schema availability 的闭合集合；generic string access 仍会
原样保留未来的未知值。

## Input SDK 1.00 的强类型覆盖

Input 是独立 API surface，不会被硬塞进 `TelemetryPlugin`：

| Surface | 覆盖 |
| --- | --- |
| API version | Input API 1.00，以及彼此独立的 ETS2/ATS input game version |
| Device class | Generic 与 semantical |
| Device shape | 显式声明 1 至 400 个 bool/float input |
| Callback flag | First in frame、first after activation，并保留未知 bit |
| Lifecycle | 仅 init 可注册、可选 activity callback、重复 next-event poll、shutdown 前由 SCS 自动注销 |
| Export | `scs_input_init` 与 `scs_input_shutdown` |

`InputIndex`、`InputDeviceId`、`InputAxisValue`、`InputValue` 与
`InputEventFlags` 保持 callback domain 显式。`InputAxisValue` 只接受 -1.0 至 1.0
闭区间内的 finite normalized position；无效值会被拒绝，而不是静默 clamp。Runtime
会验证 device/input 名称、device-local index、注册 value type、panic containment，
以及部分初始化失败后保留的 stale context。安全示例与隔离宏 fixture 的应用源码均不含
`unsafe`。

独立 semantical fixture 遵循官方 SDK sample 的 `light` bool input。新的 controls 文件
会引用 `semantical.light?0`，所以游戏不需要用户 binding 就能激活并消费该 device。
确定性的 false/true cycle 会为第二种 device class 提供直接真实游戏证据，同时不把该
行为混进 Generic artifact。

## 所有契约都保持显式

- **订阅由插件声明，而不是 framework 猜测。** 实现 callback 不会触发注册，
  framework 也不会自动订阅整个 catalog。
- **Index domain 彼此独立。** `SdkIndex`、`TrailerIndex` 与
  `TrailerConfigurationId` 不会被静默互换；legacy `trailer` 与编号式
  `trailer.0` 始终不同。
- **Required 与 optional registration 的语义不同。** Optional channel 只容忍
  `NotFound` 和 `UnsupportedType`；optional event 只容忍 `Unsupported` 与
  `NotFound`。其他错误都会保留事务式回滚。
- **版本域彼此独立。** SDK archive suffix、协商得到的 `TelemetryApiVersion`、
  每个游戏的 `GameSchemaVersion` 与公开游戏版本不可互换。
- **未来 ABI 会被明确拒绝。** 未知 raw version 可用于诊断，但不会被解释为最新
  已知 layout。
- **游戏身份不会丢失。** ETS2 与 ATS 使用 typed variant；未知 game ID 会保留为
  owned `Other`，而不是被归类到某个已知游戏。
- **SDK 调用严格受 scope 限制。** `SdkCall` 不可存储，也不实现 `Send` 或
  `Sync`，与 SCS 的 callback scope 和主线程要求一致。

Capability history 同样是显式数据。每个 descriptor 与 association 都携带根据官方
SDK 1.0 至 1.14 header 独立整理的 ETS2/ATS schema minimum。Gameplay event 与
signed 64-bit value 需要 Telemetry API 1.01 之类的 API-level 要求，则始终与每个
游戏的 schema availability 分开。

## Runtime 安全模型

Framework 统一承担下游插件不应重复实现的生命周期机制：

- initialization 与 SDK registration 构成一个事务；
- partial failure 会按完成顺序的反方向回滚；
- normal shutdown 会逆序 unregister；
- 每一个 foreign ABI boundary 之前都进行 panic containment；
- mutex poison 具有明确恢复路径；
- callback context 拥有稳定的 allocation address 与有效 provenance；
- unregister 失败时保留 foreign-visible context，不释放 SDK 仍可能引用的内存；
- session generation 会隔离旧 session 的延迟 callback 与后续 plugin instance。

Unsafe operation 只留在最小且经过审计的 FFI/runtime 边界。Wrapper 不会读取 inactive
tagged-union member，也不会读取 SCS 没有义务初始化的 ABI padding。Callback
ownership model 通过 Miri strict provenance 验证。

## 构建经过验证的原生插件

在仓库根目录执行对应平台脚本。每个脚本都会先构建安全 example，再验证最终产物，
而不是相信 Cargo target 目录或文件扩展名。

| 平台 | Target 与兼容性 | 命令 | 产物 |
| --- | --- | --- | --- |
| Windows | x86-64 GNU | `scripts/build-windows-plugin.sh` | `scs_sdk_telemetry_example.dll` |
| Windows Input | x86-64 GNU | `scripts/build-windows-input-plugin.sh` | `scs_sdk_input_example.dll` |
| Windows Semantical Input | x86-64 GNU | `scripts/build-windows-input-semantical-plugin.sh` | `scs_sdk_input_semantical_example.dll` |
| Linux | x86-64，通过 Zig 保持 glibc 2.17 下限 | `scripts/build-linux-plugin.sh` | `libscs_sdk_telemetry_example.so` |
| Linux Input | x86-64，通过 Zig 保持 glibc 2.17 下限 | `scripts/build-linux-input-plugin.sh` | `libscs_sdk_input_example.so` |
| Linux Semantical Input | x86-64，通过 Zig 保持 glibc 2.17 下限 | `scripts/build-linux-input-semantical-plugin.sh` | `libscs_sdk_input_semantical_example.so` |
| macOS | x86-64；Apple Silicon 上通过 Rosetta | `scripts/build-macos-plugin.sh` | `libscs_sdk_telemetry_example.dylib` |
| macOS Input | x86-64；Apple Silicon 上通过 Rosetta | `scripts/build-macos-input-plugin.sh` | `libscs_sdk_input_example.dylib` |
| macOS Semantical Input | x86-64；Apple Silicon 上通过 Rosetta | `scripts/build-macos-input-semantical-plugin.sh` | `libscs_sdk_input_semantical_example.dylib` |

验证内容包括原生文件格式、x86-64 架构以及精确的 dynamic export 集合。macOS
build 还会为本地加载应用并验证 ad-hoc signature；这不等于 Developer ID signing
或 notarization。

安装路径、live log 检查与预期 runtime marker 见
[example 的平台和 ETS2 验证指南](examples/telemetry-plugin/README.zh.md#构建与验证)。

## Release 与 crates.io 发布

推送与 workspace version 完全一致的 tag（例如 `v0.1.0`）会启动
[Release workflow](.github/workflows/release.yml)。tag 指向的提交必须已经包含在
`origin/master` 中；指向未合并提交的 tag 会在任何发布动作开始前被拒绝。

Workflow 会先运行完整的质量、package、Miri 与平台 gate，然后按依赖顺序发布四个
crate：

```text
scs-sdk-sys -> scs-sdk
scs-sdk-plugin-macros -> scs-sdk-plugin
scs-sdk + scs-sdk-sys + scs-sdk-plugin-macros -> scs-sdk-plugin
```

`publish-crates` job 使用 GitHub environment `crates-io`，并要求其中存在
`CARGO_REGISTRY_TOKEN` secret。crates.io 已经可见的精确 crate version 会被跳过，
因此失败的 workflow 可以续跑，而不会尝试重复发布。新发布的底层依赖会通过 Cargo
registry index 进行有界轮询，确认可见后才发布依赖它的上层 crate。

四个 crate 全部可见后，workflow 才会创建 draft GitHub Release、上传全部资产、比较
远端资产清单与预期清单，最后把 draft 转为正式 Release。稳定 tag 会成为 latest；
`v0.2.0-rc.1` 之类的 semantic prerelease tag 会保持 prerelease 状态。

每个 Release 包含三个平台归档：

```text
scs-sdk-crates-v0.1.0-windows-x86_64.zip
scs-sdk-crates-v0.1.0-linux-x86_64-glibc-2.17.tar.gz
scs-sdk-crates-v0.1.0-macos-x86_64.tar.gz
```

每个归档都包含经过验证的 Telemetry、Generic Input 与 Semantical Input example 动态库，
以及中英文 README、workspace license、保留的 SCS SDK 声明和第三方声明。两个 Input
example 是互斥安装 fixture，同一时间最多安装其中一个；Telemetry example 可以与选定
的 Input example 共存。

Release 归档由 `checksums.txt` 覆盖，并通过 GitHub Actions keyless OIDC 使用 Sigstore
cosign 签名。验证时应使用精确 tag 与 workflow identity：

```bash
TAG=v0.1.0

cosign verify-blob checksums.txt \
  --signature checksums.txt.sig \
  --certificate checksums.txt.pem \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity \
    "https://github.com/AptS-1547/scs-sdk-crates/.github/workflows/release.yml@refs/tags/$TAG"

sha256sum -c checksums.txt
```

macOS 可用 `shasum -a 256 -c checksums.txt` 完成最后的哈希校验。

## 仓库地图

```text
crates/scs-sdk-sys/             raw no_std x86-64 ABI
crates/scs-sdk/                 safe no_std typed wrapper 与 catalog
crates/scs-sdk-plugin/          safe lifecycle/runtime/framework
crates/scs-sdk-plugin-macros/   exported-entry-point proc macro
examples/input-plugin/          安全的 generic input-device plugin
examples/input-semantical-plugin/
                                直接驱动 semantical light mix 的 E2E plugin
examples/telemetry-plugin/      真实的 safe application-boundary plugin
examples/telemetry-fallback-plugin/
                                手动 loader fallback E2E probe
scripts/                        boundary、build、install 与 artifact 检查
third-party/scs_sdk_1_14/       官方 SDK 1.14 分发
third-party/scs_sdk_history/    官方 SDK 1.0–1.14 历史与声明
```

| 接下来阅读 | 用途 |
| --- | --- |
| [`scs-sdk-plugin`](crates/scs-sdk-plugin/README.zh.md) | 编写安全插件并理解 lifecycle guarantee。 |
| [Telemetry example](examples/telemetry-plugin/README.zh.md) | 查看显式订阅、typed callback、build artifact 与真实 ETS2 验证。 |
| [Input example](examples/input-plugin/README.zh.md) | 查看显式 device registration 与 frame-scoped bool/float event generation。 |
| [Semantical input example](examples/input-semantical-plugin/README.zh.md) | 证明无需 controller binding 的 `semantical.light?0` 直接 routing。 |
| [`scs-sdk`](crates/scs-sdk/README.zh.md) | Typed descriptor、value、index、version、schema history 与 decoding。 |
| [`scs-sdk-sys`](crates/scs-sdk-sys/README.zh.md) | 审计 raw ABI 与官方 header mapping。 |
| [`scs-sdk-plugin-macros`](crates/scs-sdk-plugin-macros/README.zh.md) | 检查 exported-entry-point contract 与独立 consumer fixture。 |
| [Fallback E2E probe](examples/telemetry-fallback-plugin/README.zh.md) | 复现 SCS loader 从 API 1.01 到 1.00 的 negotiation。 |

## 开发与 CI

Workspace 固定使用 Rust `1.85.0`，并把 formatting/boundary check、workspace test、
Miri 与三个平台的 artifact build 保持为独立 CI gate。独立 proc-macro consumer
fixture 必须能编译为真实 `cdylib`，在 release linking 后保留精确的
telemetry/input export set，并验证 combined 四导出产物；缺少任一 plugin trait
时都必须产生预期的 trait-bound diagnostic。

<details>
<summary><strong>运行完整的本地 foundation gate</strong></summary>

```bash
cargo fmt --all -- --check
bash -n scripts/*.sh
scripts/check-license-copies.sh
scripts/check-plugin-boundary.sh
scripts/check-plugin-macro-fixtures.sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --locked
cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
MIRIFLAGS=-Zmiri-strict-provenance \
  cargo +nightly-2026-04-12 miri test --locked -p scs-sdk-plugin
git diff --check
git diff --cached --check
```

Release artifact 或 export 发生变化时，还要执行上面的对应 Windows、Linux 与 macOS
build script。只在 host 平台通过 Cargo build 不能证明跨平台产物契约。

</details>

## 许可证与归属

本项目自行编写的 Rust 代码可由用户选择
[Apache License 2.0](LICENSE-APACHE) 或 [MIT](LICENSE-MIT) 许可。

官方 SDK 文件以及来源于 SCS SDK 的 ABI declaration、constant、identifier、catalog、
schema-history metadata 与相关文档保留 SCS Software 的声明。SDK 1.0–1.5 使用
[2013 年声明](LICENSE-SCS-SDK-2013)，SDK 1.6–1.14 使用
[2016 年声明](LICENSE-SCS-SDK-2016)。这些材料不会被重新许可为 workspace license。

完整归属与项目独立性说明见[第三方声明](THIRD_PARTY_NOTICES.zh.md)。
