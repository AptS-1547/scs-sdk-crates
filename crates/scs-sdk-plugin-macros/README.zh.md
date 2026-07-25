# `scs-sdk-plugin-macros`

**中文** | [English](README.md)

`scs-sdk-plugin-macros` 是刻意保持狭窄的 procedural-macro 层，用于将 safe Rust
插件导出到 SCS telemetry loader ABI。它只负责一个 macro：

```rust
scs_sdk_plugin::export_plugin!(Plugin::default());
```

大多数应用应通过 [`scs-sdk-plugin`](../scs-sdk-plugin/) 的 re-export 使用它，而不是
直接依赖此 proc-macro crate。

## Expansion 契约

`export_plugin!` 将输入解析为恰好一个普通 Rust expression。它可以是 constructor、
struct literal，或任何结果实现了 `TelemetryPlugin` 的表达式。

每次初始化尝试通过 framework 的 ABI、pointer、version 与 lifecycle 验证后，该表达式
只会求值一次。生成的 factory 会把结果强制转换为 framework plugin trait object，
因此缺少 `TelemetryPlugin` 实现会在编译期失败，而不是拖到游戏 ABI 边界才出事。

Expansion 会创建：

- 一个 process-lifetime `Runtime` static，在动态库加载期间地址保持稳定；
- `scs_telemetry_init`；
- `scs_telemetry_shutdown`。

这正好是 SCS 要求的两个 loader-visible export。生成的 entry point 保留
`extern "system"`、raw ABI parameter 与 result 类型，以及固定符号名。原始指针、
unsafe 调用、symbol attribute 和 ABI 文档生成在 framework 边界内；手写应用源码仍是
普通 safe Rust。

每个插件 `cdylib` 只调用一次此 macro。第二次调用会再次尝试定义同一组固定 runtime
与 loader symbol。

## 依赖卫生

生成代码通过 consumer 的直接 `scs-sdk-plugin` 依赖解析 absolute path，不依赖调用方
源码中的 import、type alias 或局部实现细节。

此 proc-macro crate 不会反向依赖 `scs-sdk-plugin`，因为那会形成 Cargo dependency
cycle：

```text
scs-sdk-plugin -> scs-sdk-plugin-macros
       ^                    |
       +--------------------+  forbidden cycle
```

生成 token 改为引用公共 framework 契约与其文档化的 macro-hygiene path。runtime 行为
仍由 `scs-sdk-plugin` 实现并接受审计。

proc-macro 进程自身只有标准语法/quote 工具链：

```text
proc-macro2
quote
syn
```

它不包含 SDK adapter、平台 linker、全局插件状态或产品依赖。

## Macro 不负责什么

Macro 不解析 Telemetry API version，不验证初始化 pointer，不决定兼容性，不注册
subscription，不 dispatch callback，不执行 rollback，不解释 SDK value，也不实现产品
行为。它生成固定 ABI surface，然后把所有生命周期决策委托给 framework `Runtime`。

保持此 crate 足够小很重要：生成 ABI token 的任何变化都会影响全部 consumer，即使它们
的应用源码一个字都没改。

## 独立 consumer fixture

Ignored proc-macro doctest 不足以作为证明。仓库在
[`../scs-sdk-plugin/tests/fixtures/export-plugin`](../scs-sdk-plugin/tests/fixtures/export-plugin/)
维护独立 consumer workspace：

- pass fixture 只依赖公共 `scs-sdk-plugin` crate，在 safe source 中实现
  `TelemetryPlugin`，并把 `export_plugin!(Plugin::default())` 编译为真实 `cdylib`；
- missing-trait fixture 使用同一公共边界，但刻意省略 trait implementation，必须专门
  因缺少 `TelemetryPlugin` bound 而以 Rust error `E0277` 失败。

Release fixture build 会检查最终 linked 与 stripped 动态库，而不是只看 macro 是否解析：

| 平台 | 产物验证 |
| --- | --- |
| Windows x86-64 | PE shared library 架构与两个 loader export。 |
| Linux x86-64 | ELF shared object 架构与两个 loader export。 |
| macOS x86-64 | Mach-O dynamic library 架构与两个 loader export。 |

这会跨真实公共 dependency boundary 捕获 path hygiene、trait bound、LTO、symbol
visibility、calling convention 与最终 link regression。

## 验证

在仓库根目录运行：

```bash
cargo fmt --all -- --check
cargo test --locked -p scs-sdk-plugin-macros
cargo clippy --locked -p scs-sdk-plugin-macros --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings \
  cargo doc --locked -p scs-sdk-plugin-macros --no-deps
scripts/check-plugin-macro-fixtures.sh
scripts/check-plugin-boundary.sh
```

修改 expansion 或 export 时还必须运行：

```bash
scripts/build-windows-plugin-macro-fixture.sh
scripts/build-linux-plugin-macro-fixture.sh
scripts/build-macos-plugin-macro-fixture.sh
```

每个构建脚本都会验证最终平台产物和两个 SCS export。

## 许可证

Workspace 自有 Rust 代码可由你选择使用
[Apache License 2.0](LICENSE-APACHE) 或 [MIT](LICENSE-MIT)。

[`third-party/scs_sdk_1_14/`](../../third-party/scs_sdk_1_14/) 中的官方 SDK 文件仍是
SCS Software 材料，受其独立分发的许可证约束。
