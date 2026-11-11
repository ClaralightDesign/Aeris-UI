# Rolling Digits

比例数字滚动组件。支持固定长度的 ASCII 数字和冒号。字宽变化时，整行的字形位置与纵向滚动同步过渡。组件适用于 `no_std`，无依赖。

## 接入

在调用项目的 `Cargo.toml` 中加入 Cargo 路径依赖，路径指向本目录。

```toml
[dependencies]
rolling-digits = { path = "../Aeris-UI/crates/rolling-digits" }
```

```rust
use rolling_digits::{Direction, Metrics, RollingDigits};

// 使用实际字体的字宽，顺序为 0 到 9。
let metrics = Metrics {
    digits: [18, 12, 17, 17, 18, 17, 18, 16, 18, 18],
    colon: 8,
};
let mut time = RollingDigits::new(*b"0:05:30", metrics, 240, 14).unwrap();
time.set(*b"0:05:29", Direction::Down, 1000);
let frame = time.sample(1033);
assert!(frame.outgoing[6].is_some());
```

渲染端复用每个位置的新旧标签，按帧设置 `center_x`、`y` 和 `opacity`。帧坐标相对于文本区域左侧，`width` 是当帧总宽。右对齐时可用容器宽度减去 `width` 作为整行的起始偏移。保留上一次帧数据即可跳过相同的属性更新。

用单调毫秒时钟调用 `set` 和 `sample`。`is_animating()` 为真时继续采样，变为假后停止帧刷新。快速更新时只保留一个待播放目标。`finish()` 可立即显示最新值。

[Aeris UI 的 LVGL 实现](../aeris-ui/src/device/rolling_number.rs)
