# SCS SDK Telemetry 普通示例

**中文** | [English](README.md)

这个 crate 是 `scs-sdk-crates` 的主要实机示例。它会构建一个原生 SCS
telemetry 插件，同时保证所有手写 application 代码都留在 framework 边界的
safe Rust 一侧。

它同时承担两种职责：

- 提供一个可直接阅读的 `TelemetryPlugin` 实现示例；
- 作为在真实 Euro Truck Simulator 2 进程中运行的端到端 fixture。

它不是 ETS2 Dispatch 产品插件。网络、网页应用、任务调度、持久化、存档解析和
其他产品行为属于独立仓库。

## 示例展示了什么

插件只使用公开的 `scs-sdk-plugin` API 及其 typed `sdk` re-export。
application 代码显式完成：

1. 声明插件 metadata 与 compatibility requirements；
2. 在初始化阶段逐项订阅需要的 event 和 channel；
3. 接收 typed channel update，并构建 latest-value snapshot；
4. 处理 lifecycle、configuration 和 gameplay event；
5. 通过游戏日志 callback 输出有频率上限的诊断信息；
6. 在初始化与 shutdown 时重置产品状态。

SCS 导出入口由下面这行生成：

```rust
export_plugin!(TelemetryExample::default());
```

application 不声明 ABI 函数，不检查裸指针，不使用 C 字符串，也不直接调用
`scs-sdk-sys`。

## 兼容性契约

该示例要求：

| 版本域 | 最低版本 | 原因 |
| --- | ---: | --- |
| Telemetry API | 1.01 | Gameplay event 与 signed 64-bit gameplay value 需要 API 1.01。 |
| ETS2 telemetry schema | 1.14 | SCS 在这个游戏 schema 中引入 gameplay event descriptor。 |
| 游戏 | Euro Truck Simulator 2 | 当前 compatibility declaration 显式列出 ETS2。 |

Telemetry API、游戏 telemetry schema 和公开游戏版本是三个独立的版本域。真实
启动日志因此可能同时出现：

```text
game_display_name="Euro Truck Simulator 2 1.60.1.7s"
telemetry_api=1.1
telemetry_schema=1.19
```

## 显式订阅

### Event 类别

示例显式注册全部 6 种 telemetry event 类别：

| Event | 示例用途 |
| --- | --- |
| `FrameStart` | 记录 real-time render timestamp，并检测 timer restart。 |
| `FrameEnd` | 在一帧更新结束后输出限频 snapshot。 |
| `Paused` | telemetry 暂停时停止输出驾驶 snapshot。 |
| `Started` | telemetry 开始时恢复驾驶 snapshot。 |
| `Configuration` | 解码并记录当前任务配置。 |
| `Gameplay` | 解码 SDK 1.14 的全部 6 种 gameplay payload。 |

framework 初始化日志应报告：

```text
events=6
```

### Channel

示例注册 8 个 scalar channel：

| Channel | SDK 单位/值 | 示例用途 |
| --- | --- | --- |
| `truck.world.placement` | placement | 世界坐标与 heading。 |
| `truck.speed` | 米每秒 | 显示时转换为公里每小时。 |
| `truck.engine.rpm` | 每分钟转数 | 发动机状态探针。 |
| `truck.engine.gear` | signed integer | 当前发动机挡位。 |
| `truck.navigation.distance` | 米 | 剩余路线距离，显示为公里。 |
| `truck.navigation.time` | 秒 | 剩余导航时间。 |
| `truck.navigation.speed.limit` | 米每秒 | 当前导航限速，显示为公里每小时。 |
| `job.cargo.damage` | 比例 | 最新货损值。 |

framework 初始化日志应报告：

```text
channels=8
```

各 channel callback 相互独立。`FrameEnd` 读取每个字段最近收到的值；示例不会
假设 SCS 在同一帧内按某种固定顺序调用 channel callback。

## 日志行为

### Snapshot 探针

telemetry 活跃时，示例每秒最多输出一份 snapshot：

```text
[scs-sdk-example] probe speed=85.3km/h rpm=1485 gear=16 \
position=(27077.895,2.983,-8572.040) heading=0.7134 \
navigation_distance=705.64km navigation_time=39887s \
speed_limit=80.0km/h cargo_damage=0.000
```

存储的 SDK speed 不会被修改。只有显示值会把非常接近零的 signed floating-point
zero 规范成零，避免输出嘈杂的 `-0.0km/h`。

### 任务配置

当前任务日志会同时记录 SCS 提供的显示名称与稳定 ID：

```text
[scs-sdk-example] job cargo=轿车 cargo_id=cars_fr mass=9010kg \
source=Amsterdam(amsterdam)/TDC Auto Terminal(tdc_auto) \
destination=Panevezys(panevezys)/BHV(bhv) \
market_raw=freight_market market_known=Some(FreightMarket) \
income=69663 planned_distance=1614km delivery_time=110550 \
cargo_loaded=true special_job=false
```

已知 enum-like value 与原始 SDK 字符串会刻意同时保留。以后新增的值即使还没有
进入当前 typed catalog，也仍然能在诊断日志中看到。

### Gameplay event

示例处理 SDK 1.14 header 定义的每一种 gameplay event：

| SDK event | 示例 marker |
| --- | --- |
| `job.delivered` | `[scs-sdk-example] job delivered ...` |
| `job.cancelled` | `[scs-sdk-example] job cancelled ...` |
| `player.fined` | `[scs-sdk-example] player fined ...` |
| `player.tollgate.paid` | `[scs-sdk-example] tollgate paid ...` |
| `player.use.ferry` | `[scs-sdk-example] ferry used ...` |
| `player.use.train` | `[scs-sdk-example] train used ...` |

一次真实 ETS2 收费站验证产生了：

```text
[tollgate] Activated 'end' tollgate
[scs-sdk-example] tollgate paid amount=79
```

这段顺序证明 `pay.amount` gameplay attribute 已经真实交付，并完成 typed signed
64-bit 解码；它不只是证明 generic gameplay event 类别注册成功。

## Safe application 边界

在仓库根目录运行：

```bash
scripts/check-plugin-boundary.sh examples/telemetry-plugin
```

该审计会拒绝手写 application 源码中的 raw ABI 访问，包括 `unsafe`、裸指针、
external ABI declaration、C 字符串类型、直接访问 `scs-sdk-sys`，以及使用宏专用
的 `scs_sdk_plugin::__private` 模块。

必要的 foreign-boundary 代码仍然收口并审计在更低层的 framework、wrapper 和
sys crate 中。

## 构建与验证

以下命令都从仓库根目录执行。

### Windows x86-64

```bash
scripts/build-windows-plugin.sh
```

产物：

```text
target/x86_64-pc-windows-gnu/release/scs_sdk_telemetry_example.dll
```

### Linux x86-64，glibc 2.17 baseline

```bash
scripts/build-linux-plugin.sh
```

产物：

```text
target/x86_64-unknown-linux-gnu/release/libscs_sdk_telemetry_example.so
```

### macOS x86-64

```bash
scripts/build-macos-plugin.sh
```

产物：

```text
target/x86_64-apple-darwin/release/libscs_sdk_telemetry_example.dylib
```

即使宿主机是 Apple Silicon，macOS target 仍然是 x86-64，因为当前 ETS2 可执行
文件和插件进程通过 Rosetta 运行。仓库脚本会验证原生文件格式、x86-64 架构与
精确的 loader-visible export。macOS 构建还会为本地使用应用并验证 ad-hoc
signature。

## 在 macOS 安装

完整退出 ETS2 后运行：

```bash
scripts/install-macos-plugin.sh
```

installer 会：

1. 在构建产物的私有副本上操作；
2. 在存在 quarantine 时从副本移除它；
3. 应用 ad-hoc signature；
4. 验证 Mach-O 格式、x86-64 架构、签名和 exports；
5. 把已验证产物复制进 ETS2 application bundle；
6. 再次验证安装目标；
7. 只移除 fallback 与 legacy example 的精确文件名。

普通示例和 fallback 示例是互斥的 E2E 探针。切换到 loader fallback 测试时，
使用 [`../telemetry-fallback-plugin/README.zh.md`](../telemetry-fallback-plugin/README.zh.md)
中的独立说明。

## 真实 ETS2 验证

实时日志位于：

```text
~/Library/Application Support/Euro Truck Simulator 2/game.log.txt
```

启动时确认：

```text
loading 'libscs_sdk_telemetry_example'
[scs-sdk-plugin] starting plugin name="SCS SDK Telemetry Example" ...
[scs-sdk-plugin] detected ... telemetry_api=1.1 telemetry_schema=...
[scs-sdk-plugin] initialized ... events=6 channels=8
```

进入驾驶场景后，根据测试目标触发相关行为：开始或暂停 telemetry、接受或加载
任务、经过收费站、收到罚款，或者使用渡轮/火车。最终退出游戏时确认：

```text
[scs-sdk-example] example state shutdown
[scs-sdk-plugin] shutdown complete plugin name="SCS SDK Telemetry Example" ...
unloaded 'libscs_sdk_telemetry_example'
```

ETS2 每次启动进程都会重写 `game.log.txt`。需要长期保留 E2E 证据时，应及时复制
该日志。

## 开发检查

至少运行：

```bash
cargo fmt --all -- --check
scripts/check-plugin-boundary.sh examples/telemetry-plugin
cargo test --locked -p scs-sdk-telemetry-example
cargo clippy --locked -p scs-sdk-telemetry-example --all-targets -- -D warnings
scripts/check-plugin-macro-fixtures.sh
```

涉及插件行为或 exports 的修改，还应运行上面列出的对应平台构建与验证脚本。

## 许可证

workspace 自编 Rust 代码使用 **MIT OR Apache-2.0**。`third-party/` 下的官方
SCS SDK 文件仍然受 SCS 自己的许可证约束。
