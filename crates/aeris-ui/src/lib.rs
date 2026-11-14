//! Aeris 的布局模型、设计令牌及可选的 Canopus LVGL 组件。

#![cfg_attr(not(test), no_std)]

pub mod assets;
pub mod capsule;
pub mod card;
#[cfg(any(
    test,
    feature = "band-10-pro",
    all(feature = "devbox", target_os = "emscripten")
))]
mod confirm_slider;
pub mod font;
pub mod geometry;
pub mod header;
pub mod motion;
pub mod nine_slice;
#[cfg(any(
    test,
    feature = "band-10-pro",
    all(feature = "devbox", target_os = "emscripten")
))]
mod numeric_wheel;
pub mod page_indicator;
pub mod progress_ring;
pub mod theme;
pub mod theme_slots;

pub use assets::{BitmapAsset, ALL as BITMAP_ASSETS};
pub use header::{
    HeaderAction, HeaderLayout, HeaderProps, HeaderVariant, BUTTON_SIZE, HEADER_HEIGHT,
    HEADER_WIDTH, ICON_SIZE, PADDING,
};
pub use page_indicator::{
    PageIndicatorLayout, PageIndicatorState, INDICATOR_ACTIVE_SIZE, INDICATOR_HEIGHT,
    INDICATOR_INACTIVE_SIZE, INDICATOR_WIDTH,
};
pub use theme::Theme;

#[cfg(any(
    all(feature = "band-10-pro", not(target_arch = "wasm32")),
    all(feature = "devbox", target_os = "emscripten")
))]
pub mod device;

#[cfg(all(feature = "devbox", target_os = "emscripten"))]
pub mod devbox;

#[cfg(all(feature = "devbox", target_os = "emscripten"))]
extern crate self as canopus_target_private;
#[cfg(all(feature = "devbox", target_os = "emscripten"))]
pub use devbox::bindings::*;
