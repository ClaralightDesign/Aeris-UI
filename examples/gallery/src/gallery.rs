//! 展厅组件与示例交互。

use aeris_ui::{
    capsule::CapsuleKind,
    device::{
        anim, assets,
        cards::{self as widgets, Choice},
        checkbox::{self, Checkbox},
        confirmation_overlay, dialog,
        header::{HeaderBar, HeaderOptions, HeaderStyle},
        lv::{self, Font, Obj},
        option_grid::{self, OptionItem, OptionTitle},
        page as ui,
        page_indicator::{self, Indicator},
        progress_ring::Ring,
        rolling_number::RollingNumber,
        stepped_slider::SteppedSlider,
    },
    geometry::{Rect, Viewport},
};
use canopus_target_private::*;

use crate::{confirmation_demo, native_app};

const NULL: Obj = core::ptr::null_mut();
const CARD_STEP: i32 = 106;

const PAGE_COUNT: usize = 3;

struct Demo {
    root: Obj,
    pager: Obj,
    cards: Obj,
    width: i32,
    header: Option<HeaderBar>,
    indicator: Option<Indicator<PAGE_COUNT>>,
    rolling: Option<RollingNumber>,
    fallback: Obj,
    ring: Option<Ring>,
    sample: usize,
    ticker: Obj,
    option_label: Obj,
    choices: [Option<Choice>; 3],
    selected: [bool; 3],
    stepped_slider: Option<SteppedSlider>,
    checkboxes: [Option<Checkbox>; 3],
}

const EMPTY: Demo = Demo {
    root: NULL,
    pager: NULL,
    cards: NULL,
    width: 0,
    header: None,
    indicator: None,
    rolling: None,
    fallback: NULL,
    ring: None,
    sample: 0,
    ticker: NULL,
    option_label: NULL,
    choices: [None; 3],
    selected: [false, true, false],
    stepped_slider: None,
    checkboxes: [const { None }; 3],
};
static mut DEMO: Demo = EMPTY;

fn demo() -> &'static mut Demo {
    unsafe { &mut *core::ptr::addr_of_mut!(DEMO) }
}

pub unsafe fn create(parent: Obj) -> i32 {
    let viewport = aeris_ui::device::display_profile().viewport;
    let safe = viewport.bounds();
    unsafe {
        let root = ui::screen_in(parent, viewport);
        if root.is_null() {
            return -1;
        }
        let pager = lv::pager(root, safe.w, safe.h);
        if pager.is_null() {
            lvx_object_delete(root);
            return -1;
        }
        lvx_object_align(pager, 1, safe.x, safe.y);
        let cards = lv::page(pager, 0, safe.w, safe.h);
        let buttons = lv::page(pager, safe.w, safe.w, safe.h);
        let feedback = lv::page(pager, 2 * safe.w, safe.w, safe.h);
        if cards.is_null() || buttons.is_null() || feedback.is_null() {
            lvx_object_delete(root);
            return -1;
        }
        // 让静态卡片上的触摸命中纵向滚动页。
        lvx_object_add_flag(cards, lv::CLICKABLE);
        lvx_object_add_flag(buttons, lv::CLICKABLE);
        let Some(header) = HeaderBar::new_for_viewport(
            root,
            viewport,
            "Gallery",
            Some(cards),
            HeaderOptions {
                style: HeaderStyle::Navigation,
                large_title: true,
                on_left: Some(native_app::close_gallery),
                on_right: None,
            },
        ) else {
            lvx_object_delete(root);
            return -1;
        };
        let indicator = Indicator::new_for_viewport(root, viewport, select_page);
        for scroll in [cards, buttons, feedback] {
            ui::scroll_bars(scroll, Some(&header), Some(&indicator));
        }
        let first = 4;
        card_page(cards, safe.w, first + header.content_top());
        button_page(buttons, safe.w, first);
        feedback_page(root, feedback, viewport, first);
        demo().root = root;
        demo().pager = pager;
        demo().cards = cards;
        demo().width = safe.w;
        demo().header = Some(header);
        demo().indicator = Some(indicator);
        lvx_event_add(cards, scroll, 11, NULL);
        lvx_event_add(cards, scroll, 12, NULL);
        lvx_event_add(pager, scroll, 11, NULL);
        lvx_event_add(pager, scroll, 12, NULL);
        scroll(NULL);
    }
    0
}

unsafe fn card_page(body: Obj, width: i32, first: i32) {
    let card_width = width - 8;
    unsafe {
        lv::bind(
            widgets::jump_card_in(
                body,
                Rect::new(4, first, card_width, 98),
                if width < 240 {
                    "Inputs"
                } else {
                    "Input Controls"
                },
                "Wheel & slider",
                Default::default(),
            )
            .0,
            native_app::open_inputs,
            0,
        );
        let state = demo();
        state.stepped_slider = SteppedSlider::create_in(
            body,
            Rect::new(4, first + CARD_STEP, card_width, 98),
            3,
            2,
            set_level,
        );
        for (index, (title, subtitle)) in [
            ("Switch", "Tap to toggle"),
            ("Option A", "Radio button"),
            ("Option B", "Radio button"),
        ]
        .into_iter()
        .enumerate()
        {
            let card = lv::card_in(
                body,
                Rect::new(4, first + (index as i32 + 2) * CARD_STEP, card_width, 98),
            );
            if !card.is_null() {
                let selected = state.selected[index];
                state.choices[index] = Some(if index == 0 {
                    Choice::toggle_in(
                        card,
                        card_width,
                        title,
                        subtitle,
                        selected,
                        Default::default(),
                    )
                } else {
                    Choice::single_choice_in(
                        card,
                        card_width,
                        title,
                        subtitle,
                        selected,
                        Default::default(),
                    )
                });
                lv::bind(card, select, index);
            }
        }
        let mut y = first + 5 * CARD_STEP;
        for (index, (title, subtitle, initial)) in [
            ("Custom", "Use your own key", checkbox::State::Unchecked),
            ("Custom", "Use your own key", checkbox::State::Mixed),
            ("Official", "Free provider key", checkbox::State::Checked),
        ]
        .into_iter()
        .enumerate()
        {
            let area = Rect::new(4, y, card_width, checkbox::HEIGHT);
            let row = Checkbox::create(body, area, title, subtitle, initial);
            if let Some(row) = &row {
                lv::bind(row.root(), toggle_checkbox, index);
            }
            state.checkboxes[index] = row;
            y += checkbox::HEIGHT + 8;
        }
        for (title, subtitle, art, color, kind) in [
            (
                "Success",
                "Completed",
                assets::CARD_STATUS_SUCCESS,
                0xC3E5E1,
                widgets::StatusKind::Success,
            ),
            (
                "Warning",
                "Check input",
                assets::CARD_STATUS_WARNING,
                0xFFE0B0,
                widgets::StatusKind::Warning,
            ),
            (
                "Error",
                "Try again",
                assets::CARD_STATUS_ERROR,
                0xFFD2CE,
                widgets::StatusKind::Error,
            ),
        ] {
            let card = if card_width == widgets::WIDTH {
                widgets::status_card_with(body, y, title, subtitle, art, color)
            } else {
                widgets::status_card_in(
                    body,
                    Rect::new(4, y, card_width, widgets::STATUS_CARD_HEIGHT),
                    title,
                    subtitle,
                    widgets::StatusCardStyle::Simple,
                    widgets::StatusCardOptions {
                        kind,
                        show_disclosure_indicator: false,
                        ..Default::default()
                    },
                )
            };
            if !card.is_null() {
                lvx_object_clear_flag(card, lv::CLICKABLE);
            }
            y += if card_width == widgets::WIDTH {
                CARD_STEP
            } else {
                widgets::STATUS_CARD_HEIGHT + 8
            };
        }
        let (disabled, _) = widgets::jump_card_in(
            body,
            Rect::new(4, y, card_width, 98),
            "Navigation Card",
            "Disabled",
            widgets::JumpCardOptions {
                card: widgets::CardOptions {
                    dimmed: true,
                    ..Default::default()
                },
                indicator: widgets::JumpIndicator::None,
            },
        );
        if !disabled.is_null() {
            lvx_object_clear_flag(disabled, lv::CLICKABLE | lv::SCROLLABLE);
        }
    }
}

unsafe fn button_page(body: Obj, width: i32, first: i32) {
    unsafe {
        lv::label(
            body,
            b"Capsule Buttons\0",
            4,
            first,
            width - 8,
            32,
            0xAAAAAA,
            Font::Small24,
        );
        let mut y = first + 40;
        for (kind, label) in [
            (CapsuleKind::Regular, "Regular Button\0"),
            (CapsuleKind::Large, "Large Button\0"),
            (CapsuleKind::LargePrimary, "Primary Button\0"),
        ] {
            ui::button_in(
                body,
                Rect::new(0, y, width, kind.height()),
                label,
                kind,
                preview_button,
                0,
            );
            y += kind.height() + 8;
        }
        let top = y + 16;
        demo().option_label = lv::label(
            body,
            b"Option Grid\0",
            4,
            top,
            width - 8,
            32,
            0xAAAAAA,
            Font::Small24,
        );
        option_grid::grid_in(
            body,
            Rect::new(0, top + 40, width, 0),
            [
                OptionItem {
                    title: OptionTitle::Text(c"Eco"),
                    subtitle: c"Low power",
                    data: 0,
                },
                OptionItem {
                    title: OptionTitle::Text(c"Boost"),
                    subtitle: c"High power",
                    data: 1,
                },
                OptionItem {
                    title: OptionTitle::Number(100),
                    subtitle: c"Percent",
                    data: 2,
                },
                OptionItem {
                    title: OptionTitle::Number(50),
                    subtitle: c"Percent",
                    data: 3,
                },
            ],
            select_option,
        );
    }
}

extern "C" fn preview_button(_event: Obj) {}

extern "C" fn set_level(event: Obj) {
    if let Some(slider) = &mut demo().stepped_slider {
        if slider.handle_event(event).is_some() {
            start_ticker();
        }
    }
}

extern "C" fn toggle_checkbox(event: Obj) {
    let index = unsafe { lvx_event_get_user_data(event) };
    if let Some(Some(row)) = demo().checkboxes.get_mut(index) {
        row.animate_cycle(anim::now_ms());
        start_ticker();
    }
}

extern "C" fn select_option(event: Obj) {
    let index = unsafe { lvx_event_get_user_data(event) };
    let label = demo().option_label;
    if let Some(text) = [
        c"Eco selected",
        c"Boost selected",
        c"100% selected",
        c"50% selected",
    ]
    .get(index)
    {
        if !label.is_null() {
            unsafe { lvx_label_set_text(label, text.as_ptr().cast()) };
        }
    }
}

extern "C" fn select_page(event: Obj) {
    let index = unsafe { lvx_event_get_user_data(event) };
    let state = demo();
    let pager = state.pager;
    if !pager.is_null() {
        unsafe { lvx_object_scroll_to_x(pager, index as i32 * state.width, 1) };
    }
}

extern "C" fn scroll(_event: Obj) {
    let state = demo();
    if state.pager.is_null() {
        return;
    }
    let position = page_indicator::position_in(state.pager, PAGE_COUNT, state.width);
    if let Some(indicator) = &mut state.indicator {
        indicator.paint(position);
    }
    if let Some(header) = &state.header {
        let offset = unsafe { lvx_object_scroll_y(state.cards) };
        let page_offset = position.min(1000) * header.collapse_distance() / 1000;
        header.update_scroll(offset.max(page_offset));
    }
}

extern "C" fn select(event: Obj) {
    let index = unsafe { lvx_event_get_user_data(event) };
    if index >= 3 {
        return;
    }
    let state = demo();
    let range = if index == 0 { 0..1 } else { 1..3 };
    for i in range {
        let selected = if i == 0 {
            !state.selected[0]
        } else {
            i == index
        };
        if state.selected[i] != selected {
            state.selected[i] = selected;
            if let Some(choice) = state.choices[i].as_mut() {
                choice.animate_to(selected);
            }
        }
    }
    start_ticker();
}

fn draw(now: u32) -> bool {
    let mut moving = false;
    for choice in demo().choices.iter_mut().flatten() {
        moving |= choice.draw(now);
    }
    for row in demo().checkboxes.iter_mut().flatten() {
        moving |= row.draw(now);
    }
    if let Some(slider) = &mut demo().stepped_slider {
        moving |= slider.draw(now);
    }
    moving
        | demo()
            .rolling
            .as_mut()
            .is_some_and(|rolling| rolling.draw(now))
}

pub fn pause() {
    stop_ticker();
    let state = demo();
    confirmation_overlay::hide(state.root);
    if let Some(rolling) = &mut state.rolling {
        rolling.finish();
    }
    for (choice, selected) in state.choices.iter_mut().zip(state.selected) {
        if let Some(choice) = choice {
            choice.set_selected(selected);
        }
    }
    for row in state.checkboxes.iter_mut().flatten() {
        row.set_state(row.state());
    }
    if let Some(slider) = &mut state.stepped_slider {
        slider.finish();
    }
}

pub fn destroy() {
    stop_ticker();
    *demo() = EMPTY;
}

fn start_ticker() {
    let state = demo();
    if state.ticker.is_null() {
        state.ticker = unsafe { lvx_timer_create(tick, 16, NULL) };
        if state.ticker.is_null() {
            pause();
        }
    }
}

fn stop_ticker() {
    let state = demo();
    if !state.ticker.is_null() {
        unsafe { lvx_timer_delete(state.ticker) };
        state.ticker = NULL;
    }
}

extern "C" fn tick(_timer: Obj) {
    if !draw(anim::now_ms()) {
        stop_ticker();
    }
}

const SAMPLES: [([u8; 8], u32); 3] = [(*b"01:24:36", 96), (*b"05:18:42", 176), (*b"12:30:00", 256)];

unsafe fn feedback_page(root: Obj, body: Obj, viewport: Viewport, first: i32) {
    let width = viewport.bounds().w;
    let compact = width < 300;
    unsafe {
        let state = demo();
        lv::label(
            body,
            if compact {
                b"Digits & Progress\0"
            } else {
                b"Rolling Digits & Progress\0"
            },
            4,
            first,
            width - 8,
            32,
            0xAAAAAA,
            Font::Small24,
        );
        let (text, steps) = SAMPLES[0];
        state.rolling = RollingNumber::new(
            body,
            Rect::new(4, first + 38, if compact { width - 8 } else { 162 }, 32),
            &text,
            30,
        );
        if state.rolling.is_none() {
            state.fallback = lv::label(
                body,
                b"01:24:36\0",
                20,
                first + 38,
                (width - 24).min(210),
                40,
                0xFFFFFF,
                Font::Medium28,
            );
        }
        let base = lv::background(body, 70, 70, 0x1F1F1F, 35);
        if !base.is_null() {
            lvx_object_align(
                base,
                1,
                if compact {
                    (width - 70) / 2
                } else {
                    width - 90
                },
                first + if compact { 74 } else { 26 },
            );
            lvx_object_clear_flag(base, lv::CLICKABLE | lv::SCROLLABLE);
            state.ring = Ring::card(base);
        }
        if let Some(ring) = &mut state.ring {
            ring.set(steps);
        }
        let button_width = if compact { width } else { width / 2 };
        let button_y = first + if compact { 152 } else { 122 };
        ui::button_in(
            body,
            Rect::new(0, button_y, button_width, 82),
            "Next Sample\0",
            CapsuleKind::Large,
            next_sample,
            0,
        );
        let dialog = dialog::confirm_in(
            root,
            viewport,
            if compact {
                "Confirm\0"
            } else {
                "Confirm Selection\0"
            },
            &[if compact {
                "Apply changes?\0"
            } else {
                "Apply this selection?\0"
            }],
            "Apply\0",
            "Cancel\0",
            dialog::dismiss,
        );
        if !dialog.is_null() {
            ui::button_in(
                body,
                Rect::new(
                    if compact { 0 } else { button_width },
                    if compact { button_y + 88 } else { button_y },
                    button_width,
                    82,
                ),
                "Open Dialog\0",
                CapsuleKind::LargePrimary,
                dialog::show,
                root as usize,
            );
        }
        let confirmation_y = if compact {
            button_y + 176
        } else {
            button_y + 90
        };
        confirmation_demo::buttons(body, root, confirmation_y, width);
    }
}

extern "C" fn next_sample(_event: Obj) {
    let state = demo();
    state.sample = (state.sample + 1) % SAMPLES.len();
    let (text, steps) = SAMPLES[state.sample];
    if let Some(ring) = &mut state.ring {
        ring.set(steps);
    }
    if let Some(rolling) = &mut state.rolling {
        if rolling.set(&text, anim::now_ms(), true) == Some(true) {
            start_ticker();
        }
    } else if !state.fallback.is_null() {
        let mut label = [0; 9];
        label[..8].copy_from_slice(&text);
        unsafe { lvx_label_set_text(state.fallback, label.as_ptr()) };
    }
}
