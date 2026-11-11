//! 带滚动数字、进度环和操作按钮的计时卡。

use crate::theme::Color;
use canopus_target_private::*;

use super::{
    anim, assets,
    cards::{container, text, text_font, LINE_HEIGHT, SECOND_LINE_Y, WIDTH},
    lv::{self, Font, Obj},
    progress_ring::Ring,
    rolling_number::RollingNumber,
};

const TIMER_BUTTON_X: i32 = WIDTH - 84;
const TIMER_BUTTON_Y: i32 = 14;
const EVENT_SHORT_CLICKED: u32 = 4;
const EVENT_LONG_PRESSED: u32 = 5;
const NULL: Obj = core::ptr::null_mut();
const COUNTDOWN_PX: i32 = 30;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CardState {
    Ready,
    Paused,
    Running,
}

#[derive(Clone, Copy)]
pub struct CardActions {
    pub open: LvxEventCallback,
    pub menu: LvxEventCallback,
    pub toggle: LvxEventCallback,
    pub data: usize,
}

pub struct CountdownCard {
    pub root: Obj,
    button: Obj,
    time: Obj,
    rolling_time: Option<RollingNumber>,
    ring: Option<Ring>,
    displayed: [u8; 8],
    state: CardState,
}

impl CountdownCard {
    pub unsafe fn new(
        parent: Obj,
        y: i32,
        initial: [u8; 8],
        subtitle: &str,
        actions: CardActions,
    ) -> Option<Self> {
        unsafe {
            let card = lv::card(parent, 4, y);
            if card.is_null() {
                return None;
            }
            let width = WIDTH - 123;
            let rolling_time = RollingNumber::fixed(
                card,
                crate::geometry::Rect::new(22, 16, width, LINE_HEIGHT),
                &initial,
                COUNTDOWN_PX,
                false,
            );
            let label = if rolling_time.is_none() {
                let text = core::str::from_utf8(&initial).unwrap_or_default();
                text_font(
                    card,
                    text,
                    22,
                    16,
                    width,
                    LINE_HEIGHT,
                    Color::PrimaryText.resolve(),
                    Font::Medium28,
                    COUNTDOWN_PX,
                )
            } else {
                NULL
            };
            let subtitle = text(
                card,
                subtitle,
                22,
                SECOND_LINE_Y,
                width,
                LINE_HEIGHT,
                Color::SecondaryText.resolve(),
                false,
            );
            if !subtitle.is_null() {
                lvx_label_set_long_mode(subtitle, 1);
            }
            let button = lv::image(
                card,
                assets::CARD_TIMER_START,
                TIMER_BUTTON_X,
                TIMER_BUTTON_Y,
            );
            if button.is_null() {
                lvx_object_delete(card);
                return None;
            }
            lvx_object_clear_flag(button, lv::CLICKABLE);
            lvx_event_add(card, actions.open, EVENT_SHORT_CLICKED, actions.data as Obj);
            lvx_event_add(card, actions.menu, EVENT_LONG_PRESSED, actions.data as Obj);
            add_timer_button_hit_area(card, actions);
            Some(Self {
                root: card,
                button,
                time: label,
                rolling_time,
                ring: Ring::card(button),
                displayed: initial,
                state: CardState::Ready,
            })
        }
    }

    pub fn render(
        &mut self,
        bytes: [u8; 8],
        state: CardState,
        ring_steps: u32,
        animate_digits: bool,
    ) -> bool {
        let mut moving = false;
        if self.state != state {
            let path = match state {
                CardState::Ready => assets::CARD_TIMER_START,
                CardState::Paused => assets::CARD_TIMER_PLAY,
                CardState::Running => assets::CARD_TIMER_PAUSE,
            };
            lv::style(self.button, 28, 0);
            lv::style(
                self.button,
                lv::BG_OPA,
                if state == CardState::Ready { 0 } else { 128 },
            );
            lv::style(self.button, 12, 35);
            unsafe { lv::set_image_source(self.button, path) };
            self.state = state;
        }
        if bytes != self.displayed {
            if let Some(time) = &mut self.rolling_time {
                moving = time
                    .set(&bytes, anim::now_ms(), animate_digits)
                    .unwrap_or(false);
            } else if !self.time.is_null() {
                let mut label = [0; 9];
                label[..8].copy_from_slice(&bytes);
                unsafe { lvx_label_set_text(self.time, label.as_ptr()) };
            }
            self.displayed = bytes;
        } else if !animate_digits {
            self.finish();
        }
        if let Some(ring) = &mut self.ring {
            ring.set(if state == CardState::Ready {
                0
            } else {
                ring_steps
            });
        }
        moving
    }

    /// 设置计时按钮进度环的 RGB 色值。
    pub fn set_ring_color(&self, color: u32) {
        if let Some(ring) = &self.ring {
            ring.set_color(color);
        }
    }

    pub fn draw(&mut self, now: u32) -> bool {
        self.rolling_time
            .as_mut()
            .is_some_and(|time| time.draw(now))
    }

    pub fn finish(&mut self) {
        if let Some(time) = &mut self.rolling_time {
            time.finish();
        }
    }
}
unsafe fn add_timer_button_hit_area(card: Obj, actions: CardActions) {
    const BANDS: [(i32, i32); 13] = [
        (2, 20),
        (4, 30),
        (4, 42),
        (6, 52),
        (6, 60),
        (8, 66),
        (8, 68),
        (8, 66),
        (6, 60),
        (6, 52),
        (4, 42),
        (4, 30),
        (2, 20),
    ];
    let mut y = TIMER_BUTTON_Y + 1;
    for (height, width) in BANDS {
        let hit = unsafe { container(card, TIMER_BUTTON_X + (70 - width) / 2, y, width, height) };
        if !hit.is_null() {
            unsafe {
                lvx_object_add_flag(hit, lv::CLICKABLE);
                lvx_event_add(
                    hit,
                    actions.toggle,
                    EVENT_SHORT_CLICKED,
                    actions.data as Obj,
                );
                lvx_event_add(hit, actions.menu, EVENT_LONG_PRESSED, actions.data as Obj);
            }
        }
        y += height;
    }
}
