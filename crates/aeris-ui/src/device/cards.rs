//! 卡片文字、状态卡片和分层选择控件。

use super::{
    anim, assets,
    lv::{self, Font, Obj},
};
use crate::card::Transition;
use crate::geometry::Rect;
use crate::theme::Color;
use canopus_target_private::*;

pub const WIDTH: i32 = 328;
pub const CARD_TEXT_PX: i32 = 25;
pub const CARD_TEXT_SIZE: crate::Apx = crate::Apx::new(4);

pub fn card_text_px() -> i32 {
    super::display_profile().viewport.apx(CARD_TEXT_SIZE)
}
const NULL: Obj = core::ptr::null_mut();
pub const LINE_HEIGHT: i32 = 32;
pub const SECOND_LINE_Y: i32 = 16 + LINE_HEIGHT + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardOptions {
    pub show_subtitle: bool,
    pub dimmed: bool,
}

impl Default for CardOptions {
    fn default() -> Self {
        Self {
            show_subtitle: true,
            dimmed: false,
        }
    }
}

pub(super) fn apply_dimming(root: Obj, dimmed: bool) {
    if dimmed {
        lv::style(root, lv::OPA, 128);
    }
}

#[derive(Clone, Copy)]
pub struct Choice {
    #[cfg(target_os = "emscripten")]
    card: Obj,
    on: Obj,
    off: Obj,
    thumb: Obj,
    transition: Transition,
    radio: bool,
}

const EMPTY_CHOICE: Choice = Choice {
    #[cfg(target_os = "emscripten")]
    card: NULL,
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
        text_apx(
            parent,
            value,
            x,
            y,
            width,
            height,
            color,
            font,
            CARD_TEXT_SIZE,
        )
    }
}

pub unsafe fn text_apx(
    parent: Obj,
    value: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: u32,
    font: Font,
    size: crate::Apx,
) -> Obj {
    let viewport = super::display_profile().viewport;
    let object = unsafe {
        text_font(
            parent,
            value,
            x,
            y,
            width,
            height,
            color,
            font,
            viewport.apx(size),
        )
    };
    lv::inspect_text_apx(object, viewport, size);
    object
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
        let label = pair_font(
            parent,
            title,
            subtitle,
            width,
            secondary,
            Font::Medium24,
            card_text_px(),
        );
        lv::inspect_text_apx(label, super::display_profile().viewport, CARD_TEXT_SIZE);
        label
    }
}

unsafe fn card_labels(
    card: Obj,
    title: &str,
    subtitle: &str,
    width: i32,
    secondary: u32,
    options: CardOptions,
) -> (Obj, Obj) {
    if options.show_subtitle {
        return unsafe {
            let pair = pair_labels(
                card,
                title,
                subtitle,
                width,
                secondary,
                Font::Medium24,
                card_text_px(),
            );
            lv::inspect_text_apx(pair.0, super::display_profile().viewport, CARD_TEXT_SIZE);
            pair
        };
    }
    (
        unsafe {
            text(
                card,
                title,
                22,
                (lv::CARD_HEIGHT - LINE_HEIGHT) / 2,
                width,
                LINE_HEIGHT,
                Color::PrimaryText.resolve(),
                true,
            )
        },
        NULL,
    )
}

impl Choice {
    #[deprecated(note = "Use Choice::toggle and CardOptions")]
    pub unsafe fn switch(card: Obj, title: &str, subtitle: &str, selected: bool) -> Self {
        unsafe { Self::toggle(card, title, subtitle, selected, CardOptions::default()) }
    }

    pub unsafe fn toggle(
        card: Obj,
        title: &str,
        subtitle: &str,
        checked: bool,
        options: CardOptions,
    ) -> Self {
        unsafe { Self::toggle_in(card, WIDTH, title, subtitle, checked, options) }
    }

    pub unsafe fn toggle_in(
        card: Obj,
        width: i32,
        title: &str,
        subtitle: &str,
        checked: bool,
        options: CardOptions,
    ) -> Self {
        let mut entry = Self {
            #[cfg(target_os = "emscripten")]
            card,
            transition: Transition::new(checked),
            ..EMPTY_CHOICE
        };
        unsafe {
            let (heading, detail) = card_labels(
                card,
                title,
                subtitle,
                (width - if width == WIDTH { 147 } else { 125 }).max(1),
                Color::SecondaryText.resolve(),
                options,
            );
            apply_dimming(card, options.dimmed);
            if width != WIDTH {
                truncate_labels(heading, detail);
            }
            let control = container(card, width - 99, 29, 81, 40);
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
        lv::reload_value(card, i32::from(checked));
        render_choice(&entry);
        entry
    }

    #[deprecated(note = "Use Choice::single_choice and CardOptions")]
    pub unsafe fn radio(card: Obj, title: &str, subtitle: &str, selected: bool) -> Self {
        unsafe { Self::single_choice(card, title, subtitle, selected, CardOptions::default()) }
    }

    pub unsafe fn single_choice(
        card: Obj,
        title: &str,
        subtitle: &str,
        selected: bool,
        options: CardOptions,
    ) -> Self {
        unsafe { Self::single_choice_in(card, WIDTH, title, subtitle, selected, options) }
    }

    pub unsafe fn single_choice_in(
        card: Obj,
        width: i32,
        title: &str,
        subtitle: &str,
        selected: bool,
        options: CardOptions,
    ) -> Self {
        let mut entry = Self {
            #[cfg(target_os = "emscripten")]
            card,
            transition: Transition::new(selected),
            radio: true,
            ..EMPTY_CHOICE
        };
        unsafe {
            let (heading, detail) = card_labels(
                card,
                title,
                subtitle,
                (width - 104).max(1),
                Color::SecondaryText.resolve(),
                options,
            );
            apply_dimming(card, options.dimmed);
            if width != WIDTH {
                truncate_labels(heading, detail);
            }
            entry.off = container(card, width - 58, 29, 40, 40);
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
        lv::reload_value(card, i32::from(selected));
        render_choice(&entry);
        entry
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.transition = Transition::new(selected);
        #[cfg(target_os = "emscripten")]
        lv::reload_value(self.card, i32::from(selected));
        render_choice(self);
    }

    pub fn animate_to(&mut self, selected: bool) {
        self.transition.retarget(selected, anim::now_ms());
        #[cfg(target_os = "emscripten")]
        lv::reload_value(self.card, i32::from(selected));
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

#[deprecated(note = "Use cards::switcher")]
pub unsafe fn stepper(parent: Obj, y: i32, on_step: LvxEventCallback) -> (Obj, Obj) {
    unsafe { switcher(parent, y, on_step) }
}

/// Returns the row and its current-value label. Click data 0 means previous, 1 means next.
pub unsafe fn switcher(parent: Obj, y: i32, on_step: LvxEventCallback) -> (Obj, Obj) {
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

#[deprecated(note = "Use cards::status_card_with_options and StatusCardOptions")]
pub unsafe fn status_card(parent: Obj, y: i32, title: &str, subtitle: &str) -> Obj {
    unsafe {
        legacy_status_card(
            parent,
            y,
            title,
            subtitle,
            assets::CARD_STATUS_SUCCESS,
            0xC3E5E1,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusKind {
    #[default]
    Success,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusCardStyle {
    #[default]
    Default,
    Simple,
}

pub const STATUS_CARD_HEIGHT: i32 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusCardOptions {
    pub kind: StatusKind,
    pub show_subtitle: bool,
    pub show_disclosure_indicator: bool,
}

impl Default for StatusCardOptions {
    fn default() -> Self {
        Self {
            kind: StatusKind::Success,
            show_subtitle: true,
            show_disclosure_indicator: true,
        }
    }
}

pub unsafe fn status_card_with_options(
    parent: Obj,
    y: i32,
    title: &str,
    subtitle: &str,
    options: StatusCardOptions,
) -> Obj {
    unsafe {
        status_card_with_style(
            parent,
            y,
            title,
            subtitle,
            StatusCardStyle::Default,
            options,
        )
    }
}

/// Creates any of the six built-in status card styles. The caller owns click handling.
///
/// # Safety
/// Call on the UI thread with a valid parent object.
pub unsafe fn status_card_with_style(
    parent: Obj,
    y: i32,
    title: &str,
    subtitle: &str,
    style: StatusCardStyle,
    options: StatusCardOptions,
) -> Obj {
    unsafe {
        status_card_in(
            parent,
            Rect::new(4, y, WIDTH, STATUS_CARD_HEIGHT),
            title,
            subtitle,
            style,
            options,
        )
    }
}

/// Places a status row in the available width.
pub unsafe fn status_card_in(
    parent: Obj,
    area: Rect,
    title: &str,
    subtitle: &str,
    style: StatusCardStyle,
    options: StatusCardOptions,
) -> Obj {
    let width = area.w;
    let compact = width < 260;
    let show_disclosure = options.show_disclosure_indicator && !compact;
    let use_bitmap = style == StatusCardStyle::Default && width == WIDTH;
    let (background, icon, simple_icon, simple_color, opacity) = match options.kind {
        StatusKind::Success => (
            assets::STATUS_SUCCESS_BACKGROUND,
            assets::STATUS_SUCCESS_ICON,
            assets::STATUS_SUCCESS_SIMPLE_ICON,
            0x32BE61,
            102,
        ),
        StatusKind::Warning => (
            assets::STATUS_WARNING_BACKGROUND,
            assets::STATUS_WARNING_ICON,
            assets::STATUS_WARNING_SIMPLE_ICON,
            0xE2AB21,
            89,
        ),
        StatusKind::Error => (
            assets::STATUS_ERROR_BACKGROUND,
            assets::STATUS_ERROR_ICON,
            assets::STATUS_ERROR_SIMPLE_ICON,
            0xDB5038,
            102,
        ),
    };
    unsafe {
        let card = if use_bitmap {
            lv::image(parent, background, area.x, area.y)
        } else {
            lv::background(parent, width, STATUS_CARD_HEIGHT, simple_color, 32)
        };
        if card.is_null() {
            return card;
        }
        if !use_bitmap {
            lvx_object_align(card, 1, area.x, area.y);
            lv::style(card, lv::BG_OPA, opacity);
        }
        lv::inspect_name(card, c"AerisUI.StatusCard");
        lvx_object_clear_flag(card, lv::SCROLLABLE | lv::CLICKABLE);
        let trailing = if show_disclosure { 52 } else { 0 };
        let icon_x = width - 18 - trailing - 52 - 64;
        if !compact {
            lv::image_child(card, if use_bitmap { icon } else { simple_icon }, icon_x, 0);
        }
        if show_disclosure {
            lv::image_child(
                card,
                assets::STATUS_DISCLOSURE,
                width - 58,
                (STATUS_CARD_HEIGHT - 40) / 2,
            );
        }
        const TEXT_X: i32 = 22;
        const TITLE_HEIGHT: i32 = 32;
        const SUBTITLE_HEIGHT: i32 = 68;
        const TEXT_GAP: i32 = 2;
        let show_subtitle = options.show_subtitle && !subtitle.is_empty();
        let text_height = TITLE_HEIGHT
            + if show_subtitle {
                SUBTITLE_HEIGHT + TEXT_GAP
            } else {
                0
            };
        let title_y = (STATUS_CARD_HEIGHT - text_height) / 2;
        let text_width = if compact {
            width - 2 * TEXT_X
        } else {
            icon_x - 4 - TEXT_X
        };
        let heading = text(
            card,
            title,
            TEXT_X,
            title_y,
            text_width,
            TITLE_HEIGHT,
            0xFFFFFF,
            false,
        );
        if !heading.is_null() {
            lvx_label_set_long_mode(heading, 1);
        }
        if show_subtitle {
            let detail = text(
                card,
                subtitle,
                TEXT_X,
                title_y + TITLE_HEIGHT + TEXT_GAP,
                text_width,
                SUBTITLE_HEIGHT,
                0xFFFFFF,
                false,
            );
            lv::style(detail, lv::TEXT_OPA, 179);
            if !detail.is_null() {
                lvx_label_set_long_mode(detail, 1);
            }
        }
        card
    }
}

#[deprecated(note = "Use cards::status_card_with_options for built-in status styles")]
pub unsafe fn status_card_with(
    parent: Obj,
    y: i32,
    title: &str,
    subtitle: &str,
    art: &[u8],
    secondary: u32,
) -> Obj {
    unsafe { legacy_status_card(parent, y, title, subtitle, art, secondary) }
}

unsafe fn legacy_status_card(
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JumpIndicator {
    #[default]
    Chevron,
    UpDown,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JumpCardOptions {
    pub card: CardOptions,
    pub indicator: JumpIndicator,
}

#[deprecated(note = "Use cards::jump_card_with_options and JumpCardOptions")]
pub unsafe fn jump_card(parent: Obj, y: i32, title: &str, subtitle: &str) -> Obj {
    unsafe { jump_card_with_options(parent, y, title, subtitle, JumpCardOptions::default()).0 }
}

#[deprecated(note = "Use cards::jump_card_with_options and JumpCardOptions")]
pub unsafe fn jump_card_with_detail(
    parent: Obj,
    y: i32,
    title: &str,
    subtitle: &str,
) -> (Obj, Obj) {
    unsafe { jump_card_with_options(parent, y, title, subtitle, JumpCardOptions::default()) }
}

/// Returns the card and its subtitle label, which is null when hidden.
pub unsafe fn jump_card_with_options(
    parent: Obj,
    y: i32,
    title: &str,
    subtitle: &str,
    options: JumpCardOptions,
) -> (Obj, Obj) {
    unsafe {
        jump_card_in(
            parent,
            Rect::new(4, y, WIDTH, lv::CARD_HEIGHT),
            title,
            subtitle,
            options,
        )
    }
}

/// Places a jump card within the available rectangle at its standard row height.
///
/// # Safety
/// Call on the UI thread with a live parent.
pub unsafe fn jump_card_in(
    parent: Obj,
    area: Rect,
    title: &str,
    subtitle: &str,
    options: JumpCardOptions,
) -> (Obj, Obj) {
    let indicator = match options.indicator {
        JumpIndicator::Chevron => Some(assets::CARD_CHEVRON),
        JumpIndicator::UpDown => Some(assets::CARD_UP_DOWN),
        JumpIndicator::None => None,
    };
    let trailing = if indicator.is_some() { 98 } else { 44 };
    if parent.is_null() || area.w <= trailing || area.h < lv::CARD_HEIGHT {
        return (NULL, NULL);
    }
    unsafe {
        let card = lv::card_in(parent, Rect::new(area.x, area.y, area.w, lv::CARD_HEIGHT));
        if card.is_null() {
            return (card, NULL);
        }
        lv::inspect_name(card, c"AerisUI.JumpCard");
        let (heading, detail) = card_labels(
            card,
            title,
            subtitle,
            area.w - trailing,
            Color::SecondaryText.resolve(),
            options.card,
        );
        if area.w != WIDTH {
            truncate_labels(heading, detail);
        }
        if let Some(art) = indicator {
            lv::image_child(card, art, area.w - 58, 29);
        }
        apply_dimming(card, options.card.dimmed);
        (card, detail)
    }
}

unsafe fn truncate_labels(heading: Obj, detail: Obj) {
    for label in [heading, detail]
        .into_iter()
        .filter(|label| !label.is_null())
    {
        unsafe { lvx_label_set_long_mode(label, 1) };
    }
}
