//! Non-interactive text panels for explanatory content and section labels.

use core::ffi::CStr;

use crate::{geometry::Rect, theme::Color};
use canopus_target_private::*;

use super::{
    assets, cards,
    lv::{self, Obj},
};

const INSET: i32 = 18;
const MIN_HEIGHT: i32 = 80;
const TITLE_HEIGHT: i32 = 34;
const BODY_LINE_HEIGHT: i32 = 34;
const ICON_SIZE: i32 = 48;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextBlockStyle {
    #[default]
    Standard,
    Compact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextBlockOptions<'a> {
    pub style: TextBlockStyle,
    pub show_title: bool,
    pub show_subtitle: bool,
    pub icon: Option<&'a CStr>,
}

impl Default for TextBlockOptions<'_> {
    fn default() -> Self {
        Self {
            style: TextBlockStyle::Standard,
            show_title: true,
            show_subtitle: true,
            icon: None,
        }
    }
}

/// Creates a transparent text block. The area height bounds the wrapped body text.
///
/// # Safety
/// The parent must be valid and creation must take place on the UI thread.
pub unsafe fn create(
    parent: Obj,
    area: Rect,
    title: &str,
    subtitle: &str,
    options: TextBlockOptions<'_>,
) -> Option<Obj> {
    let compact = options.style == TextBlockStyle::Compact;
    let show_body = options.show_subtitle && !compact;
    let min_height = if options.show_title && show_body {
        102
    } else {
        MIN_HEIGHT
    };
    let text_x = if compact {
        INSET + ICON_SIZE + 16
    } else {
        INSET
    };
    let width = area.w - text_x - INSET;
    if parent.is_null() || width <= 0 || area.h < min_height {
        return None;
    }

    unsafe {
        let root = cards::container(parent, area.x, area.y, area.w, area.h);
        if root.is_null() {
            return None;
        }
        lv::inspect_name(root, c"AerisUI.TextBlock");

        if compact {
            let tile = lv::background(root, ICON_SIZE, ICON_SIZE, Color::PrimaryText.resolve(), 12);
            if !tile.is_null() {
                lvx_object_align(tile, 1, INSET, (area.h - ICON_SIZE) / 2);
                lvx_object_clear_flag(tile, lv::CLICKABLE | lv::SCROLLABLE);
                lv::style(tile, lv::BG_OPA, 51);
                let icon = options
                    .icon
                    .map(CStr::to_bytes_with_nul)
                    .unwrap_or(assets::TEXT_BLOCK_LIGHTBULB);
                lv::image_child(tile, icon, 8, 8);
            }
        }

        let title_y = if show_body {
            16
        } else {
            (area.h - TITLE_HEIGHT) / 2
        };
        if options.show_title {
            let label = cards::text(
                root,
                title,
                text_x,
                title_y,
                width,
                TITLE_HEIGHT,
                Color::PrimaryText.resolve(),
                false,
            );
            if label.is_null() {
                lvx_object_delete(root);
                return None;
            }
            lvx_label_set_long_mode(label, 1);
        }
        if show_body {
            let y = if options.show_title {
                title_y + TITLE_HEIGHT + 2
            } else {
                16
            };
            let label = cards::text(
                root,
                subtitle,
                text_x,
                y,
                width,
                ((area.h - y - 16) / BODY_LINE_HEIGHT) * BODY_LINE_HEIGHT,
                Color::SecondaryText.resolve(),
                false,
            );
            if label.is_null() {
                lvx_object_delete(root);
                return None;
            }
            lv::style(label, lv::TEXT_LINE_SPACE, 2);
            lvx_label_set_long_mode(label, 0);
        }
        Some(root)
    }
}
