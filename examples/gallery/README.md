# Aeris UI Gallery

Gallery 是组件库内的独立示例应用。小米手环 10 Pro 和小米手环 11 共用 `src/gallery.rs`，由 Aeris UI 组件根据设备配置适配字号、标题栏、分页和滚动留白。

在 Aeris UI 根目录运行 `cargo aeris-devbox --demo`，或进入此目录运行 `cargo aeris-devbox`。DevBox 使用同一份 Rust 和 LVGL 应用源码，默认并排显示两种设备；修改源码会自动重新构建。

横向切换卡片、按钮和反馈展页；点击 **Inputs / Input Controls** 可进入滚轮及确认滑块。反馈展页包含对话框和结果浮层示例。交互状态仅用于本地演示。

准备好 Canopus CLI 后，在组件库根目录运行 `sh examples/gallery/build.sh` 生成并校验安装包。可通过 `CANOPUS_CLI`、`CANOPUS_ROOT`、`AERIS_PACKAGE_OUTPUT` 指定工具、框架和输出位置。目前原生安装包目标为小米手环 10 Pro `3.101.043`；手环 11 原生打包与真机验证见仓库待办。

生成的安装包与构建缓存位于本目录的 `dist/`、`target/`、`build/`，不纳入版本控制。字体许可见 `fonts/OFL.txt`。

## 主题

在应用根目录的 [aeris.config.toml](aeris.config.toml) 的 `[theme]` 中设置 `button_style` 为 `"glass"`（玻璃）或 `"flat"`（扁平）。`theme.colors.accent` 是强调色，Aeris UI 会用它推导主按钮的填充色和前景色。可选的 `theme.colors.background`、`theme.colors.primary_text` 和 `theme.colors.secondary_text` 分别覆盖背景、主文字和次要文字。色值使用 `#RRGGBB`，省略的色值沿用组件默认值。

主题配置在构建时嵌入应用。`[devbox]` 配置预览设备、构建特性、源码入口和监听目录。DevBox 会监听该文件，原生安装包需重新构建。
