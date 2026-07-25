# SCS SDK Input 示例

**中文** | [English](README.md)

这个 crate 是 `scs-sdk-crates` 对 SCS Input API 1.00 支持的 safe application
边界示例和真实 loader fixture。它可以为 Euro Truck Simulator 2 或 American
Truck Simulator 构建原生插件，同时不让手写 application 代码接触裸指针、C 字符串、
外部 ABI 声明或 `unsafe`。

这个设备刻意保持确定性。它不读取真实硬件，也不实现网络、bridge、调度、持久化或
其他产品行为。

## 展示的公共契约

示例只依赖 `scs-sdk-plugin`，并实现 `InputPlugin`。application 代码显式完成：

1. 声明插件 metadata 与 Input API compatibility；
2. 在 `InputPlugin::initialize` 中注册一个 device；
3. 显式启用 device activity notification；
4. 处理 SCS 每帧发起的 pull request；
5. 使用 device-local `InputIndex` 返回 typed float 与 bool event；
6. 当前事件序列结束时返回 `None`；
7. shutdown 时重置自身状态。

两个 Input loader 入口由下面这行生成：

```rust
scs_sdk_plugin::export_input_plugin!(Plugin::default());
```

最终动态库恰好导出：

```text
scs_input_init
scs_input_shutdown
```

Input runtime 与 Telemetry runtime 相互独立。另一个插件动态库可以同时提供
Telemetry 入口；产品确实需要两个 API 时，也可以在同一个 `cdylib` 中各调用一次
export macro。

## 兼容性契约

示例只接受当前已经审计的公共接口：

| 版本域 | 要求 |
| --- | --- |
| Input API | 1.00 |
| ETS2 Input game version | 1.00，或 major 1 内更高的兼容 minor |
| ATS Input game version | 1.00，或 major 1 内更高的兼容 minor |
| 架构 | Windows、Linux 与 macOS x86-64 插件进程 |

Input API version、per-game Input version、Telemetry API version、Telemetry
schema version 和公开 game version 是相互独立的版本域。runtime 验证 Input
版本时不会借用 Telemetry compatibility。

## Device 行为

插件注册一个名为 `scs_sdk_input_example` 的 generic device，包含以下两个严格按序的
input：

| Index | Configuration name | Display name | Value type | 行为 |
| ---: | --- | --- | --- | --- |
| 0 | `example_axis` | `Example Axis` | `Float` | 每 60 个 input frame 增加 0.125，从 -1.0 到 1.0 后重置为 -1.0。 |
| 1 | `example_button` | `Example Button` | `Bool` | 每 120 个 input frame 翻转一次。 |

顺序是 device contract 的一部分。`InputIndex` 只属于当前 device，不能与 Telemetry
SDK index 或 device identity 混用。

示例显式请求 activity notification，只在 SCS 报告 device active 时产生 event。
request 带有 `first_in_frame` 时，插件推进确定性状态并重置 event cursor；随后依次
返回 axis、button，最后返回 `None`。framework 会把 `None` 映射为 SDK 的
`NotFound`，表示本轮序列结束。

示例保留官方 sample 的 0.125 步长，但刻意把 probe 扩展到 -1.0 至 1.0，以便真实游戏
E2E 同时覆盖负 float。这个 signed sweep 也是 safe wrapper 的 normalized axis 契约。
Application 代码必须先构造 `InputAxisValue`，才能返回 `InputValue::Float`；构造器会拒绝
NaN、正负无穷，以及 -1.0 至 1.0 之外的有限值，而不是静默 clamp。

`first_after_activation` 会作为 E2E marker 写入日志，但它不会代替
`first_in_frame`；官方 API 中这两个 flag 含义不同。

## Safe application 边界

在仓库根目录运行：

```bash
scripts/check-plugin-boundary.sh examples/input-plugin
```

示例使用 `#![forbid(unsafe_code)]`。boundary audit 还会拒绝裸指针、手写 ABI 函数、
C 字符串类型、直接访问 `scs-sdk-sys`，以及 macro 私有路径
`scs_sdk_plugin::__private`。必要的 FFI、callback context、panic containment 和
lifetime 处理都收口在经过审计的 sys、wrapper 与 runtime crate 中。

## 构建与验证

以下命令都从仓库根目录执行。每个脚本会构建 release 动态库，并验证原生文件格式、
x86-64 架构与精确的两个 loader export。macOS 脚本还会应用并验证 ad-hoc 签名。

### Windows x86-64

```bash
scripts/build-windows-input-plugin.sh
```

产物：

```text
target/x86_64-pc-windows-gnu/release/scs_sdk_input_example.dll
```

### Linux x86-64，glibc 2.17 baseline

```bash
scripts/build-linux-input-plugin.sh
```

产物：

```text
target/x86_64-unknown-linux-gnu/release/libscs_sdk_input_example.so
```

### macOS x86-64

```bash
scripts/build-macos-input-plugin.sh
```

产物：

```text
target/x86_64-apple-darwin/release/libscs_sdk_input_example.dylib
```

即使宿主机是 Apple Silicon，macOS target 仍然保持 x86-64，因为当前 SCS 游戏插件
进程通过 Rosetta 运行。

## 安装与操作

替换插件前应完整退出游戏。macOS 上使用下面两条命令构建并安装经过验证的私有副本：

```bash
scripts/build-macos-input-plugin.sh
scripts/install-macos-input-plugin.sh
```

installer 会保留 release artifact，在私有副本上移除 quarantine、应用 ad-hoc 签名、
验证精确的两个 Input export，只替换 Input 示例的精确文件名，并再次验证安装目标。
Telemetry fixture 与无关插件会保持原样，因为两套 SCS 接口可以共存。

其他平台手动安装时，把对应产物复制到游戏的原生 plugin 目录：

| 平台 | Plugin 目录 |
| --- | --- |
| Windows | `<GAME_INSTALL>/bin/win_x64/plugins/` |
| Linux | `<GAME_INSTALL>/bin/linux_x64/plugins/` |
| macOS | `<GAME_APP>/Contents/MacOS/plugins/` |

不要用不同文件名同时安装两份 Input 示例。Telemetry 示例可以保留，因为它导出的是
另一套 loader 接口。

游戏启动后，在 `game.log.txt` 中确认 library load 与以下 runtime marker：

```text
[scs-sdk-plugin/input] starting plugin name="SCS SDK Input Example" ...
[scs-sdk-plugin/input] detected ... input_api=1.0 input_game_version=1.0
[scs-sdk-input-example] registered generic device with 2 inputs
[scs-sdk-plugin/input] initialized ... devices=1
```

打开游戏的输入/控制器设置，绑定或激活示例 input，让 SCS 开始轮询 device。activity
切换、激活后的第一次 poll 和一组有界的完整事件序列会显示为：

```text
[scs-sdk-input-example] device active=true
[scs-sdk-input-example] first poll after activation
[scs-sdk-input-example] emitted index=0 type=float value=...
[scs-sdk-input-example] emitted index=1 type=bool value=...
[scs-sdk-input-example] event sequence exhausted
```

这些 marker 证明真实游戏调用了 activity 与 event trampoline，覆盖了两种 raw union
表示，并一直轮询到 `NotFound` 序列边界。每次 activation 只输出一次，因此不会刷爆
`game.log.txt`。

最终退出游戏时确认：

```text
[scs-sdk-input-example] shutdown
[scs-sdk-plugin/input] shutdown complete plugin name="SCS SDK Input Example" ...
unloaded 'libscs_sdk_input_example'
```

release E2E gate 要求从同一次真实游戏运行中保留完整的 load、registration、
activation、float、bool、exhaustion、shutdown 与 unload 序列。macOS ETS2 的实时日志
位于：

```text
~/Library/Application Support/Euro Truck Simulator 2/game.log.txt
```

## 开发检查

```bash
cargo fmt --all -- --check
scripts/check-plugin-boundary.sh examples/input-plugin
scripts/check-plugin-macro-fixtures.sh
cargo test --locked -p scs-sdk-input-example
cargo clippy --locked -p scs-sdk-input-example --all-targets -- -D warnings
```

涉及 export 或 artifact 行为的修改，还必须运行上面列出的三个平台构建脚本。

## 许可证

Workspace 自有 Rust 代码可由你选择使用 Apache License 2.0 或 MIT。SCS-derived ABI
名称与契约仍受仓库根目录记录的原始 SCS SDK 声明约束。本示例是独立社区项目，与 SCS
Software 不存在隶属或官方背书关系。
