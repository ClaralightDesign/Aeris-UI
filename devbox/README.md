# Aeris DevBox

DevBox 从应用目录启动独立的浏览器调试器，从项目的正常启动入口运行应用，让你按实际页面路径操作，并检查当前画面的 Aeris UI 和 LVGL 对象。默认使用手环 10 Pro 的 336 × 480 画布。

## 启动

DevBox 是可单独安装的 Cargo 子命令。从应用的目录执行，启动器会用 Cargo 解析到的 Aeris UI 版本，适用于 Git 依赖和本地路径依赖。网页运行文件和依赖安装在应用的 `target/aeris-devbox/` 下。

首次安装时执行以下命令。

```sh
cargo install --locked --git https://github.com/ClaralightDesign/Aeris-UI.git cargo-aeris-devbox
```

准备 Node.js 20.19+ 或 22.12+、pnpm、Make、Rust 的 `wasm32-unknown-emscripten` 目标、Emscripten 4.0.20 和 LVGL 9.4.0 源码。激活 Emscripten 环境后，在应用目录运行：

```sh
rustup target add wasm32-unknown-emscripten
export LVGL_DIR=/path/to/lvgl
cargo aeris-devbox
```

打开终端显示的地址。可用 `cargo aeris-devbox configs/watch.toml` 选择其他配置，或设置 `AERIS_DEVBOX_PORT` 选择端口。也可通过 `EMCC`、`EMAR` 指定已安装的 Emscripten 工具。

## 预览画布

WASM 画布的中文使用小米 MiSans 字体。DevBox 在首帧前从小米 CDN 加载字体，每个字重加载失败或超时后使用系统字体。

100% 时，336 × 480 的设备画面显示为 168 × 240 CSS 像素。鼠标位于设备画面内时，滚轮和触控板滚动会作用于画面内的控件；触控板双指从左向右滑动会触发应用返回（与 Escape 相同）。在画面外的空白处滚动可平移外层画布。

缩放菜单支持输入到 0.1%。触控板捏合可连续缩放。用 ⌘/Ctrl 加 `+` 或 `-` 调整缩放，按 ⌘/Ctrl 加 `0` 恢复到 100% 并居中。

可直接拖动预览区空白处，也可按住空格或鼠标中键从画面上拖动。截图和录屏仍使用设备像素。

按住 Option（macOS）或 Alt，在预览区拖动框选。可以从设备外的空白处起拖，也可以用同一个框选中多台设备。框内显示元素轮廓、尺寸和可读取的圆角半径，标签会随框边裁切。标注使用设备像素，松开按键即可返回普通预览。

在「截图与录屏」中选择全屏或区域截图后，可以导出 PNG、WebP 或 JPG。帧率和鼠标指针设置适用于录屏。

文字的字号、apx 来源、实际渲染字号、字体、字重、颜色和行高同时显示在元素侧边栏与画布选择器中。实际渲染字号包含对象及父对象的缩放，滚动收缩的标题会连续更新。字号与行高分别显示；选择器 tooltip 隐藏数值为零的尺寸、边距和不可见背景，黑色文字仍会正常显示。

## 接入应用

在应用目录放置 `aeris.config.toml`。路径相对于配置文件。Cargo 本地依赖的 Rust 源码自动监听，应用素材可通过 `watch` 补充。

在 `[devbox]` 下添加 `devices = ["xiaomi-band-10-pro-3.101.043", "xiaomi-band-11-4.100.155", "round-390", "square-480"]` 可并排预览四种屏幕。第一项是默认设备。点击设备或上方名称即可切换调试工具的作用对象。每个实例有独立的页面、场景和本地存储。应用可用 `aeris_ui::device::display_profile().viewport` 获取当前画布和安全区。

「源代码」面板从 `manifest` 对应的应用包目录开始，可以展开目录并打开多个文件标签。`source` 用于指定默认打开的项目文件，路径仍相对于 `aeris.config.toml`。

浏览器标签页图标默认读取应用包内 `Canopus.toml` 的 `[native_app].icon`。也可以在 `aeris.config.toml` 中用 `icon` 指定一张相对于配置文件的图片。没有可用图标时会显示内置占位图。

```toml
[devbox]
title = "我的手表应用"
manifest = "Cargo.toml"
features = ["devbox"]
hot_reload = true
auto_refresh = true
agent_cli = false
mcp = false
source = "src/lib.rs"
watch = ["assets"]
```

主题可写在同一文件的 `[theme]` 和 `[theme.colors]` 下，预览场景使用 `[[devbox.scenes]]`。DevBox 仍支持已有的 `devbox.json` 项目。

确认应用使用的 Aeris UI 支持 DevBox。在应用的 Cargo 特性表中加入：

```toml
[features]
devbox = ["aeris-ui/devbox"]
```

应用的 Cargo 库声明 `crate-type = ["staticlib"]` 并提供浏览器构建 feature。这个 feature 启用 `aeris-ui/devbox`，使用与设备版共享的 Rust 页面、状态和导航逻辑。把固件专属依赖限定在设备目标上，在 Emscripten 目标为应用实际使用的固件服务提供浏览器适配。

```rust
#[cfg(all(feature = "devbox", target_os = "emscripten"))]
aeris_ui::devbox_app!(browser::start);
```

`browser::start` 的签名是 `fn(aeris_ui::devbox::Obj) -> bool`。它接收 LVGL 根对象，在 UI 线程初始化应用并打开首页。若要处理预览中的 Escape，可额外导出 `extern "C" fn aeris_devbox_back()`，转交应用的返回逻辑。

轻翎计时器的接入复用了既有页面生命周期，并为页面栈、时钟和本地存储提供浏览器适配。设备振动、系统通知和 AstroBox 激活仍依赖实机。应用也可以用 `scenes` 和 `devbox_scenes!` 保留独立的组件测试入口。

工作台默认启用 `hot_reload = true`。构建成功后，DevBox 在后台启动新版并重放本次会话的操作，确认页面、滚动位置和支持的控件状态已恢复后再切换。恢复或构建失败时可以继续操作旧版。恢复失败后可选择「从首页启动新版」。应用本地存储会跨重载保留。

`hot_reload = false` 时，源码或素材改动仍会自动构建。工作台等到你点击「重载」才从首页加载新版。`auto_refresh = false` 只暂停保存后的自动切换，仍可手动热重载。工具栏「设置」可临时切换这两项。临时开关在同一次 DevBox 服务运行期间有效，刷新浏览器后仍有效，重启服务后恢复项目配置。

「重新构建」和「清理构建缓存」都会排队构建。清理操作会先移除当前项目的 DevBox Cargo 和 LVGL 编译缓存，期间可以继续使用当前预览。浏览器整页刷新会重新启动预览应用，Agent 保持自己的重载行为。

图层面板自动识别 DevBox 背景快照生成的模糊，包括对话框和 ConfirmationOverlay。自定义组件使用自己的模糊绘制逻辑时，可在 DevBox 构建中调用 `aeris_ui::devbox::set_backdrop_blur(object, radius_px)`，报告该对象的实际模糊半径（设备像素）。传入 0 可清除标记。

## Agent 接入

安装支持 Agent 接入的 Cargo 启动器，并将应用的 Aeris UI 依赖更新到包含 Agent 接入的修订。在应用目录安装无头 Chromium，然后运行 DevBox。

```sh
cargo aeris-devbox agent install-browser
cargo aeris-devbox
```

用 `agent_cli` 和 `mcp` 设置两种接口的项目默认值，省略时均为关闭。预览工具栏的「设置」可以临时覆盖，两种接口共用一个独立的无头预览，应用本地存储在该预览内跨重载保留。关闭某个开关会立即撤销该接口的访问密钥，重新开启后请复制新的连接信息。

### MCP

打开「MCP」，点击「复制 MCP 配置」。将 `mcpServers.aeris-devbox` 中的本地 Streamable HTTP 地址和 `Authorization` 请求头填入支持该传输和自定义请求头的 MCP 客户端。`devbox_wait` 可等待构建及首帧，`devbox_snapshot` 返回结构化对象树，`devbox_screenshot` 直接返回 PNG 图片。客户端还可发现状态、坐标点击和滑动、返回、切换场景、重载及日志工具。MCP 仅监听本机，关闭工作台服务后连接失效。

### CLI

开启「Agent CLI」后，点击「复制会话路径」。无人值守时也可直接启动启用 CLI 的前台会话：

```sh
cargo aeris-devbox agent serve
```

`install-browser` 和 `serve` 均接受可选的 `aeris.config.toml` 路径。前台服务输出一行 JSON，其中的 `session` 是会话描述文件的绝对路径。描述文件包含访问令牌，供当前用户的终端和 Agent 使用。按 Ctrl+C 关闭会话并清理其临时构建产物和浏览器状态。

在另一个终端中设置 `SESSION` 为上述路径。以下示例需要 `jq`，用于读取 JSON 中的编号。

```sh
agent() { cargo aeris-devbox agent "$@" --session "$SESSION"; }
agent wait
BUILD_ID=$(agent status | jq -r '.build.buildId')
# 修改并保存应用代码，然后等待更新的构建和首帧。
agent wait --after "$BUILD_ID"
agent snapshot > /tmp/devbox-tree.json
PREVIEW_ID=$(jq -r '.previewId' /tmp/devbox-tree.json)
# 根据对象树中的可见控件边界替换示例坐标。
agent tap 82 220 --preview "$PREVIEW_ID"
agent snapshot
agent screenshot --out /tmp/devbox-after.png
agent logs --source runtime
```

- `snapshot` 返回画布尺寸和 LVGL 对象树。使用节点的 `x`、`y`、`width`、`height` 选择可见区域内的点击位置。坐标从设备画布左上角的 `0,0` 开始。节点 ID 仅供本次观察使用。
- `swipe <x1> <y1> <x2> <y2> --preview <id>` 模拟滑动，`back --preview <id>` 发送 Escape。可以用 `scene <id>` 切换已配置场景，或用 `reload` 重启当前场景。预览重建后请重新获取 `previewId` 和坐标。
- `wait` 默认等待 5 分钟，可通过 `--timeout <毫秒>` 调整。最新构建失败时返回 `BUILD_FAILED`。用 `logs --source build` 查看与失败 `buildId` 对应的日志。保存修复后可用 `wait --after <失败的 buildId>` 等待下一版。
- `logs --since <cursor>` 从上一次返回的游标继续读取。日志保留最近 500 条，游标早于保留范围时返回 `truncated: true`。运行日志带有 `previewId`。
- `status` 列出设备 ID。CLI 用 `--device <id>`，MCP 用 `device` 参数指定设备。省略时使用第一台。输入命令的 `previewId` 必须与设备匹配。
- 命令向 stdout 输出一条带 `version: 1` 的 JSON。失败时以非零退出码和 `error.code` 报告原因。截图路径已存在时返回 `OUTPUT_EXISTS`。运行 `cargo aeris-devbox agent --help` 可查看完整用法。

## DevBox 内部开发

开发启动器时，可在 Aeris UI 仓库根目录运行 `cargo install --locked --path crates/cargo-aeris-devbox`。

DemoMode 用 DevBox 的普通 Rust 和 LVGL 构建流程运行真实的 Aeris UI Gallery。示例源码位于库内的 `examples/gallery`，也可通过 `AERIS_GALLERY_DEVBOX_CONFIG` 指定其他 Gallery 配置。按前文准备好 Node.js、pnpm、Rust 浏览器目标、Emscripten 和 LVGL。

```sh
pnpm --dir devbox install --frozen-lockfile
pnpm --dir devbox demo
```

已安装 Cargo 启动器时，也可以运行 `cargo aeris-devbox --demo`。

打开终端显示的地址，DemoMode 默认并排显示小米手环 10 Pro 和小米手环 11。修改 Gallery 的 `examples/gallery/src/gallery.rs` 可验证「保存后自动刷新」，并在工作台的「源代码」面板查看同一份文件。

也可以在 DemoMode 中验证 Agent CLI。安装上面的 Cargo 启动器后，从仓库根目录运行以下命令，再用上面的 `agent` 函数连接。可在对象树查找 `Input Controls` 卡片的位置并点击，再用 `back` 返回。

```sh
node devbox/cli.mjs agent install-browser
node devbox/cli.mjs agent serve --demo
```

安装 Chromium 后，可用 `node --test devbox/agent.test.mjs` 验证 DemoMode 的 CLI、工作台开关和 MCP 接入。

`node --test devbox/reload.test.mjs devbox/reload-workbench.test.mjs` 验证操作重放、存储隔离和浏览器工作台热更新。浏览器测试在临时目录构建 Gallery。
