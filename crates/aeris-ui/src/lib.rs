//! Aeris 的布局模型、设计令牌及可选的 Canopus LVGL 组件。

#![cfg_attr(not(test), no_std)]

#[cfg(all(feature = "devbox", target_os = "emscripten"))]
extern crate alloc;

pub mod apx;
pub mod assets;
pub mod capsule;
pub mod card;
#[cfg(any(
    test,
    feature = "band-10-pro",
    feature = "band-11",
    all(feature = "devbox", target_os = "emscripten")
))]
mod confirm_slider;
pub mod font;
pub mod geometry;
pub mod header;
pub mod layout;
pub mod motion;
pub mod nine_slice;
#[cfg(any(
    test,
    feature = "band-10-pro",
    feature = "band-11",
    all(feature = "devbox", target_os = "emscripten")
))]
mod numeric_wheel;
pub mod page_indicator;
pub mod progress_ring;
pub mod theme;
pub mod theme_slots;
pub mod typography;

pub use apx::{Apx, ApxScale};
pub use assets::{BitmapAsset, ALL as BITMAP_ASSETS};
pub use header::{
    HeaderAction, HeaderLayout, HeaderProps, HeaderStyle, HeaderVariant, BUTTON_SIZE,
    HEADER_HEIGHT, HEADER_WIDTH, ICON_SIZE, PADDING,
};
pub use page_indicator::{
    PageIndicatorLayout, PageIndicatorState, INDICATOR_ACTIVE_SIZE, INDICATOR_HEIGHT,
    INDICATOR_INACTIVE_SIZE, INDICATOR_WIDTH,
};
pub use theme::Theme;
pub use typography::TextSize;

#[cfg(any(
    all(any(feature = "band-10-pro", feature = "band-11"), not(target_arch = "wasm32")),
    all(feature = "devbox", target_os = "emscripten")
))]
pub mod device;

#[cfg(all(feature = "devbox", target_os = "emscripten"))]
pub mod devbox;

#[cfg(all(feature = "devbox", target_os = "emscripten"))]
extern crate self as canopus_target_private;
#[cfg(all(feature = "devbox", target_os = "emscripten"))]
pub use devbox::bindings::*;
