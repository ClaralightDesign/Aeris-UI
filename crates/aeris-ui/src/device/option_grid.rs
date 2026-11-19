//! 带主标题与副标题的双列选项网格。

use core::ffi::CStr;

use crate::{geometry::Rect, theme::Color};
use canopus_target_private::*;

use super::{
    anim, assets, font,
    lv::{self, Font, Obj},
    page,
};

const OPTION_WIDTH: i32 = 152;
const OPTION_HEIGHT: i32 = 112;
const OPTION_GAP: i32 = 14;
const LONG_ELLIPSIS: u32 = 1;

pub enum OptionTitle<'a> {
    Number(u32),
    Text(&'a CStr),
}

pub struct OptionItem<'a> {
    pub title: OptionTitle<'a>,
    pub subtitle: &'a CStr,
    pub data: usize,
}

pub unsafe fn grid<'a>(
    panel: Obj,
    top: i32,
    items: impl IntoIterator<Item = OptionItem<'a>>,
    on_select: LvxEventCallback,
) {
    unsafe {
        grid_in(
            panel,
            Rect::new(0, top, page::WIDTH, page::HEIGHT),
            items,
            on_select,
        )
    }
}

/// Places one or two columns inside the given width. The parent scrolls
/// when the rows extend below its viewport.
///
/// # Safety
/// Call on the UI thread with a live parent.
pub unsafe fn grid_in<'a>(
    panel: Obj,
    area: Rect,
    items: impl IntoIterator<Item = OptionItem<'a>>,
    on_select: LvxEventCallback,
) {
    if area.w <= 0 {
        return;
    }
    let columns = if area.w >= 2 * OPTION_WIDTH + OPTION_GAP {
        2
    } else {
        1
    };
    let width = OPTION_WIDTH.min(area.w);
    let grid_width = columns * width + (columns - 1) * OPTION_GAP;
    let left = area.x + (area.w - grid_width) / 2;
    unsafe {
        for (index, item) in items.into_iter().enumerate() {
            let x = left + index as i32 % columns * (width + OPTION_GAP);
            let y = area.y + index as i32 / columns * (OPTION_HEIGHT + OPTION_GAP);
            build_button(panel, x, y, width, item, on_select);
        }
    }
}

/// Creates the in-page 152 × 112 value and unit button used by the option grid.
pub unsafe fn in_page_capsule_button(
    parent: Obj,
    x: i32,
    y: i32,
    item: OptionItem<'_>,
    on_select: LvxEventCallback,
) -> Obj {
    unsafe { build_button(parent, x, y, OPTION_WIDTH, item, on_select) }
}

unsafe fn build_button(
    parent: Obj,
    x: i32,
    y: i32,
    width: i32,
    item: OptionItem<'_>,
    on_select: LvxEventCallback,
) -> Obj {
    unsafe {
        let button = if width == OPTION_WIDTH {
            lv::image(parent, assets::TIMER_CAPSULE_SHORT, x, y)
        } else {
            let button = lv::background(
                parent,
                width,
                OPTION_HEIGHT,
                Color::PrimaryText.resolve(),
                36,
            );
            if !button.is_null() {
                lvx_object_align(button, 1, x, y);
                lv::style(button, lv::BG_OPA, 31);
                lvx_object_add_flag(button, lv::CLICKABLE);
            }
            button
        };
        if button.is_null() {
            return button;
        }
        lv::inspect_name(button, c"AerisUI.OptionGridItem");
        lvx_object_clear_flag(button, lv::SCROLLABLE);
        anim::attach(button, width / 2, OPTION_HEIGHT / 2, anim::Feedback::Scale);

        let mut digits = [0; 11];
        let (title, numeric) = match item.title {
            OptionTitle::Number(value) => (decimal(value, &mut digits), true),
            OptionTitle::Text(text) => (text.to_bytes_with_nul(), false),
        };
        lv::reload_label(button, title);
        if !numeric
            || font::fixed_apx(
                button,
                font::OPTION,
                title,
                [0, 22, width, 44],
                2,
                40,
                crate::TextSize::Number.apx(),
            )
            .is_none()
        {
            let label = lv::label(
                button,
                title,
                0,
                22,
                width,
                44,
                Color::PrimaryText.resolve(),
                Font::Demibold32,
            );
            lv::text_apx(
                label,
                width,
                44,
                2,
                Font::Demibold32,
                if numeric {
                    crate::TextSize::Number.apx()
                } else {
                    crate::TextSize::Display.apx()
                },
            );
            if !label.is_null() {
                lvx_label_set_long_mode(label, LONG_ELLIPSIS);
            }
        }
        let subtitle = lv::label(
            button,
            item.subtitle.to_bytes_with_nul(),
            0,
            66,
            width,
            28,
            Color::Accent.resolve(),
            Font::Medium24,
        );
        if !subtitle.is_null() {
            lvx_label_set_long_mode(subtitle, LONG_ELLIPSIS);
        }
        lvx_event_add(button, on_select, EVENT_CLICKED, item.data as Obj);
        button
    }
}

fn decimal(mut value: u32, buffer: &mut [u8; 11]) -> &[u8] {
    let mut index = buffer.len() - 1;
    buffer[index] = 0;
    loop {
        index -= 1;
        buffer[index] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            return &buffer[index..];
        }
    }
}
