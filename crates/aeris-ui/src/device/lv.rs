//! 计时器使用的 LVGL 组件，所有操作均在 UI 线程执行。

use core::ffi::c_void;

use crate::{
    capsule::{CapsuleKind, CapsuleLayout},
    theme,
};
use canopus_target_private::*;

use super::{anim, assets};

pub type Obj = *mut c_void;

const ALIGN_TOP_LEFT: u32 = 1;
const CARD_WIDTH: i32 = 328;
pub(super) const CARD_HEIGHT: i32 = 98;
pub const CLICKABLE: u32 = 1 << 1;
pub const SCROLLABLE: u32 = 1 << 4;
pub const SCROLL_CHAIN: u32 = (1 << 8) | (1 << 9);
pub const GESTURE_BUBBLE: u32 = 1 << 15;
pub const OVERFLOW_VISIBLE: u32 = 1 << 20;

pub(super) const BG_COLOR: u32 = 28;
pub const BG_OPA: u32 = 29;
const RADIUS: u32 = 12;
const BORDER_WIDTH: u32 = 48;
const BORDER_OPA: u32 = 50;
const IMAGE_RECOLOR: u32 = 69;
const IMAGE_RECOLOR_OPA: u32 = 70;
const TEXT_COLOR: u32 = 88;
pub(super) const TEXT_OPA: u32 = 89;
pub const OPA: u32 = 95;
pub const TRANSLATE_X: u32 = 106;
pub const TRANSLATE_Y: u32 = 107;

#[derive(Copy, Clone)]
pub enum Font {
    Small24,
    Medium24,
    Medium28,
    Demibold32,
    #[cfg(feature = "gallery")]
    Rounded44,
}

impl Font {
    /// 固件字体样式的基础字号。
    pub const fn base_px(self) -> i32 {
        match self {
            Font::Demibold32 => 32,
            #[cfg(feature = "gallery")]
            Font::Rounded44 => 44,
            Font::Medium28 => 28,
            Font::Small24 | Font::Medium24 => 24,
        }
    }
}

pub const TRANSFORM_SCALE_X: u32 = 108;
pub const TRANSFORM_SCALE_Y: u32 = 109;
pub const TRANSFORM_PIVOT_X: u32 = 111;
pub const TRANSFORM_PIVOT_Y: u32 = 112;

/// 设置文字的显示字号。
pub fn text_px(object: Obj, width: i32, height: i32, align: u32, font: Font, px: i32) {
    if object.is_null() || px == font.base_px() {
        return;
    }
    #[cfg(feature = "gallery")]
    {
        let _ = align;
        let line_height = super::linoval::apply(object, font, px);
        if height < line_height {
            unsafe { lvx_object_set_size(object, width, line_height) };
        }
    }
    #[cfg(not(feature = "gallery"))]
    {
        let scale = (px * 256) / font.base_px();
        // 文字以对齐点为缩放原点。
        let pivot_x = match align {
            1 => 0,
            3 => width,
            _ => width / 2,
        };
        style(object, TRANSFORM_PIVOT_X, pivot_x as u32);
        style(object, TRANSFORM_PIVOT_Y, (height / 2) as u32);
        style(object, TRANSFORM_SCALE_X, scale as u32);
        style(object, TRANSFORM_SCALE_Y, scale as u32);
    }
}

pub fn style(object: Obj, property: u32, value: u32) {
    if !object.is_null() {
        unsafe { lvx_object_set_local_style_u32(object, property, value, 0) };
    }
}

pub unsafe fn background(parent: Obj, width: i32, height: i32, color: u32, radius: u32) -> Obj {
    let object = unsafe { lvx_object_create(parent) };
    if object.is_null() {
        return object;
    }
    unsafe {
        lvx_object_set_size(object, width, height);
        lvx_object_align(object, ALIGN_TOP_LEFT, 0, 0);
    }
    style(object, BG_COLOR, color);
    style(object, BG_OPA, 255);
    style(object, RADIUS, radius);
    style(object, BORDER_WIDTH, 0);
    style(object, BORDER_OPA, 0);
    object
}

pub unsafe fn image(parent: Obj, path: &[u8], x: i32, y: i32) -> Obj {
    let object = unsafe { lvx_image_create(parent) };
    if object.is_null() {
        return object;
    }
    unsafe {
        if !set_image_source(object, path) {
            lvx_object_delete(object);
            return core::ptr::null_mut();
        }
        lvx_object_align(object, ALIGN_TOP_LEFT, x, y);
    }
    object
}

/// 设置当前模块数据目录中的位图，也接受以 NUL 结尾的绝对路径。
///
/// # 安全要求
/// 在 UI 线程调用，且对象必须是有效的 LVGL 图像。
pub unsafe fn set_image_source(object: Obj, path: &[u8]) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    let mut buffer = [0; 160];
    #[cfg(not(target_arch = "wasm32"))]
    let Some(path) = super::image_path(path, &mut buffer) else {
        return false;
    };
    unsafe { lvx_image_set_src(object, path.as_ptr().cast()) };
    true
}

pub unsafe fn label(
    parent: Obj,
    text: &[u8],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: u32,
    font: Font,
) -> Obj {
    let object = unsafe { lvx_label_create(parent) };
    if object.is_null() {
        return object;
    }
    unsafe {
        #[cfg(not(feature = "gallery"))]
        lvx_label_set_text(object, text.as_ptr());
        lvx_object_set_size(object, width, height);
        lvx_object_align(object, ALIGN_TOP_LEFT, x, y);
        lvx_label_set_text_align_center(object);
        lvx_object_clear_flag(object, CLICKABLE);
        #[cfg(feature = "gallery")]
        {
            let line_height = super::linoval::apply(object, font, font.base_px());
            if height < line_height {
                lvx_object_set_size(object, width, line_height);
            }
        }
        #[cfg(not(feature = "gallery"))]
        {
            let style_ptr = match font {
                Font::Small24 => canopus_target_private::STYLE_MISANS_REGULAR_24,
                Font::Medium24 => generated_symbols::STYLE_MISANS_MEDIUM_24_ENTRY,
                Font::Medium28 => generated_symbols::STYLE_MISANS_MEDIUM_28_ENTRY,
                Font::Demibold32 => STYLE_MISANS_DEMIBOLD_32,
            };
            lvx_style_apply(object, style_ptr as *const c_void, 255, 0);
        }
        #[cfg(feature = "gallery")]
        lvx_label_set_text(object, text.as_ptr());
    }
    style(object, BG_OPA, 0);
    style(object, BORDER_OPA, 0);
    style(object, TEXT_COLOR, color);
    style(object, TEXT_OPA, 255);
    object
}

pub unsafe fn label_right(
    parent: Obj,
    text: &[u8],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: u32,
) -> Obj {
    let object = unsafe { label(parent, text, x, y, width, height, color, Font::Small24) };
    style(object, 94, 3); // LV_TEXT_ALIGN_RIGHT
    object
}

pub unsafe fn dot(parent: Obj, x: i32, y: i32, size: i32, opacity: u32) -> Obj {
    let object = unsafe { lvx_object_create(parent) };
    if object.is_null() {
        return object;
    }
    unsafe {
        lvx_object_set_size(object, size, size);
        lvx_object_align(object, ALIGN_TOP_LEFT, x, y);
    }
    style(object, BG_COLOR, 0xFFFFFF);
    style(object, BG_OPA, opacity);
    style(object, RADIUS, (size / 2) as u32);
    style(object, BORDER_WIDTH, 0);
    style(object, BORDER_OPA, 0);
    style(object, OPA, 255);
    object
}

pub unsafe fn pager(parent: Obj, width: i32, height: i32) -> Obj {
    let object = unsafe { lvx_object_create(parent) };
    if object.is_null() {
        return object;
    }
    unsafe {
        lvx_object_set_size(object, width, height);
        lvx_object_align(object, ALIGN_TOP_LEFT, 0, 0);
        lvx_object_add_flag(object, 1 << 7); // 每次仅滚动一页
        lvx_object_clear_flag(object, 1 << 5); // 禁用边缘回弹
        lvx_object_set_scroll_dir(object, 3); // 横向滚动
        lvx_object_set_scroll_snap_x(object, 3); // 居中吸附
        lvx_object_set_scrollbar_mode(object, 0);
    }
    style(object, BG_OPA, 0);
    style(object, BORDER_OPA, 0);
    object
}

pub unsafe fn page(parent: Obj, x: i32, width: i32, height: i32) -> Obj {
    let object = unsafe { lvx_object_create(parent) };
    if object.is_null() {
        return object;
    }
    unsafe {
        lvx_object_set_size(object, width, height);
        lvx_object_align(object, ALIGN_TOP_LEFT, x, 0);
        lvx_object_add_flag(object, 1 << 12); // 可吸附页面
        lvx_object_set_scroll_dir(object, 12); // 纵向滚动
        lvx_object_set_scrollbar_mode(object, 0);
        lvx_object_clear_flag(object, 1 << 1); // 可点击
    }
    style(object, BG_OPA, 0);
    style(object, BORDER_OPA, 0);
    object
}

pub struct HeaderButton {
    pub root: Obj,
    pub icon: Obj,
}

pub unsafe fn header_button(
    parent: Obj,
    background: &[u8],
    glyph: &[u8],
    x: i32,
    y: i32,
) -> HeaderButton {
    unsafe { round_button(parent, background, glyph, x, y, 68, 14) }
}

pub(super) fn recolor(image: Obj, color: u32) {
    style(image, IMAGE_RECOLOR, color);
    style(image, IMAGE_RECOLOR_OPA, 255);
}

pub unsafe fn round_button(
    parent: Obj,
    background: &[u8],
    glyph: &[u8],
    x: i32,
    y: i32,
    size: i32,
    inset: i32,
) -> HeaderButton {
    let root = unsafe { image(parent, background, x, y) };
    let mut button = HeaderButton {
        root,
        icon: core::ptr::null_mut(),
    };
    if root.is_null() {
        return button;
    }
    unsafe {
        lvx_object_clear_flag(root, SCROLLABLE);
        button.icon = image(root, glyph, inset, inset);
        if !button.icon.is_null() {
            lvx_object_clear_flag(button.icon, CLICKABLE | SCROLLABLE);
            if background == assets::BUTTON_PRIMARY {
                if let Some(colors) = theme::primary_icon_button() {
                    recolor(button.icon, colors.foreground);
                }
            }
        }
        anim::attach(root, size / 2, size / 2, anim::Feedback::Scale);
    }
    button
}

pub unsafe fn card(parent: Obj, x: i32, y: i32) -> Obj {
    unsafe { card_with(parent, x, y, anim::Feedback::Card) }
}

pub unsafe fn card_with(parent: Obj, x: i32, y: i32, feedback: anim::Feedback) -> Obj {
    let object = unsafe { image(parent, assets::CARD_FACE, x, y) };
    if object.is_null() {
        return object;
    }
    style(object, BG_OPA, 0);
    style(object, BORDER_WIDTH, 0);
    style(object, BORDER_OPA, 0);
    unsafe {
        lvx_object_clear_flag(object, SCROLLABLE);
        anim::attach(object, CARD_WIDTH / 2, CARD_HEIGHT / 2, feedback);
    }
    object
}

pub unsafe fn capsule(parent: Obj, x: i32, y: i32, width: i32, kind: CapsuleKind) -> Obj {
    let height = kind.height();
    let layout = CapsuleLayout::measure(width, height);
    let (left, center, right) = match kind {
        CapsuleKind::Regular => (
            assets::CAPSULE_LEFT,
            assets::CAPSULE_CENTER_WIDE,
            assets::CAPSULE_RIGHT,
        ),
        CapsuleKind::Large => (
            assets::CAPSULE_LARGE_LEFT,
            assets::CAPSULE_LARGE_CENTER_WIDE,
            assets::CAPSULE_LARGE_RIGHT,
        ),
        CapsuleKind::LargePrimary => (
            assets::CAPSULE_PRIMARY_LEFT,
            assets::CAPSULE_PRIMARY_CENTER_WIDE,
            assets::CAPSULE_PRIMARY_RIGHT,
        ),
    };
    let object = unsafe { lvx_object_create(parent) };
    if object.is_null() {
        return object;
    }
    unsafe {
        lvx_object_set_size(object, width, height);
        lvx_object_align(object, ALIGN_TOP_LEFT, x, y);
        lvx_object_clear_flag(object, SCROLLABLE);
    }
    style(object, BG_OPA, 0);
    style(object, BORDER_WIDTH, 0);
    style(object, BORDER_OPA, 0);
    style(object, RADIUS, (height / 2) as u32);
    // 裁去胶囊切片四角的黑底。
    style(object, 45, 1);
    unsafe {
        anim::attach(object, width / 2, height / 2, anim::Feedback::Capsule);
        for (path, rect, source_width) in [
            (left, layout.left, 80),
            (center, layout.center, 170),
            (right, layout.right, 80),
        ] {
            let child = image(object, path, rect.x, rect.y);
            if !child.is_null() {
                lvx_object_clear_flag(child, CLICKABLE | SCROLLABLE);
                style(child, 111, 0);
                style(child, 112, 0);
                lvx_image_set_scale(
                    child,
                    rect.w * 256 / source_width,
                    height * 256 / kind.height(),
                );
            }
        }
    }
    object
}

pub unsafe fn image_child(parent: Obj, path: &[u8], x: i32, y: i32) {
    let image = unsafe { image(parent, path, x, y) };
    if !image.is_null() {
        unsafe { lvx_object_clear_flag(image, CLICKABLE) };
    }
}

pub fn bind(object: Obj, callback: LvxEventCallback, data: usize) {
    if !object.is_null() {
        unsafe { lvx_event_add(object, callback, EVENT_CLICKED, data as Obj) };
    }
}

pub fn touch_point(event: Obj) -> Option<[i32; 2]> {
    unsafe {
        let indev = generated_symbols::raw_lv_event_get_indev(event);
        if indev.is_null() {
            return None;
        }
        let mut point = [0; 2];
        generated_symbols::raw_lv_indev_get_point(indev, point.as_mut_ptr());
        Some(point)
    }
}
