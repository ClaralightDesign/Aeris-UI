//! 卡片文字、状态卡片和分层选择控件。

use super::{
    anim, assets,
    lv::{self, Font, Obj},
};
use crate::card::Transition;
use crate::theme::Color;
use canopus_target_private::*;

pub const WIDTH: i32 = 328;
pub const CARD_TEXT_PX: i32 = 25;
const NULL: Obj = core::ptr::null_mut();
pub const LINE_HEIGHT: i32 = 32;
pub const SECOND_LINE_Y: i32 = 16 + LINE_HEIGHT + 1;

#[derive(Clone, Copy)]
pub struct Choice {
    on: Obj,
    off: Obj,
    thumb: Obj,
    transition: Transition,
    radio: bool,
}

const EMPTY_CHOICE: Choice = Choice {
    on: NULL,
    off: NULL,
    thumb: NULL,
    transition: Transition::new(false),
    radio: false,
};

pub unsafe fn container(parent: Obj, x: i32, y: i32, width: i32, height: i32) -> Obj {
    unsafe {
        let object = lv::background(parent, width, height, 0, 0);
        if !object.is_null() {
            lvx_object_align(object, 1, x, y);
            lvx_object_clear_flag(object, lv::CLICKABLE | lv::SCROLLABLE);
            lv::style(object, lv::BG_OPA, 0);
        }
        object
    }
}

pub unsafe fn text_font(
    parent: Obj,
    value: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: u32,
    font: Font,
    px: i32,
) -> Obj {
    // 固定文案在栈上补终止符，LVGL 会复制文本。
    let mut bytes = [0u8; 256];
    let length = value.floor_char_boundary(value.len().min(bytes.len() - 1));
    bytes[..length].copy_from_slice(&value.as_bytes()[..length]);
    unsafe {
        let object = lv::label(parent, &bytes, x, y, width, height, color, font);
        lv::style(object, 94, 1);
        lv::text_px(object, width, height, 1, font, px);
        object
    }
}

pub unsafe fn text(
    parent: Obj,
    value: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: u32,
    medium: bool,
) -> Obj {
    let font = if medium {
        Font::Medium24
    } else {
        Font::Small24
    };
    unsafe {
        text_font(
            parent,
            value,
            x,
            y,
            width,
            height,
            color,
            font,
            CARD_TEXT_PX,
        )
    }
}

pub unsafe fn pair_font(
    parent: Obj,
    title: &str,
    subtitle: &str,
    width: i32,
    secondary: u32,
    font: Font,
    px: i32,
) -> Obj {
    unsafe { pair_labels(parent, title, subtitle, width, secondary, font, px).0 }
}

unsafe fn pair_labels(
    parent: Obj,
    title: &str,
    subtitle: &str,
    width: i32,
    secondary: u32,
    font: Font,
    px: i32,
) -> (Obj, Obj) {
    unsafe {
        let label = text_font(
            parent,
            title,
            22,
            16,
            width,
            LINE_HEIGHT,
            Color::PrimaryText.resolve(),
            font,
            px,
        );
        let detail = text(
            parent,
            subtitle,
            22,
            SECOND_LINE_Y,
            width,
            LINE_HEIGHT,
            secondary,
            false,
        );
        if !detail.is_null() {
            lvx_label_set_long_mode(detail, 1);
        }
        (label, detail)
    }
}

pub unsafe fn pair(parent: Obj, title: &str, subtitle: &str, width: i32, secondary: u32) -> Obj {
    unsafe {
        pair_font(
            parent,
            title,
            subtitle,
            width,
            secondary,
            Font::Medium24,
            CARD_TEXT_PX,
        )
    }
}

impl Choice {
    pub unsafe fn switch(card: Obj, title: &str, subtitle: &str, selected: bool) -> Self {
        let mut entry = Self {
            transition: Transition::new(selected),
            ..EMPTY_CHOICE
        };
        unsafe {
            pair(card, title, subtitle, 181, Color::SecondaryText.resolve());
            let control = container(card, WIDTH - 99, 29, 81, 40);
            if !control.is_null() {
                lv::inspect_name(control, c"AerisUI.Switch");
                entry.off = lv::image(control, assets::CARD_SWITCH_OFF_BG, 0, 0);
                entry.on = lv::image(control, assets::CARD_SWITCH_ON_BG, 0, 0);
                entry.thumb = lv::image(control, assets::CARD_SWITCH_THUMB, 5, 5);
                for object in [entry.off, entry.on, entry.thumb] {
                    if !object.is_null() {
                        lvx_object_clear_flag(object, lv::CLICKABLE | lv::SCROLLABLE);
                    }
                }
            }
        }
        render_choice(&entry);
        entry
    }

    pub unsafe fn radio(card: Obj, title: &str, subtitle: &str, selected: bool) -> Self {
        let mut entry = Self {
            transition: Transition::new(selected),
            radio: true,
            ..EMPTY_CHOICE
        };
        unsafe {
            pair(card, title, subtitle, 224, Color::SecondaryText.resolve());
            entry.off = container(card, WIDTH - 58, 29, 40, 40);
            if !entry.off.is_null() {
                lv::inspect_name(entry.off, c"AerisUI.Radio");
                entry.on = lv::dot(entry.off, 0, 0, 40, 255);
                lv::style(entry.on, 28, 0x3488FF);
                entry.thumb = lv::dot(entry.off, 10, 10, 20, 255);
                lv::style(entry.off, 28, 0xFFFFFF);
                lv::style(entry.off, lv::BG_OPA, 26);
                lv::style(entry.off, 12, 20);
            }
        }
        render_choice(&entry);
        entry
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.transition = Transition::new(selected);
        render_choice(self);
    }

    pub fn animate_to(&mut self, selected: bool) {
        self.transition.retarget(selected, anim::now_ms());
    }

    pub fn draw(&mut self, now: u32) -> bool {
        let previous = self.transition.value;
        let moving = self.transition.sample(now);
        if previous != self.transition.value {
            render_choice(self);
        }
        moving
    }
}

fn render_choice(entry: &Choice) {
    let amount = entry.transition.value;
    lv::style(entry.on, lv::OPA, (255 * amount / 1000) as u32);
    if entry.radio {
        let size = 22 * amount / 1000;
        if !entry.thumb.is_null() {
            unsafe {
                lvx_set_hidden(entry.thumb, u32::from(size == 0));
                lvx_object_set_size(entry.thumb, size.max(1), size.max(1));
                lvx_object_align(entry.thumb, 1, (40 - size) / 2, (40 - size) / 2);
            }
        }
    } else {
        lv::style(entry.off, lv::OPA, (255 * (1000 - amount) / 1000) as u32);
        lv::style(entry.thumb, 106, (27 * amount / 1000) as u32);
        lv::style(entry.thumb, lv::OPA, (153 + 102 * amount / 1000) as u32);
    }
}

pub unsafe fn stepper(parent: Obj, y: i32, on_step: LvxEventCallback) -> (Obj, Obj) {
    unsafe {
        let row = container(parent, 4, y, 328, 65);
        lv::inspect_name(row, c"AerisUI.Stepper");
        if row.is_null() {
            return (row, row);
        }
        let label = lv::label(row, b"\0", 65, 16, 198, 33, 0xDDDDDD, Font::Small24);
        for (index, (x, image)) in [
            (1, assets::EDITOR_STAGE_PREV),
            (262, assets::EDITOR_STAGE_NEXT),
        ]
        .into_iter()
        .enumerate()
        {
            let hit = container(row, x, 0, 65, 65);
            if !hit.is_null() {
                lv::image_child(hit, image, 16, 16);
                anim::attach(hit, 32, 32, anim::Feedback::Scale);
                lv::bind(hit, on_step, index);
            }
        }
        (row, label)
    }
}

pub unsafe fn switch_row(list: Obj, y: i32, title: &str, on_click: LvxEventCallback) {
    unsafe {
        let row = lv::background(list, 328, 68, 0x1F1F1F, 34);
        lv::inspect_name(row, c"AerisUI.SwitchRow");
        if !row.is_null() {
            lvx_object_align(row, 1, 4, y);
            lvx_object_clear_flag(row, lv::SCROLLABLE);
            anim::attach(row, 164, 34, anim::Feedback::Scale);
            lv::label(
                row,
                title.as_bytes(),
                0,
                18,
                328,
                32,
                Color::PrimaryText.resolve(),
                Font::Medium24,
            );
            lv::bind(row, on_click, 0);
        }
    }
}

pub unsafe fn status_card(parent: Obj, y: i32, title: &str, subtitle: &str) -> Obj {
    unsafe {
        status_card_with(
            parent,
            y,
            title,
            subtitle,
            assets::CARD_STATUS_SUCCESS,
            0xC3E5E1,
        )
    }
}

pub unsafe fn status_card_with(
    parent: Obj,
    y: i32,
    title: &str,
    subtitle: &str,
    art: &[u8],
    secondary: u32,
) -> Obj {
    unsafe {
        let card = lv::image(parent, art, 4, y);
        lv::inspect_name(card, c"AerisUI.StatusCard");
        if !card.is_null() {
            lvx_object_clear_flag(card, lv::SCROLLABLE);
            pair(card, title, subtitle, WIDTH - 152, secondary);
        }
        card
    }
}

pub unsafe fn jump_card(parent: Obj, y: i32, title: &str, subtitle: &str) -> Obj {
    unsafe { jump_card_with_detail(parent, y, title, subtitle).0 }
}

pub unsafe fn jump_card_with_detail(
    parent: Obj,
    y: i32,
    title: &str,
    subtitle: &str,
) -> (Obj, Obj) {
    unsafe {
        let card = lv::card(parent, 4, y);
        lv::inspect_name(card, c"AerisUI.JumpCard");
        if card.is_null() {
            return (card, NULL);
        }
        let (_, detail) = pair_labels(
            card,
            title,
            subtitle,
            230,
            Color::SecondaryText.resolve(),
            Font::Medium24,
            CARD_TEXT_PX,
        );
        lv::image_child(card, assets::CARD_CHEVRON, WIDTH - 58, 29);
        (card, detail)
    }
}
