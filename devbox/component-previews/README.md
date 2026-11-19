# Aeris UI 文档样式预览

这些 DevBox 场景调用 Aeris UI 的组件入口，为 AstroBox 插件文档中的样式标签提供交互预览。独立页面接收 `?scene=组件:样式[:设置…]`，例如 `?scene=header:large-title`。场景在 `src/lib.rs` 注册，文档站的 `src/components/aeris/preview.tsx` 负责选择场景。

按 [DevBox 准备说明](../README.md#启动) 配置 Emscripten、Rust 浏览器目标和 LVGL 后，从仓库根目录重新导出：

```sh
LVGL_DIR=/path/to/lvgl sh devbox/component-previews/export.sh \
  ../AstroBox-NG-Plugin-Docs/public/assets/canopus/ui-gallery/component-previews.html
```

若已有由 `devbox/native/Makefile` 构建的 `liblvgl.a`，可将它的绝对路径传给 `AERIS_PREVIEW_LVGL_LIB`，跳过 LVGL 的重复编译。

布局示例可通过 `?scene=layout:settings` 打开。点击按钮更新设置页的选中项，向上拖动查看说明。用 `?scene=layout:empty` 检查没有子节点的 Box、Flex 和 Grid。嵌入 DevBox 检查器时加上 `&debug` 即可查看组件名、原始工具类和渲染后的矩形。

从 Aeris UI 根目录运行 `node devbox/cli.mjs devbox/component-previews/aeris.config.toml`，可以在同一个工作台并排查看四种屏幕。独立导出的 HTML 也可以通过 `device` 查询参数选取设备。
