# `scs-sdk-sys`

**中文** | [English](README.md)

`scs-sdk-sys` 是面向 64 位游戏进程、无依赖、`no_std`、手写实现的
**SCS Telemetry SDK 1.14** 公共原始 ABI 层。它镜像 Euro Truck Simulator 2
与 American Truck Simulator 使用的 C 声明，不负责安全 wrapper 策略、插件生命周期
或产品逻辑。

权威来源是仓库内保留的官方 header 分发：
[`third-party/scs_sdk_1_14/`](../../third-party/scs_sdk_1_14/)。Rust 定义会逐项
对照这些文件审计布局、数值、函数签名、调用约定与以 NUL 结尾的字节常量。

> 本 crate 覆盖 SDK 1.14 的公共 **telemetry** 接口。SDK 中的 input-device API
> 目前尚未由本 workspace 实现。

## 设计约束

- **纯 Rust：** 编译不需要 C/C++ shim、CMake、生成式 binding、bindgen 或 Clang。
- **无依赖：** crate 只使用 `core`，没有第三方 Rust 依赖。
- **`no_std`：** 原始 binding 不依赖操作系统运行时或分配支持。
- **x86-64 ABI 契约：** 非 64 位指针目标会触发编译错误。支持的插件产物目标是
  Windows x86-64、Linux x86-64 与 macOS x86-64。
- **保持 header 形状：** crate 原样保留 C 字段顺序、符号性、宽度、常量和
  catalog 顺序，而不是在这一层追求易用性。

## 模块

| 模块 | 原始层职责 |
| --- | --- |
| `constants` | SDK result code、版本常量、日志级别、flag 与共享 ABI 标量类型。 |
| `value` | `repr(C)` tagged value、向量、欧拉角、placement、named value 与 value union。 |
| `telemetry` | 初始化结构、函数指针、callback 类型、事件 payload 布局与插件入口类型。 |
| `channels` | common、truck、trailer、job channel 字节字符串常量与有序 catalog。 |
| `configuration` | Configuration ID 与 attribute 常量。 |
| `gameplay` | Gameplay event ID 与 attribute 常量。 |
| `games` | 官方 ETS2、ATS 标识符与版本常量。 |

SDK 1.14 telemetry 原始清单为：

| 接口面 | 数量 |
| --- | ---: |
| Channels | 107 |
| Configuration IDs | 6 |
| Configuration attributes | 60 |
| Gameplay event IDs | 6 |
| Gameplay attributes | 15 |

原始 `ALL` catalog 保持 header 顺序与原始元数据。上层会增加 typed descriptor 与
association catalog；本 crate 不根据产品行为推导这些策略。

## ABI 与 unsafe 边界

公开 foreign layout 使用 `#[repr(C)]`，callback 与 SDK 函数指针使用
`extern "system"`，从而在各平台选择正确调用约定。跨越游戏/插件边界的结构由
编译期 size、alignment 与 offset assertion 保护。

部分原始声明刻意保持“不好用”的真实形态：

- tagged value 内含 C union，因此读取成员在本层仍是 `unsafe`；
- SCS 没有义务初始化的 padding 使用 `MaybeUninit<u32>`，而不是被读取、比较或格式化；
- foreign string 与 catalog 名称继续使用以 NUL 结尾的原始字节字符串；
- 可选函数指针与 opaque callback context 保留 foreign pointer 表示。

这些选择是在保持真实 ABI，而不是凭空制造有效性保证。tag 校验、受生命周期约束的
借用、typed value 解码与易用错误类型属于 [`scs-sdk`](../scs-sdk/)。

## 适用对象

此 crate 主要供审计或扩展 typed wrapper 和 runtime 的开发者使用。普通插件 crate
应依赖 [`scs-sdk-plugin`](../scs-sdk-plugin/)，并使用它的 `sdk` re-export。
应用插件不应自行读取 union、处理原始指针、导入 raw SCS 类型或声明 loader 入口。

## 验证

在仓库根目录运行：

```bash
cargo fmt --all -- --check
cargo test --locked -p scs-sdk-sys
cargo clippy --locked -p scs-sdk-sys --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk-sys --no-deps
```

修改 ABI layout、padding、union、catalog 或版本常量时，还必须验证安全解释层：

```bash
cargo test --locked -p scs-sdk
cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
```

若修改 exported callback 或初始化布局，还需要运行适用的 Windows、Linux、macOS
产物构建与 export 验证脚本。

## 许可证

Workspace 自有 Rust 代码可由你选择使用
[Apache License 2.0](LICENSE-APACHE) 或 [MIT](LICENSE-MIT)。

[`third-party/scs_sdk_1_14/`](../../third-party/scs_sdk_1_14/) 中的官方文件仍是
SCS Software 的第三方材料，受 SDK 随附的独立许可约束。将它们保留在本仓库内不代表
这些材料被重新许可为 workspace license。
