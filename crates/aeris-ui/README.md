# Aeris UI

Aeris UI 是面向 Canopus 的 Rust 组件库。默认构建提供布局、字形和动效模型。启用 `band-10-pro` 可使用小米手环 10 Pro 固件 `3.101.043` 的 LVGL 组件。`gallery` 启用 Gallery 配色和 Linoval Sans 标签。依赖声明和初始化见[接入说明](../../README.md#接入)。

应用的主题配置放在应用根目录，与 `Cargo.toml` 和 `Canopus.toml` 同级。可参考 Gallery 的 [aeris.config.toml](../../examples/gallery/aeris.config.toml) 和 [build.rs](../../examples/gallery/build.rs)。应用构建时校验配置，启动时在创建组件之前调用 `aeris_ui::theme::apply`。调用 `page::button_with_theme` 或设置 `round_button::Options::theme` 可以为单个按钮指定外观。

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

新版组件可使用 `HeaderBar`、`ButtonOptions`、`CardOptions`、`TextBlockOptions` 和 `CircularProgressIndicator`。大标题通过 `HeaderBar::update_scroll` 跟随页面滚动。卡片副标题、状态卡尾部指示图标、文本块标题与说明、进度底轨可独立控制。已标记弃用的入口保留原有行为，Rust 的弃用提示包含替代接口。

备注键盘（`keyboard::Keyboard`）已弃用。

组件在 UI 线程创建。调用方在页面销毁时清理组件句柄，在事件回调中转交事件。滚轮支持 1–3 列，范围为 `0..=99` 内的闭区间，每列有独立标签和初值。滑块在松手时报告确认结果，实际操作由应用决定。

`device::display_profile()` 提供当前屏幕的 `Viewport`。`Viewport::content()` 返回相对页面根对象的矩形安全区，可交给 `layout::compute` 或宽度可变的组件，例如 `HeaderBar::new_in`、`cards::jump_card_in` 和 `option_grid::grid_in`。DevBox 可以在 10 Pro、手环 11、圆屏和方屏之间切换；DevBox 弹窗和结果浮层的背景模糊按当前画布尺寸生成。原生弹窗模糊目前仅支持 10 Pro。

Gallery 使用同一套页面源码，分页和滚动容器取 `Viewport::bounds()` 铺满屏幕。`HeaderBar::new_for_viewport` 的 `HeaderStyle::Navigation` 在手环 11 使用同一个标题随滚动从 7 apx 平滑缩至 4.5 apx，在 10 Pro 保留返回和大标题切换。`page::scroll_bars` 根据实际挂载的标题栏、分页栏设置上下留白，子组件从留白后的原点开始排布，页面不需要额外放置占位对象。也可用 `page::scroll_with_bars` 一次创建并配置滚动区域。

`Indicator::new_for_viewport` 自动创建渐变透明背景。10 Pro 的分页点更靠近屏幕底部；手环 11 使用弧形分页点，非强调点统一为 40% 透明度。滚动位置连续驱动分页状态，点击事件传递原始页码。

组件库默认文字使用 `TextSize` 的 apx 规则；`Font` 的历史名称表示基础字体资源。正文为 4 apx，在 10 Pro、手环 11、Redmi Watch 上分别显示为 25、22、30 px；标题、显示文字和大标题分别为 4.5、5、7 apx。用 `lv::text_apx` 或 `cards::text_apx` 指定逻辑字号，`lv::text_px` 和 `cards::text_font` 保留显式像素覆盖。`Apx::from_halves(9)` 表示 4.5 apx；组件通过 `Viewport::apx` 完成最终取整。

资源位于 `assets/`，资源表在 `src/device/assets.rs` 和 `src/device/font_assets.rs`。字体及许可在 `fonts/`。

## Box、Flex 与 Grid

`layout` 可以在默认构建中使用。用 `Box::children` 包裹内容，用 `Flex::row` 或 `Flex::column` 排列控件，用 `Grid::columns` 布置等宽列。`Leaf::new` 接收现有组件需要的数据。容器也可以用空切片创建，用于绘制底板或占据布局空间。

```rust
use aeris_ui::{
    layout::{Box as LayoutBox, ClassDef, Flex, Grid, Leaf},
    utilities,
};

const STYLES: &[ClassDef] = &[
    ClassDef::new("settings-card", utilities!("p-3 bg-[#242428] rounded-xl")),
];

let filters = [
    Leaf::new("Daily").class(utilities!("w-[120px]")),
    Leaf::new("Workout").class(utilities!("w-[144px]")),
    Leaf::new("Sleep").class(utilities!("w-[120px]")),
];
let tiles = [
    Leaf::new("Display"),
    Leaf::new("Sound"),
    Leaf::new("Quick tip").class(utilities!("col-span-2")),
];
let children = [
    Flex::row(&filters).class(utilities!("w-full flex-wrap gap-2 items-center")),
    Grid::columns(2, &tiles).class(utilities!("w-full gap-2")),
];
let panel = LayoutBox::children(&children)
    .class(utilities!(STYLES; "settings-card w-full gap-y-3"));
```

`utilities!` 在编译期处理工具类。数字尺寸和间距按 4 个设备像素计，`p-3` 对应 12 px。`p-[18px]` 指定任意整数像素，`bg-[#242428]` 指定 RGB，`bg-accent` 引用 Aeris 主题。后写的工具类覆盖相同属性，`.style(StylePatch { .. })` 中的显式属性优先。横向 Flex 支持换行和行间对齐，纵向 Flex 支持单列伸缩，Grid 支持 1–8 列、跨列和自动行高。

实现 `Measure<T>` 来提供叶子的首选宽度和指定宽度下的高度。再调用 `layout::compute`，传入 `Viewport::content()` 和用 `LayoutSlot::EMPTY` 初始化的工作区。结果包含按声明顺序排列的父级相对矩形。启用设备功能后，可以在 UI 线程用 `device::layout::mount` 创建对象，叶子回调可直接调用 `page::button_with_options` 等入口。滚动内容使用自动高度，并挂在 `page::scroll_in` 创建的对象下。几何改变时，由应用重算并重建布局子树。

DevBox 的对象树会显示 `AerisUI.Box`、`AerisUI.Flex` 和 `AerisUI.Grid`。在「样式」页查看原始工具类，也可以用类名搜索节点。无尺寸的空容器可从对象树选择，有尺寸的空容器也可从画布选中。
