# Aeris DevBox

DevBox 从应用目录启动独立的浏览器调试器，从项目的正常启动入口运行应用，让你按实际页面路径操作，并检查当前画面的 Aeris UI 和 LVGL 对象。每个项目有自己的预览产物、监听范围和日志。浏览器预览使用手环 10 Pro 的 336 × 480 画布。

## 启动

DevBox 是可单独安装的 Cargo 子命令。从应用的目录执行，启动器会用 Cargo 解析到的 Aeris UI 版本，适用于 Git 依赖和本地路径依赖。网页运行文件和依赖安装在应用的 `target/aeris-devbox/` 下。

首次安装命令。选择包含 DevBox 的 Aeris UI 修订，替换下面的 `<rev>`。

```sh
cargo install --locked --git https://github.com/ClaralightDesign/Aeris-UI.git --rev <rev> cargo-aeris-devbox
```

准备 Node.js 20.19+ 或 22.12+、pnpm、Make、Rust 的 `wasm32-unknown-emscripten` 目标、Emscripten 4.0.20 和 LVGL 9.4.0 源码。激活 Emscripten 环境后，在应用目录运行：

```sh
rustup target add wasm32-unknown-emscripten
export LVGL_DIR=/path/to/lvgl
cargo aeris-devbox
```

打开终端显示的地址。可用 `cargo aeris-devbox configs/watch.json` 选择其他配置，或设置 `AERIS_DEVBOX_PORT` 选择端口。也可通过 `EMCC`、`EMAR` 指定已安装的 Emscripten 工具。

## 预览画布

100% 时，336 × 480 的设备画面显示为 168 × 240 CSS 像素。缩放菜单支持输入到 0.1%。触控板捏合可连续缩放，双指滑动可平移。用 ⌘/Ctrl 加 `+` 或 `-` 调整缩放，按 ⌘/Ctrl 加 `0` 恢复到 100% 并居中。

可直接拖动预览区空白处，也可按住空格或鼠标中键从画面上拖动。截图和录屏仍使用设备像素。

在「截图与录屏」中选择全屏或区域截图后，可以导出 PNG、WebP 或 JPG。帧率和鼠标指针设置适用于录屏。

## 接入应用

在应用目录放置 `devbox.json`。路径相对于配置文件。Cargo 本地依赖的 Rust 源码自动监听，应用素材可通过 `watch` 补充。

「源代码」面板从 `manifest` 对应的应用包目录开始，可以展开目录并打开多个文件标签。`source` 用于指定默认打开的项目文件，路径仍相对于 `devbox.json`。

浏览器标签页图标默认读取应用包内 `Canopus.toml` 的 `[native_app].icon`。也可以在 `devbox.json` 中用 `icon` 指定一张相对于配置文件的图片。没有可用图标时会显示内置占位图。

```json
{
  "title": "我的手表应用",
  "manifest": "Cargo.toml",
  "features": ["devbox"],
  "source": "src/lib.rs",
  "watch": ["assets"]
}
```

应用保留原有的 Aeris UI Git 依赖，将 `rev` 更新到支持 DevBox 的版本。在应用的 Cargo 特性表中加入：

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

保存源码或素材后会在构建成功时重启应用，保留浏览器本地存储。构建失败时可以继续操作上一成功版本，并在控制台查看错误。

预览工具栏的「设置」提供「重新构建」和「清理构建缓存」。两项操作都会排队构建，成功后立即刷新预览。清理操作会先移除当前项目的 DevBox Cargo 和 LVGL 编译缓存，期间服务继续运行，并保留上一次构建成功的画面。

## Agent 接入

安装支持 Agent 接入的 Cargo 启动器，并将应用的 Aeris UI 依赖更新到包含 Agent 接入的修订。在应用目录安装无头 Chromium，然后运行 DevBox。

```sh
cargo aeris-devbox agent install-browser
cargo aeris-devbox
```

在预览工具栏的「设置」中，可分别开启「Agent CLI」和「MCP」。普通工作台启动时两项均关闭，刷新页面保留当前开关，重启 DevBox 后恢复默认。两种接口共用一个独立的无头预览，应用本地存储在该预览内跨重载保留。关闭某个开关会立即撤销该接口的访问密钥，重新开启后请复制新的连接信息。

### MCP

打开「MCP」，点击「复制 MCP 配置」。将 `mcpServers.aeris-devbox` 中的本地 Streamable HTTP 地址和 `Authorization` 请求头填入支持该传输和自定义请求头的 MCP 客户端。`devbox_wait` 可等待构建及首帧，`devbox_snapshot` 返回结构化对象树，`devbox_screenshot` 直接返回 PNG 图片。客户端还可发现状态、坐标点击和滑动、返回、切换场景、重载及日志工具。MCP 仅监听本机，关闭工作台服务后连接失效。

### CLI

开启「Agent CLI」后，点击「复制会话路径」。无人值守时也可直接启动启用 CLI 的前台会话：

```sh
cargo aeris-devbox agent serve
```

`install-browser` 和 `serve` 均接受可选的 `devbox.json` 路径。前台服务输出一行 JSON，其中的 `session` 是会话描述文件的绝对路径。描述文件包含访问令牌，供当前用户的终端和 Agent 使用。按 Ctrl+C 关闭会话并清理其临时构建产物和浏览器状态。

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
- 命令向 stdout 输出一条带 `version: 1` 的 JSON。失败时以非零退出码和 `error.code` 报告原因。截图路径已存在时返回 `OUTPUT_EXISTS`。运行 `cargo aeris-devbox agent --help` 可查看完整用法。

## DevBox 内部开发

开发启动器时，可在 Aeris UI 仓库根目录运行 `cargo install --locked --path crates/cargo-aeris-devbox`。

DemoMode 用于开发和回归检查 DevBox 自身的界面。它用 Canvas 倒计时提供可交互画面，便于检查对象树、运行日志、截图和录屏。在 Aeris UI 仓库根目录启动时只需要 Node.js 和 pnpm。

```sh
pnpm --dir devbox install --frozen-lockfile
pnpm --dir devbox demo
```

打开终端显示的地址后，修改 `devbox/web/demo.ts` 可验证「保存后自动刷新」。

也可以在 DemoMode 中验证 Agent CLI。安装上面的 Cargo 启动器后，从仓库根目录运行以下命令，再用上面的 `agent` 函数连接。倒计时的开始／暂停按钮位于 `168,380`，重置位于 `168,430`。

```sh
node devbox/cli.mjs agent install-browser
node devbox/cli.mjs agent serve --demo
```

安装 Chromium 后，可用 `node --test devbox/agent.test.mjs` 验证 DemoMode 的 CLI、工作台开关和 MCP 接入。
