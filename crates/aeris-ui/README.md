# Aeris UI

Aeris UI 是面向 Canopus 的 Rust 组件库。默认构建提供布局、字形和动效模型。启用 `band-10-pro` 可使用小米手环 10 Pro 固件 `3.101.043` 的 LVGL 组件。`gallery` 启用 Gallery 配色和 Linoval Sans 标签。依赖声明和初始化见[接入说明](../../README.md#接入)。

| 组件 | 设备端入口 | 资源与使用方 |
| --- | --- | --- |
| 页面容器、标题栏、分页 | `page`、`header`、`page_indicator` | 标题栏位图与分页指示器，Timer 和 Gallery |
| 胶囊按钮、卡片、开关、单选、步进器 | `page::button_in`、`cards`、`choice_list::ChoiceList` | 胶囊切片、卡片面与状态图，Timer 和 Gallery |
| 复选框 | `checkbox::Checkbox` | 卡片式复选，支持未选、部分选中和已选，Gallery 提供示例 |
| 菜单与弹窗 | `card_menu::CardMenu`、`dialog` | 菜单支持卡片定位；弹窗使用胶囊按钮 |
| 进度环 | `progress_ring::Ring` | 分段圆环，通过 `set_color` 设置进度颜色 |
| 横向进度条 | `progress_bar::ProgressBar` | 自定义底轨与填充颜色，与进度环共用 0–256 的进度范围 |
| 滚动数字 | `rolling_number::RollingNumber` | 字形图集，Timer 和 Gallery |
| 计时卡 | `countdown_card::CountdownCard` | 操作图标、进度环与字形，应用传入显示状态和操作回调 |
| 时长选项 | `option_grid::grid` | 支持数字或文字主标题、自定义副标题与点击参数 |
| 数字滚轮与确认滑块 | `numeric_wheel::NumericWheel`、`confirm_slider::Slider` | 拨轮遮罩、滑轨及调用方指定的滑块素材，Timer 和 Gallery |
| 分级滑块 | `stepped_slider::SteppedSlider` | 复用卡片底图和加减图标，支持逐档点按及轨道拖动，Gallery 提供示例 |

备注键盘（`keyboard::Keyboard`）已弃用。

组件在 UI 线程创建。调用方在页面销毁时清理组件句柄，在事件回调中转交事件。滚轮支持 1–3 列，范围为 `0..=99` 内的闭区间，每列有独立标签和初值。滑块在松手时报告确认结果，实际操作由应用决定。

`geometry::Viewport` 接收屏幕边界和矩形安全区，`content()` 返回相对页面根对象的区域。分页、按钮、滚轮和滑块的布局入口使用调用方提供的宽度或矩形。当前默认是 `Viewport::BAND_10_PRO`。后续圆屏、胶囊屏可由设备配置提供安全区，大方屏可提供更宽的区域。标题栏位图、详情与完成面板、原生弹窗模糊仍按 `336 × 480` 设计。

资源位于 `assets/`，资源表在 `src/device/assets.rs` 和 `src/device/font_assets.rs`。字体及许可在 `fonts/`。
