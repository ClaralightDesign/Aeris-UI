# Aeris UI

Aeris UI 是面向 Canopus 可穿戴设备的 Rust 组件库。库内包含布局、主题、位图资源、滚动数字和原生 LVGL 组件。设备组件支持小米手环 10 Pro 固件 `3.101.043`。

## 接入

在 Canopus 模块的 `Cargo.toml` 中添加依赖。以下路径假定模块与 Aeris UI 源码目录并列。

```toml
[dependencies]
aeris-ui = { path = "../Aeris-UI/crates/aeris-ui", features = ["band-10-pro"] }
canopus-abi = { git = "https://github.com/AstralSightStudios/Canopus.git", rev = "f196b200823f47780c73ae48c4c62da8fe5311df" }
```

模块直接引用 Canopus SDK 时，使用相同的 Git 来源和修订，确保 Cargo 共享 ABI 类型。仅使用布局或主题时，可省略 `band-10-pro`。

在模块准备回调中调用 `aeris_ui::device::set_context(ctx)`。在 UI 线程调用 `device::assets::ensure()` 解包位图，按需调用 `device::assets::ensure_fonts()` 准备字形。资源自动定位到当前模块的数据目录。横向进度条等直接由 LVGL 绘制的组件无需解包资源。

组件清单见 [aeris-ui](crates/aeris-ui/README.md)。比例数字滚动模型见 [rolling-digits](crates/rolling-digits/README.md)。

## DevBox

安装 DevBox 后，在应用目录运行 `cargo aeris-devbox`，支持直接使用 Cargo 拉取的 Git 依赖。项目通过 `aeris.config.toml` 和 Rust 启动入口运行完整应用，再检查当前页面的 UI。[接入与启动](devbox/README.md)

组件展厅位于 [examples/gallery](examples/gallery/README.md)。在本仓库运行 `cargo aeris-devbox --demo`，可并排预览 10 Pro 与手环 11；两种设备运行同一份页面源码。已完成和后续工作见 [待办](TODO.md)。

## 验证

```sh
cargo test --workspace --features aeris-ui/band-10-pro
```

## 许可证

Rust 源码使用 [AGPL-3.0-only](LICENSE)。Linoval Sans 字体使用 [SIL Open Font License 1.1](crates/aeris-ui/fonts/OFL.txt)。
