# SCS Telemetry Loader Fallback E2E 探针

**中文** | [English](README.md)

这个 crate 是一个有意保持专用的真实 ETS fixture，用来验证 SCS telemetry
loader 的 API 版本 fallback 规则。它不是普通兼容策略，也不是产品插件。

探针会故意以 `Unsupported` 拒绝 Telemetry API 1.01，只接受精确的 API 1.00，
然后证明已接受的 1.00 session 可以通过 safe plugin framework 同时交付一个
event callback 和一个 typed channel callback。

## 被测试的契约

官方 loader 会从新到旧尝试 API 版本，并且只在 `scs_telemetry_init` 返回
`SCS_RESULT_unsupported` 时继续尝试。

对于 SDK 1.14 中两个已经审计的 telemetry API，该 fixture 预期：

| Attempt | 产品结果 | 预期 loader 行为 |
| --- | --- | --- |
| Telemetry API 1.01 | `SdkError::Unsupported` | 清理被拒绝的 attempt，并重试旧 API。 |
| Telemetry API 1.00 | `Ok(())` | 提交注册并保持插件活跃。 |

完整顺序是：

```text
API 1.01
  -> product initialize
  -> SCS_RESULT_unsupported
  -> attempt-local product shutdown
  -> runtime 回到可重试状态
API 1.00
  -> product initialize
  -> 提交两个 event 与一个 channel
  -> SCS_RESULT_ok
```

插件把 API 1.00 声明为 compatibility minimum。这是有意设计：最初的 1.01
attempt 和最终的 1.00 retry 都必须进入 product initialization，再由探针应用
精确版本策略。

## 接受后的 API 1.00 surface

成功 attempt 只注册兼容 API 1.00 的 capability：

| 类别 | Capability | 证据 |
| --- | --- | --- |
| Event | `Started` | 记录每次 accepted session start。 |
| Event | `FrameEnd` | 设置严格 callback 证明的 event 一侧。 |
| Channel | `truck.speed` | 解码真实 `f32` value，并设置严格 callback 证明的 channel 一侧。 |

framework 初始化日志必须报告：

```text
events=2 channels=1
```

Gameplay event 与 signed 64-bit value 被有意排除，因为这些 representation 需要
Telemetry API 1.01。

## 严格 callback 证明

注册成功本身不能证明游戏真实交付了 callback。因此探针独立记录两个观察结果：

```text
frame_end_seen == true
latest_speed_metres_per_second.is_some()
```

只有两个条件同时成立时，探针才输出一次 confirmation：

```text
[scs-sdk-fallback-example] fallback callbacks confirmed \
telemetry_api=1.0 frame_end_seen=true speed_metres_per_second=0
```

SCS 没有承诺 changed channel value 与 `FrameEnd` 谁先到达。探针会在两条 callback
路径之后都检查 readiness，所以两种顺序都有效。停车时的 `0.0` 仍是真实 channel
delivery 证据；只有 `None` 表示还没有观察到 speed value。

浮点或物理状态可能产生非常接近零的微小 signed value。例如
`-0.000000631284 m/s` 大约是 `-0.00000227 km/h`；它能证明 value delivery，
但不是有意义的倒车证据。

## 预期真实游戏日志

ETS2 实时日志位于：

```text
~/Library/Application Support/Euro Truck Simulator 2/game.log.txt
```

成功的一轮包含下面的顺序。

### 1. 加载正确 library

```text
loading 'libscs_sdk_telemetry_fallback_example' '.../libscs_sdk_telemetry_fallback_example.dylib'
```

### 2. 故意拒绝 API 1.01

```text
[scs-sdk-plugin] detected ... telemetry_api=1.1 telemetry_schema=...
[scs-sdk-fallback-example] requesting loader retry \
rejected_telemetry_api=1.1 accepted_telemetry_api=1.0 result=unsupported
<ERROR> plugin initialization failed: fallback E2E intentionally rejects telemetry API 1.1; retry 1.0
[scs-sdk-fallback-example] rejected attempt cleaned telemetry_api=1.1
```

initialization-error 行是预期证据。返回其他结果会停止官方 loader 的 retry
sequence。

### 3. 接受 API 1.00

```text
[scs-sdk-plugin] detected ... telemetry_api=1.0 telemetry_schema=...
[scs-sdk-fallback-example] accepted loader fallback \
telemetry_api=1.0 expected_telemetry_api=1.0
[scs-sdk-plugin] initialized ... events=2 channels=1
```

游戏 telemetry schema 仍然是独立版本域。现代 ETS2 build 使用 API 1.00 时，
仍然可能报告 1.19 之类的更新 schema。

### 4. 两条 callback 路径都已交付

```text
[scs-sdk-fallback-example] fallback session started telemetry_api=1.0
[scs-sdk-fallback-example] fallback callbacks confirmed \
telemetry_api=1.0 frame_end_seen=true speed_metres_per_second=...
```

### 5. 干净 shutdown

```text
[scs-sdk-fallback-example] fallback session shutdown \
telemetry_api=1.0 callbacks_confirmed=true
[scs-sdk-plugin] shutdown complete \
plugin name="SCS SDK Telemetry Fallback E2E" version="..."
unloaded 'libscs_sdk_telemetry_fallback_example'
```

## Safety 边界

该 crate 中的全部手写源码都是 safe Rust，并带有：

```rust
#![forbid(unsafe_code)]
```

它只直接依赖 `scs-sdk-plugin`，并通过 framework 的公开 `sdk` re-export 使用
SDK 类型。它不包含裸指针、手写 ABI export、C 字符串处理、直接 sys-crate
访问或宏私有访问。

在仓库根目录验证该边界：

```bash
scripts/check-plugin-boundary.sh examples/telemetry-fallback-plugin
```

## 在 macOS 构建与验证

从仓库根目录运行：

```bash
scripts/build-macos-fallback-plugin.sh
```

产物：

```text
target/x86_64-apple-darwin/release/libscs_sdk_telemetry_fallback_example.dylib
```

脚本会：

1. 构建 `x86_64-apple-darwin` release cdylib；
2. 应用 ad-hoc signature；
3. 验证 Mach-O shared-library 格式；
4. 验证 x86-64 架构；
5. 验证 embedded signature；
6. 验证 external export set 精确等于 `_scs_telemetry_init` 与
   `_scs_telemetry_shutdown`。

即使宿主机是 Apple Silicon，target 仍然是 x86-64，因为当前 macOS ETS2
进程通过 Rosetta 加载 x86-64 插件。

## 在 macOS 安装

完整退出 ETS2 后运行：

```bash
scripts/install-macos-fallback-plugin.sh
```

installer 会先验证新产物和安装目标，然后只移除下面这些精确 alternate filename：

```text
libscs_sdk_telemetry_example.dylib
libets2_dispatch_telemetry_rust.dylib
```

普通示例与 fallback 示例互斥。只保留一个探针，可以让游戏日志只有一套清晰的
lifecycle 和 negotiation sequence。

切回普通示例：

```bash
scripts/install-macos-plugin.sh
```

普通的 6-event/8-channel 示例见
[`../telemetry-plugin/README.zh.md`](../telemetry-plugin/README.zh.md)。

## 开发检查

至少运行：

```bash
cargo fmt --all -- --check
scripts/check-plugin-boundary.sh examples/telemetry-fallback-plugin
cargo test --locked -p scs-sdk-telemetry-fallback-example
cargo clippy --locked -p scs-sdk-telemetry-fallback-example --all-targets -- -D warnings
scripts/build-macos-fallback-plugin.sh
```

单元测试覆盖精确的 API 1.00 acceptance policy，以及严格、one-shot 的双 callback
readiness 条件。真实 ETS2 测试仍然负责提供 loader negotiation 和 callback
delivery 证据。

## 非目标

该 fixture 不应扩张成：

- 普通插件拒绝 loader 最新已审计 API 的建议；
- 产品兼容策略；
- bridge、网络服务、dispatcher 或持久化层；
- 普通 telemetry 示例的替代品。

它的职责很窄：证明官方 loader fallback sequence 与接受后的 API 1.00 callback
路径。

## 许可证

workspace 自编 Rust 代码使用 **MIT OR Apache-2.0**。SCS SDK 1.0-1.5 的归属
声明保存在 [`LICENSE-SCS-SDK-2013`](../../LICENSE-SCS-SDK-2013)，SDK 1.6-1.14
的归属声明保存在 [`LICENSE-SCS-SDK-2016`](../../LICENSE-SCS-SDK-2016)。完整说明见
仓库[第三方声明](../../THIRD_PARTY_NOTICES.zh.md)。本示例是独立社区项目，与
SCS Software 不存在隶属或官方背书关系。
