# SCS SDK Semantical Input 示例

**中文** | [English](README.md)

这个 crate 是 SCS Input API 1.00 semantical device class 的独立 safe Rust
fixture。它可以为 Euro Truck Simulator 2 或 American Truck Simulator 构建真实
原生插件，同时让手写 application 代码不接触裸指针、C 字符串、外部 ABI 声明和
`unsafe`。

fixture 遵循官方 SDK 1.14 的 `input_semantical` example：注册一个名为 `light`
的 bool input。SCS 会把该名称直接映射到游戏的 `semantical.light?0` mix，因此
插件不需要经过控制器绑定步骤。

这个 crate 与 `examples/input-plugin` 分开存在。Generic example 证明可以由用户绑定的
bool 与 normalized float input；本 example 证明直接的 semantical mix routing。分成两个
原生 artifact 后，真实游戏结果才能明确归属于一种 device class。

## 展示的公共契约

示例只依赖 `scs-sdk-plugin`，并实现 `InputPlugin`。Application 代码显式完成：

1. 声明 metadata 与 Input API compatibility；
2. 在初始化期间注册一个 semantical device；
3. 声明精确的 `light` bool input；
4. 显式启用 activity notification；
5. 响应 SCS 每帧发起的 event polling；
6. 每轮唯一 event 返回后再返回 `None`；
7. shutdown 时重置状态。

Loader 入口由安全的 macro 调用生成：

```rust
scs_sdk_plugin::export_input_plugin!(Plugin::default());
```

最终动态库精确导出：

```text
scs_input_init
scs_input_shutdown
```

## 为什么 input 必须叫 `light`

官方 SDK sample 注册的是：

```text
device type: semantical
input name: light
value type: bool
```

SDK 文档规定 semantical input name 会直接映射到同名 game mix。新的 ETS2 controls
文件包含等价于下面的表达式：

```text
mix light `keyboard.l?0 | semantical.light?0`
```

macOS 当前 profile 的文件通常叫 `controls_osx.sii`；Windows 与 Linux 通常使用
`controls.sii`。文件名与平台有关，但 `semantical.light?0` 契约相同。

Semantical device 不会作为普通可绑定控制器 input 出现在 binding UI 中。这是预期
行为：游戏直接消费 `light`；如果未来 game mix 改名，需要更新插件，而不是让用户
重新绑定。

## 确定性行为

官方 C++ sample 根据 wall clock 翻转。本 fixture 改为每 60 个 input frame 翻转，
使测试和日志可复现：

| Index | Mix name | Display name | Value type | 行为 |
| ---: | --- | --- | --- | --- |
| 0 | `light` | `Lights` | `Bool` | 初始为 `false`，60 个 input frame 后变成 `true`，以后每 60 帧继续翻转。 |

每当 request 带有 `first_in_frame`，fixture 会推进状态并重置 device-local event
cursor。随后返回一次 `light` event，再返回 `None`；framework 会把 `None` 映射为
SDK 的 `NotFound`，结束当前序列。

日志刻意保持有界。每次 activation 只记录首次 `false`、首次 `true` 和一次 exhaustion
marker。后续 frame 仍会继续驱动 light mix，但不会刷爆 `game.log.txt`。

## 兼容性契约

| 版本域 | 要求 |
| --- | --- |
| Input API | 精确 1.00 |
| ETS2 Input game version | 1.00，或 major 1 内更高的兼容 minor |
| ATS Input game version | 1.00，或 major 1 内更高的兼容 minor |
| 插件架构 | Windows、Linux 与 macOS x86-64 |

Input API version 与 per-game Input version 始终独立于 Telemetry API version、
Telemetry schema version 和公开 game version。

## Safe application 边界

```bash
scripts/check-plugin-boundary.sh examples/input-semantical-plugin
```

源码使用 `#![forbid(unsafe_code)]`。Boundary audit 还会拒绝裸指针、手写 ABI
function、C string type、直接访问 `scs-sdk-sys`，以及 framework-private macro
support path。FFI、panic containment、callback ownership 与 SDK result conversion
全部留在经过审计的 crate 内。

## 构建与验证

从仓库根目录执行命令。每个脚本都会构建 release 动态库，并验证原生格式、x86-64
架构和精确的两个 Input export。Linux 构建保持 glibc 2.17 下限；macOS 构建会应用
并验证 ad-hoc signature。

| 平台 | 命令 | Artifact |
| --- | --- | --- |
| Windows x86-64 | `scripts/build-windows-input-semantical-plugin.sh` | `target/x86_64-pc-windows-gnu/release/scs_sdk_input_semantical_example.dll` |
| Linux x86-64 | `scripts/build-linux-input-semantical-plugin.sh` | `target/x86_64-unknown-linux-gnu/release/libscs_sdk_input_semantical_example.so` |
| macOS x86-64 | `scripts/build-macos-input-semantical-plugin.sh` | `target/x86_64-apple-darwin/release/libscs_sdk_input_semantical_example.dylib` |

即使宿主机是 Apple Silicon，macOS artifact 仍保持 x86-64，因为当前 SCS 游戏插件
进程通过 Rosetta 运行。

## 在 macOS 安装并测试

完整退出 ETS2，然后执行：

```bash
scripts/build-macos-input-semantical-plugin.sh
scripts/install-macos-input-semantical-plugin.sh
```

Installer 会使用私有副本、清除 quarantine、应用 ad-hoc signature、验证两个 export、
写入选定 dylib、验证安装后的文件，然后才删除 Generic Input fixture 的精确文件名。
Telemetry 动态库和无关插件保持不动。

Generic 与 semantical Input example 都导出 `scs_input_init` 和
`scs_input_shutdown`，因此它们是互斥的真实游戏 fixture。使用 Generic installer
可以切换回去；不要把任一 Input 动态库复制成其他名字并排放置。

不需要进入 binding UI。启动游戏，进入能够观察车辆灯光的状态，等待 semantical
device 翻转 `light` mix。macOS 的实时日志位于：

```text
~/Library/Application Support/Euro Truck Simulator 2/game.log.txt
```

预期 startup 与 callback 证据：

```text
[scs-sdk-plugin/input] starting plugin name="SCS SDK Semantical Input Example" ...
[scs-sdk-plugin/input] detected ... input_api=1.0 input_game_version=1.0
[scs-sdk-input-semantical-example] registered device_type=semantical inputs=1 mix=light value_type=bool ...
[scs-sdk-input-semantical-example] device active=true
[scs-sdk-input-semantical-example] first poll after activation
[scs-sdk-input-semantical-example] emitted index=0 mix=light type=bool value=false
[scs-sdk-input-semantical-example] event sequence exhausted
[scs-sdk-input-semantical-example] emitted index=0 mix=light type=bool value=true
```

退出时确认 framework shutdown 与 library unload：

```text
[scs-sdk-input-semantical-example] shutdown
[scs-sdk-plugin/input] shutdown complete plugin name="SCS SDK Semantical Input Example" ...
unloaded 'libscs_sdk_input_semantical_example'
```

完整真实游戏 E2E 证据应来自同一次运行，并包含 load、版本识别、semantical
registration、自动 activation、两个 bool 状态、exhaustion、可见灯光行为、shutdown
和 unload。

## 已确认的 ETS2 E2E

macOS x86-64 artifact 已于 2026-07-26 在 Euro Truck Simulator 2 `1.60.1.7s`
中完成真实测试。游戏成功加载动态库、协商 Input API 1.00 与 Input game version 1.00、
注册单 input semantical device、在没有 binding 操作的情况下自动激活、消费 `false`
和 `true` 两个状态、到达 `NotFound` 序列边界，并完成干净的 shutdown 与 unload。

Fixture active 期间，游戏内可见的车辆灯光会自动循环。保留的本地日志是：

```text
tmp/input-e2e-semantical-light-2026-07-26-final.log.txt
SHA-256: 0b83347bdbf015520edd1ef4646cf3db15f409d3bd1cccf1282051f79b1f55b1
```

这个结果证明了真实 semantical routing path，而不只是 device registration 或 callback
能够编译。同一次游戏 session 中的其他错误来自故意拒绝新版 Telemetry API 的 fallback
fixture 与已安装 mod；没有出现 Semantical Input runtime error、panic、deadlock 或 crash。

## 开发检查

```bash
cargo fmt --all -- --check
bash -n scripts/*.sh
scripts/check-plugin-boundary.sh examples/input-semantical-plugin
cargo test --locked -p scs-sdk-input-semantical-example
cargo clippy --locked -p scs-sdk-input-semantical-example --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --locked
```

涉及 artifact 或 installer 的修改还必须运行三个平台的 build script。

## 许可证

Workspace 自有 Rust 代码可由你选择使用 Apache License 2.0 或 MIT。SCS-derived
ABI 名称与契约仍受仓库根目录记录的原始 SCS SDK 声明约束。这个独立社区 fixture
与 SCS Software 不存在隶属或官方背书关系。
