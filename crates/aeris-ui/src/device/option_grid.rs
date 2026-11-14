//! 带主标题与副标题的双列选项网格。

use core::ffi::CStr;

use crate::theme::Color;
use canopus_target_private::*;

use super::{
    anim, assets, font,
    lv::{self, Font, Obj},
    page,
};

const OPTION_WIDTH: i32 = 152;
const OPTION_HEIGHT: i32 = 112;
const OPTION_GAP: i32 = 14;
const GRID_LEFT: i32 = (page::WIDTH - 2 * OPTION_WIDTH - OPTION_GAP) / 2;
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
        for (index, item) in items.into_iter().enumerate() {
            let x = GRID_LEFT + index as i32 % 2 * (OPTION_WIDTH + OPTION_GAP);
            let y = top + index as i32 / 2 * (OPTION_HEIGHT + OPTION_GAP);
            let button = lv::image(panel, assets::TIMER_CAPSULE_SHORT, x, y);
            if button.is_null() {
                continue;
            }
            lv::inspect_name(button, c"AerisUI.OptionGridItem");
            lvx_object_clear_flag(button, lv::SCROLLABLE);
            anim::attach(
                button,
                OPTION_WIDTH / 2,
                OPTION_HEIGHT / 2,
                anim::Feedback::Scale,
            );

            let mut digits = [0; 11];
            let (title, numeric) = match item.title {
                OptionTitle::Number(value) => (decimal(value, &mut digits), true),
                OptionTitle::Text(text) => (text.to_bytes_with_nul(), false),
            };
            if !numeric
                || font::fixed(button, font::OPTION, title, [0, 22, OPTION_WIDTH, 44], 2).is_none()
            {
                let label = lv::label(
                    button,
                    title,
                    0,
                    22,
                    OPTION_WIDTH,
                    44,
                    Color::PrimaryText.resolve(),
                    Font::Demibold32,
                );
                lv::text_px(
                    label,
                    OPTION_WIDTH,
                    44,
                    2,
                    Font::Demibold32,
                    if numeric { 40 } else { 32 },
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
                OPTION_WIDTH,
                28,
                Color::Accent.resolve(),
                Font::Medium24,
            );
            if !subtitle.is_null() {
                lvx_label_set_long_mode(subtitle, LONG_ELLIPSIS);
            }
            lvx_event_add(button, on_select, EVENT_CLICKED, item.data as Obj);
        }
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
