//! 操作结果浮层的布局、自定义内容与分段动效。

use core::ffi::CStr;

use canopus_target_private::*;

use super::{
    assets, dialog, font,
    lv::{self, Font, Obj},
    page,
};
use crate::theme::Color;

const DISC_SIZE: i32 = 182;
const CENTER_Y: i32 = 221;
const ICON_SIZE: i32 = 102;
const TEXT_INSET: i32 = 22;
const TEXT_WIDTH: i32 = page::WIDTH - 2 * TEXT_INSET;
const TEXT_HEIGHT: i32 = 34;
const TEXT_Y: i32 = 422 - TEXT_HEIGHT / 2;
const ICON_DELAY_MS: u32 = 96;
const ICON_DURATION_MS: u32 = 168;
const DETAIL_DISC_SIZE: i32 = 72;
const DETAIL_CENTER_Y: i32 = 146;
const DETAIL_TITLE_Y: i32 = 198;
const DETAIL_BODY_Y: i32 = 243;
const DETAIL_BODY_HEIGHT: i32 = 214;
pub const DEFAULT_TIMEOUT_MS: u32 = 4000;
pub const DEFAULT_FIRST_STAGE_MS: u32 = 550;
pub(super) const DETAIL_DURATION_MS: u32 = 320;
const DETAIL_BODY_TRAVEL: i32 = page::HEIGHT + 18 - DETAIL_BODY_Y;

/// 底板的外观，与图标和操作结果独立。
#[derive(Clone, Copy)]
pub enum Style {
    Neutral,
    PrimarySoft,
    Primary,
}

impl Style {
    fn disc(self) -> (u32, u32) {
        let primary = Color::Accent.resolve_or(0x4187F8);
        match self {
            Self::Neutral => (Color::PrimaryText.resolve(), 26),
            Self::PrimarySoft => (primary, 64),
            Self::Primary => (primary, 255),
        }
    }
}

/// 带正文的浮层在一级样式收到点按时的行为。
#[derive(Clone, Copy)]
pub enum FirstStageTap {
    ShowDetails,
    Dismiss,
}

/// 浮层的正文、停留时间与点按行为。
#[derive(Clone, Copy)]
pub struct Options<'a> {
    /// 提供正文时，入场后过渡到二级布局。
    pub description: Option<&'a CStr>,
    /// 带正文时，一级入场完成到二级过渡开始的停留时间。
    pub first_stage_ms: u32,
    pub first_stage_tap: FirstStageTap,
    /// 最后一段停留时间。用 `None` 关闭自动隐藏。
    pub timeout_ms: Option<u32>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            description: None,
            first_stage_ms: DEFAULT_FIRST_STAGE_MS,
            first_stage_tap: FirstStageTap::Dismiss,
            timeout_ms: Some(DEFAULT_TIMEOUT_MS),
        }
    }
}

/// 项目内的 LVGL 位图及其原始像素尺寸。
#[derive(Clone, Copy)]
pub struct Icon<'a> {
    pub path: &'a CStr,
    pub width: u16,
    pub height: u16,
}

/// 在页面前景显示操作结果。传入 `None` 使用内置对号。
///
/// # 安全要求
/// 在 UI 线程调用。root 必须是有效的 336 × 480 页面根对象。
pub unsafe fn show(root: Obj, style: Style, icon: Option<Icon<'_>>, message: &CStr) -> bool {
    unsafe { show_with_options(root, style, icon, message, Options::default()) }
}

/// 显示可展开正文、可设置停留时间的操作结果。
///
/// # 安全要求
/// 在 UI 线程调用。root 必须是有效的 336 × 480 页面根对象。
pub unsafe fn show_with_options(
    root: Obj,
    style: Style,
    icon: Option<Icon<'_>>,
    message: &CStr,
    options: Options<'_>,
) -> bool {
    if root.is_null() || dialog::busy() {
        return false;
    }
    let (path, width, height) = icon.map_or(
        (assets::CONFIRMATION_SUCCESS, ICON_SIZE, ICON_SIZE),
        |icon| {
            (
                icon.path.to_bytes_with_nul(),
                i32::from(icon.width),
                i32::from(icon.height),
            )
        },
    );
    if width == 0 || height == 0 {
        return false;
    }
    let Some(layers) = (unsafe { dialog::layers(root, page::WIDTH, page::HEIGHT) }) else {
        return false;
    };
    let (disc, glyph, text, body) = unsafe {
        lv::inspect_name(layers.surface, c"AerisUI.ConfirmationOverlay");
        lv::style(layers.surface, LV_STYLE_CLIP_CORNER, 1);
        for x in [0, page::WIDTH / 2] {
            let shade = lv::image(layers.content, assets::HEADER_BACKGROUND, x, 0);
            if !shade.is_null() {
                lvx_object_clear_flag(shade, lv::CLICKABLE | lv::SCROLLABLE);
            }
        }
        let (color, opacity) = style.disc();
        let disc = lv::background(
            layers.content,
            DISC_SIZE,
            DISC_SIZE,
            color,
            (DISC_SIZE / 2) as u32,
        );
        let glyph = lv::image(
            layers.content,
            path,
            (page::WIDTH - width) / 2,
            CENTER_Y - height / 2,
        );
        let text = lv::label(
            layers.content,
            message.to_bytes_with_nul(),
            TEXT_INSET,
            TEXT_Y,
            TEXT_WIDTH,
            TEXT_HEIGHT,
            Color::PrimaryText.resolve(),
            Font::Small24,
        );
        let body = if let Some(description) = options.description {
            let label = lv::label(
                layers.content,
                description.to_bytes_with_nul(),
                TEXT_INSET,
                DETAIL_BODY_Y,
                TEXT_WIDTH,
                DETAIL_BODY_HEIGHT,
                Color::SecondaryText.resolve(),
                Font::Small24,
            );
            if label.is_null() {
                lvx_object_delete(layers.surface);
                return false;
            }
            lv::text_px(label, TEXT_WIDTH, DETAIL_BODY_HEIGHT, 2, Font::Small24, 25);
            lv::style(label, lv::TEXT_LINE_SPACE, 1);
            lvx_label_set_long_mode(label, 1);
            lvx_object_clear_flag(label, lv::CLICKABLE | lv::SCROLLABLE);
            label
        } else {
            core::ptr::null_mut()
        };
        let (clock, _) = font::clock(layers.content, &page::clock_bytes(), [132, 10, 188, 34], 3);
        if [disc, glyph, text, clock]
            .iter()
            .any(|object| object.is_null())
        {
            lvx_object_delete(layers.surface);
            return false;
        }
        for object in [disc, glyph, text, clock] {
            lvx_object_clear_flag(object, lv::CLICKABLE | lv::SCROLLABLE);
        }
        lvx_object_align(
            disc,
            1,
            (page::WIDTH - DISC_SIZE) / 2,
            CENTER_Y - DISC_SIZE / 2,
        );
        lv::style(disc, lv::BG_OPA, opacity);
        lv::style(disc, lv::TRANSFORM_PIVOT_X, (DISC_SIZE / 2) as u32);
        lv::style(disc, lv::TRANSFORM_PIVOT_Y, (DISC_SIZE / 2) as u32);
        lv::style(glyph, lv::TRANSFORM_PIVOT_X, (width / 2) as u32);
        lv::style(glyph, lv::TRANSFORM_PIVOT_Y, (height / 2) as u32);
        lv::text_px(text, TEXT_WIDTH, TEXT_HEIGHT, 2, Font::Small24, 25);
        lvx_label_set_long_mode(text, 1);
        lvx_event_add(layers.surface, tapped, EVENT_CLICKED, root);
        lvx_event_add(layers.surface, deleted, lv::EVENT_DELETE, layers.surface);
        (disc, glyph, text, body)
    };

    #[cfg(target_arch = "wasm32")]
    let reduced_motion = unsafe { gallery_prefers_reduced_motion() };
    #[cfg(not(target_arch = "wasm32"))]
    let reduced_motion = false;
    let motion = Motion {
        root,
        disc,
        glyph,
        text,
        body,
        first_stage_ms: options.first_stage_ms,
        first_stage_tap: options.first_stage_tap,
        timeout_ms: options.timeout_ms,
        glyph_scale: (256 * ICON_SIZE / width.max(height)).max(1),
        reduced_motion,
    };
    dialog::open_confirmation(root, layers, motion)
}

/// 播放退场动画。浮层打开后也可点按关闭，或等待自动关闭。
pub fn dismiss(root: Obj) -> bool {
    dialog::dismiss_confirmation(root)
}

/// 页面暂停时立即移除其浮层。
pub fn hide(root: Obj) {
    dialog::hide_confirmation(root);
}

extern "C" fn tapped(event: Obj) {
    dialog::tap_confirmation(unsafe { lvx_event_get_user_data(event) } as Obj);
}

extern "C" fn deleted(event: Obj) {
    dialog::confirmation_deleted(unsafe { lvx_event_get_user_data(event) } as Obj);
}

#[derive(Clone, Copy)]
pub(super) struct Motion {
    pub root: Obj,
    disc: Obj,
    glyph: Obj,
    text: Obj,
    body: Obj,
    pub first_stage_ms: u32,
    pub first_stage_tap: FirstStageTap,
    pub timeout_ms: Option<u32>,
    glyph_scale: i32,
    pub reduced_motion: bool,
}

impl Motion {
    pub const ICON_END_MS: u32 = ICON_DELAY_MS + ICON_DURATION_MS;

    pub fn icon_progress(elapsed: u32) -> u32 {
        dialog::ease_out(elapsed.saturating_sub(ICON_DELAY_MS), ICON_DURATION_MS)
    }

    pub fn has_detail(self) -> bool {
        !self.body.is_null()
    }

    pub fn paint(self, content: u32, icon: u32, detail: u32) {
        // 一级标题沿用同一标签，正文从屏幕下方滑入。
        let detail = detail as i32;
        let size = DISC_SIZE * 255 - (DISC_SIZE - DETAIL_DISC_SIZE) * detail;
        let disc_scale = ((320 - 64 * content / 255) as i32 * size / (DISC_SIZE * 255)) as u32;
        lv::style(self.disc, lv::TRANSFORM_SCALE_X, disc_scale);
        lv::style(self.disc, lv::TRANSFORM_SCALE_Y, disc_scale);
        let disc_offset = (DETAIL_CENTER_Y - CENTER_Y) * detail / 255;
        lv::style(self.disc, lv::TRANSLATE_Y, disc_offset as u32);
        lv::style(self.glyph, lv::TRANSLATE_Y, disc_offset as u32);
        let content_offset = 18 * (255 - content) as i32 / 255;
        let title_offset = content_offset + (DETAIL_TITLE_Y - TEXT_Y) * detail / 255;
        lv::style(self.text, lv::TRANSLATE_Y, title_offset as u32);
        lv::style(self.glyph, lv::OPA, icon);
        let icon_scale = self.glyph_scale * (64 + 192 * icon as i32 / 255) / 256;
        let scale = (icon_scale * size / (DISC_SIZE * 255)).max(1);
        unsafe { lvx_image_set_scale(self.glyph, scale, scale) };
        if !self.body.is_null() {
            let offset = content_offset + DETAIL_BODY_TRAVEL * (255 - detail) / 255;
            lv::style(self.body, lv::OPA, detail as u32);
            lv::style(self.body, lv::TRANSLATE_Y, offset as u32);
        }
    }
}
