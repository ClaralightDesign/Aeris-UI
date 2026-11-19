//! 计时器使用的 LVGL 组件，所有操作均在 UI 线程执行。

use core::ffi::c_void;

use crate::{
    capsule::{CapsuleKind, CapsuleLayout, CAPSULE_SLICE_WIDTH},
    geometry::Rect,
    theme,
};
use canopus_target_private::*;

use super::{anim, assets, page, round_button};

pub type Obj = *mut c_void;

pub(super) const EVENT_DELETE: u32 = 36;

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
pub(super) const RADIUS: u32 = 44;
const BORDER_WIDTH: u32 = 48;
const BORDER_OPA: u32 = 50;
const IMAGE_RECOLOR: u32 = 69;
const IMAGE_RECOLOR_OPA: u32 = 70;
const TEXT_COLOR: u32 = 88;
pub(super) const TEXT_OPA: u32 = 89;
pub(super) const TEXT_LINE_SPACE: u32 = 92;
pub const OPA: u32 = 95;
pub const TRANSLATE_X: u32 = 106;
pub const TRANSLATE_Y: u32 = 107;

#[derive(Copy, Clone)]
pub enum Font {
    Small24,
    Medium24,
    Medium28,
    Demibold32,
    #[cfg(any(feature = "gallery", target_os = "emscripten"))]
    Rounded44,
}

impl Font {
    /// Default size is a design token, independent of the firmware font atlas.
    pub const fn text_size(self) -> crate::TextSize {
        match self {
            Self::Small24 | Self::Medium24 => crate::TextSize::Body,
            Self::Medium28 => crate::TextSize::Heading,
            Self::Demibold32 => crate::TextSize::Display,
            #[cfg(any(feature = "gallery", target_os = "emscripten"))]
            Self::Rounded44 => crate::TextSize::LargeTitle,
        }
    }

    /// 固件字体样式的基础字号。
    pub const fn base_px(self) -> i32 {
        match self {
            Font::Demibold32 => {
                if cfg!(all(
                    feature = "band-11",
                    not(feature = "band-10-pro"),
                    not(feature = "gallery"),
                    not(target_os = "emscripten")
                )) {
                    28
                } else {
                    32
                }
            }
            #[cfg(any(feature = "gallery", target_os = "emscripten"))]
            Font::Rounded44 => 44,
            Font::Medium28 => 28,
            Font::Small24 | Font::Medium24 => 24,
        }
    }
}

pub const TRANSFORM_SCALE_X: u32 = 108;
pub const TRANSFORM_SCALE_Y: u32 = 109;
pub(super) const TRANSFORM_ROTATION: u32 = 110;
pub const TRANSFORM_PIVOT_X: u32 = 111;
pub const TRANSFORM_PIVOT_Y: u32 = 112;

/// 设置文字的显示字号。
pub fn text_px(object: Obj, width: i32, height: i32, align: u32, font: Font, px: i32) {
    if object.is_null() || px <= 0 {
        return;
    }
    #[cfg(any(feature = "gallery", target_os = "emscripten"))]
    {
        let _ = align;
        let line_height = super::linoval::apply(object, font, px);
        if height < line_height {
            unsafe { lvx_object_set_size(object, width, line_height) };
        }
    }
    #[cfg(not(any(feature = "gallery", target_os = "emscripten")))]
    {
        let base = font.base_px();
        let scale = (px * 256 + base / 2) / base;
        let source_width = (width * base + px - 1) / px;
        let source_height = (height * base + px - 1) / px;
        unsafe { lvx_object_set_size(object, source_width, source_height) };
        let pivot_x = match align {
            1 => 0,
            3 => source_width,
            _ => source_width / 2,
        };
        let offset_x = match align {
            1 => 0,
            3 => width - source_width,
            _ => (width - source_width) / 2,
        };
        style(object, TRANSLATE_X, offset_x as u32);
        style(object, TRANSLATE_Y, 0);
        style(object, TRANSFORM_PIVOT_X, pivot_x as u32);
        style(object, TRANSFORM_PIVOT_Y, 0);
        style(object, TRANSFORM_SCALE_X, scale as u32);
        style(object, TRANSFORM_SCALE_Y, scale as u32);
    }
}

/// Resolves a logical font size at the component boundary. Explicit pixel
/// overrides remain available through `text_px`.
pub fn text_apx(object: Obj, width: i32, height: i32, align: u32, font: Font, size: crate::Apx) {
    text_apx_in(
        object,
        super::display_profile().viewport,
        width,
        height,
        align,
        font,
        size,
    );
}

pub fn text_apx_in(
    object: Obj,
    viewport: crate::geometry::Viewport,
    width: i32,
    height: i32,
    align: u32,
    font: Font,
    size: crate::Apx,
) {
    text_px(object, width, height, align, font, viewport.apx(size));
    inspect_text_apx(object, viewport, size);
}

pub(super) fn inspect_text_apx(object: Obj, viewport: crate::geometry::Viewport, size: crate::Apx) {
    #[cfg(target_os = "emscripten")]
    if !object.is_null() {
        unsafe extern "C" {
            fn gallery_text_apx(object: Obj, halves: i32, pixels: i32);
        }
        unsafe { gallery_text_apx(object, size.halves(), viewport.apx(size)) };
    }
    #[cfg(not(target_os = "emscripten"))]
    let _ = (object, viewport, size);
}

pub fn style(object: Obj, property: u32, value: u32) {
    if !object.is_null() {
        unsafe { lvx_object_set_local_style_u32(object, property, value, 0) };
    }
}

// 预览组件名随 LVGL 对象销毁，设备端不记录。
#[inline]
pub(super) unsafe fn inspect_name(_object: Obj, _name: &'static core::ffi::CStr) {
    #[cfg(target_os = "emscripten")]
    {
        extern "C" {
            fn lv_obj_set_name_static(object: Obj, name: *const core::ffi::c_char);
        }
        if !_object.is_null() {
            unsafe { lv_obj_set_name_static(_object, _name.as_ptr()) };
        }
    }
}

#[inline]
pub(super) fn reload_label(_object: Obj, _label: &[u8]) {
    #[cfg(target_os = "emscripten")]
    if !_object.is_null() {
        unsafe extern "C" {
            fn gallery_reload_label(object: Obj, label: *const u8, length: usize);
        }
        let label = _label.strip_suffix(&[0]).unwrap_or(_label);
        unsafe { gallery_reload_label(_object, label.as_ptr(), label.len()) };
    }
}

#[inline]
pub(super) fn reload_value(_object: Obj, _value: i32) {
    #[cfg(target_os = "emscripten")]
    if !_object.is_null() {
        unsafe extern "C" {
            fn gallery_reload_value(object: Obj, value: i32);
        }
        unsafe { gallery_reload_value(_object, _value) };
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
    if let Some(opacity) = accent_image_opacity(path) {
        if let Some(accent) = theme::accent_button() {
            tint(object, accent.fill, opacity);
        }
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

// Glass faces keep part of the source shading. Flat faces use the configured fill.
fn accent_image_opacity(path: &[u8]) -> Option<u32> {
    let name = path.strip_suffix(b"\0")?;
    if name.starts_with(b"round-") && name.ends_with(b"-accent.bin") {
        return Some(if name.ends_with(b"-normal-accent.bin") {
            255
        } else {
            160
        });
    }
    if name.starts_with(b"button-normal-") {
        return Some(if name.starts_with(b"button-normal-dimmed-") {
            128
        } else {
            255
        });
    }
    if name.starts_with(b"gallery-capsule-primary-") {
        return (theme::Color::Accent.resolve() != theme::Color::Accent.default_rgb())
            .then_some(160);
    }
    // DevBox tints these stock bitmaps during asset registration.
    #[cfg(not(target_arch = "wasm32"))]
    if name == b"header-button-primary.bin" || name.starts_with(b"capsule-primary-") {
        return Some(160);
    }
    None
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
        #[cfg(not(any(feature = "gallery", target_os = "emscripten")))]
        lvx_label_set_text(object, text.as_ptr());
        lvx_object_set_size(object, width, height);
        lvx_object_align(object, ALIGN_TOP_LEFT, x, y);
        lvx_label_set_text_align_center(object);
        lvx_object_clear_flag(object, CLICKABLE);
        #[cfg(any(feature = "gallery", target_os = "emscripten"))]
        {
            let px = font.text_size().pixels(super::display_profile().viewport);
            let line_height = super::linoval::apply(object, font, px);
            if height < line_height {
                lvx_object_set_size(object, width, line_height);
            }
        }
        #[cfg(not(any(feature = "gallery", target_os = "emscripten")))]
        {
            #[cfg(feature = "band-10-pro")]
            let style_ptr = match font {
                Font::Small24 => canopus_target_private::STYLE_MISANS_REGULAR_24,
                Font::Medium24 => generated_symbols::STYLE_MISANS_MEDIUM_24_ENTRY,
                Font::Medium28 => generated_symbols::STYLE_MISANS_MEDIUM_28_ENTRY,
                Font::Demibold32 => STYLE_MISANS_DEMIBOLD_32,
            };
            #[cfg(all(feature = "band-11", not(feature = "band-10-pro")))]
            let style_ptr = match font {
                Font::Small24 | Font::Medium24 => {
                    canopus_target_generated::canopus_fw_style_misans_medium_24
                }
                Font::Medium28 | Font::Demibold32 => {
                    canopus_target_generated::canopus_fw_style_misans_medium_28
                }
            };
            lvx_style_apply(object, style_ptr as *const c_void, 255, 0);
            text_apx(object, width, height, 2, font, font.text_size().apx());
        }
        #[cfg(any(feature = "gallery", target_os = "emscripten"))]
        lvx_label_set_text(object, text.as_ptr());
    }
    style(object, BG_OPA, 0);
    style(object, BORDER_OPA, 0);
    style(object, TEXT_COLOR, color);
    style(object, TEXT_OPA, 255);
    inspect_text_apx(
        object,
        super::display_profile().viewport,
        font.text_size().apx(),
    );
    object
}

pub(super) unsafe fn label_str(parent: Obj, text: &str, area: Rect, color: u32, font: Font) -> Obj {
    let mut bytes = [0; 256];
    let end = text.floor_char_boundary(text.len().min(bytes.len() - 1));
    bytes[..end].copy_from_slice(&text.as_bytes()[..end]);
    unsafe { label(parent, &bytes, area.x, area.y, area.w, area.h, color, font) }
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
    unsafe { inspect_name(object, c"AerisUI.Pager") };
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
    unsafe { inspect_name(object, c"AerisUI.Page") };
    style(object, BG_OPA, 0);
    style(object, BORDER_OPA, 0);
    object
}

pub struct HeaderButton {
    pub root: Obj,
    pub icon: Obj,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoundButtonStyle {
    #[default]
    Neutral,
    Accent,
}

/// Creates the 68 px page-edge button. Bind the click callback to `root`.
///
/// # Safety
/// The parent must be valid and the glyph path must be NUL terminated.
pub unsafe fn round_button_with_style(
    parent: Obj,
    x: i32,
    y: i32,
    glyph: &[u8],
    style: RoundButtonStyle,
) -> HeaderButton {
    let background = match style {
        RoundButtonStyle::Neutral => assets::BUTTON_BLACK,
        RoundButtonStyle::Accent => assets::BUTTON_PRIMARY,
    };
    unsafe { header_button(parent, background, glyph, x, y) }
}

pub unsafe fn header_button(
    parent: Obj,
    background: &[u8],
    glyph: &[u8],
    x: i32,
    y: i32,
) -> HeaderButton {
    let background = if theme::ButtonTheme::default() == theme::ButtonTheme::Normal {
        if background == assets::BUTTON_PRIMARY {
            round_button::STANDARD_FLAT_ACCENT
        } else if background == assets::BUTTON_BLACK || background == assets::BUTTON_ABOUT {
            round_button::STANDARD_FLAT_NEUTRAL
        } else {
            background
        }
    } else {
        background
    };
    let button = unsafe { round_button(parent, background, glyph, x, y, 68, 14) };
    unsafe { inspect_name(button.root, c"AerisUI.HeaderButton") };
    button
}

pub(super) fn recolor(image: Obj, color: u32) {
    tint(image, color, 255);
}

pub(super) fn tint(image: Obj, color: u32, opacity: u32) {
    style(image, IMAGE_RECOLOR, color);
    style(image, IMAGE_RECOLOR_OPA, opacity);
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
            if background == assets::BUTTON_PRIMARY
                || background == round_button::STANDARD_FLAT_ACCENT
            {
                if let Some(colors) = theme::primary_icon_button() {
                    recolor(button.icon, colors.foreground);
                }
            }
        }
        inspect_name(root, c"AerisUI.RoundButton");
        anim::attach(root, size / 2, size / 2, anim::Feedback::Scale);
    }
    button
}

pub unsafe fn card(parent: Obj, x: i32, y: i32) -> Obj {
    unsafe { card_with(parent, x, y, anim::Feedback::Card) }
}

/// Draws the standard row inside its assigned bounds.
pub unsafe fn card_in(parent: Obj, area: Rect) -> Obj {
    if area.w == CARD_WIDTH && area.h == CARD_HEIGHT {
        return unsafe { card(parent, area.x, area.y) };
    }
    unsafe {
        let card = background(
            parent,
            area.w,
            area.h,
            theme::Color::PrimaryText.resolve(),
            32,
        );
        if !card.is_null() {
            lvx_object_align(card, ALIGN_TOP_LEFT, area.x, area.y);
            lvx_object_clear_flag(card, SCROLLABLE);
            style(card, BG_OPA, 31);
            inspect_name(card, c"AerisUI.Card");
            anim::attach(card, area.w / 2, area.h / 2, anim::Feedback::Scale);
        }
        card
    }
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
        inspect_name(object, c"AerisUI.Card");
        anim::attach(object, CARD_WIDTH / 2, CARD_HEIGHT / 2, feedback);
    }
    object
}

pub unsafe fn capsule(parent: Obj, x: i32, y: i32, width: i32, kind: CapsuleKind) -> Obj {
    unsafe { build_capsule(parent, x, y, width, kind.height(), kind, 255) }
}

pub(super) unsafe fn build_capsule(
    parent: Obj,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    kind: CapsuleKind,
    opacity: u32,
) -> Obj {
    let area = Rect::new(x, y, width, height);
    if theme::ButtonTheme::default() == theme::ButtonTheme::Normal {
        match kind {
            CapsuleKind::Large => {
                return unsafe {
                    capsule_face(
                        parent,
                        area,
                        page::SECONDARY_BUTTON_SLICES,
                        80,
                        31 * opacity / 255,
                        false,
                    )
                };
            }
            CapsuleKind::LargePrimary => {
                return unsafe {
                    capsule_face(parent, area, page::NORMAL_BUTTON_SLICES, 80, opacity, false)
                };
            }
            CapsuleKind::Regular => {}
        }
    }
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
    unsafe {
        capsule_face(
            parent,
            area,
            [left, center, right],
            kind.height(),
            opacity,
            true,
        )
    }
}

pub(super) unsafe fn capsule_face(
    parent: Obj,
    area: Rect,
    slices: [&[u8]; 3],
    source_height: i32,
    opacity: u32,
    clip: bool,
) -> Obj {
    let Rect {
        x,
        y,
        w: width,
        h: height,
    } = area;
    let layout = CapsuleLayout::measure(width, height);
    let [left, center, right] = slices;
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
    style(object, 45, u32::from(clip));
    unsafe {
        inspect_name(object, c"AerisUI.Capsule");
        anim::attach(object, width / 2, height / 2, anim::Feedback::Capsule);
        for (path, rect, source_width) in [
            (left, layout.left, CAPSULE_SLICE_WIDTH),
            (center, layout.center, 170),
            (right, layout.right, CAPSULE_SLICE_WIDTH),
        ] {
            if rect.w != 0 {
                nine_patch_slice(object, path, rect, source_width, source_height, opacity);
            }
        }
    }
    object
}

unsafe fn nine_patch_slice(
    parent: Obj,
    path: &[u8],
    rect: Rect,
    source_width: i32,
    source_height: i32,
    opacity: u32,
) {
    let resized = rect.w != source_width || rect.h != source_height;
    unsafe {
        let clip = if resized {
            let Some(clip) = super::font::clip(parent, rect.w, rect.h, 0) else {
                return;
            };
            lvx_object_align(clip, ALIGN_TOP_LEFT, rect.x, rect.y);
            clip
        } else {
            parent
        };
        let (x, y) = if resized { (0, 0) } else { (rect.x, rect.y) };
        let child = image(clip, path, x, y);
        if child.is_null() {
            if resized {
                lvx_object_delete(clip);
            }
            return;
        }
        inspect_name(
            if resized { clip } else { child },
            c"AerisUI.NinePatchSlice",
        );
        lvx_object_clear_flag(child, CLICKABLE | SCROLLABLE);
        if opacity != 255 {
            style(child, OPA, opacity);
        }
        if resized {
            // LVGL scale is quantized to 1/256. Round up so the image covers
            // its rectangle, then let the parent clip the excess.
            style(child, TRANSFORM_PIVOT_X, 0);
            style(child, TRANSFORM_PIVOT_Y, 0);
            lvx_image_set_scale(
                child,
                (rect.w * 256 + source_width - 1) / source_width,
                (rect.h * 256 + source_height - 1) / source_height,
            );
        }
    }
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
    #[cfg(any(feature = "band-10-pro", target_os = "emscripten"))]
    use generated_symbols::{
        raw_lv_event_get_indev as event_indev, raw_lv_indev_get_point as indev_point,
    };
    #[cfg(all(
        feature = "band-11",
        not(feature = "band-10-pro"),
        not(target_arch = "wasm32")
    ))]
    let (event_indev, indev_point): (
        unsafe extern "C" fn(Obj) -> Obj,
        unsafe extern "C" fn(Obj, *mut i32),
    ) = unsafe {
        (
            core::mem::transmute(canopus_target_generated::CANOPUS_FW_LV_EVENT_GET_INDEV_CALLABLE),
            core::mem::transmute(canopus_target_generated::CANOPUS_FW_LV_INDEV_GET_POINT_CALLABLE),
        )
    };
    unsafe {
        let indev = event_indev(event);
        if indev.is_null() {
            return None;
        }
        let mut point = [0; 2];
        indev_point(indev, point.as_mut_ptr());
        Some(point)
    }
}
