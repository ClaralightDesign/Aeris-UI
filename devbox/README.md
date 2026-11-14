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

开发 DevBox 本身时，可用 `cargo install --locked --path crates/cargo-aeris-devbox` 安装本地启动器。

## 接入应用

在应用目录放置 `devbox.json`。路径相对于配置文件。Cargo 本地依赖的 Rust 源码自动监听，应用素材可通过 `watch` 补充。

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
