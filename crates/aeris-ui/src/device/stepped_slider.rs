//! 使用卡片底图的分级滑块。

use canopus_target_private::*;

use crate::theme::Color;

use super::{
    anim, assets, cards,
    lv::{self, Obj},
};

pub const WIDTH: i32 = cards::WIDTH;
pub const HEIGHT: i32 = lv::CARD_HEIGHT;
pub const MAX_STEPS: u8 = 10;

const BUTTON_WIDTH: i32 = 70;
const ICON_SIZE: i32 = 40;
const TRACK_INSET: i32 = 14;
const TRACK_WIDTH: i32 = WIDTH - 2 * (BUTTON_WIDTH + TRACK_INSET);
const TRACK_HEIGHT: i32 = 10;
const TRACK_Y: i32 = (HEIGHT - TRACK_HEIGHT) / 2;
const TRACK_DIVIDER: u32 = 0x3E3E3E;
const TRANSITION_MS: u32 = 180;
const EVENT_PRESSED: u32 = 1;
const EVENT_PRESSING: u32 = 2;
const PRESS_LOCK: u32 = 1 << 13;
const DECREASE: usize = 0;
const INCREASE: usize = 1;
const TRACK: usize = 2;

pub struct SteppedSlider {
    center: Obj,
    fill_clip: Obj,
    minus: Obj,
    plus: Obj,
    steps: u8,
    level: u8,
    fill_width: i32,
    transition: Option<(i32, u32)>,
}

impl SteppedSlider {
    /// 父对象须在滑块使用期间有效，事件须在 UI 线程转交给 `handle_event`。
    pub unsafe fn create(
        parent: Obj,
        x: i32,
        y: i32,
        steps: u8,
        initial: u8,
        on_input: LvxEventCallback,
    ) -> Option<Self> {
        if !(1..=MAX_STEPS).contains(&steps) || parent.is_null() {
            return None;
        }
        unsafe {
            let root = lv::image(parent, assets::CARD_FACE, x, y);
            if root.is_null() {
                return None;
            }
            lvx_object_clear_flag(root, lv::CLICKABLE | lv::SCROLLABLE);
            let slider = Self::mount(root, steps, initial.min(steps), on_input);
            if slider.is_none() {
                lvx_object_delete(root);
            }
            slider
        }
    }

    unsafe fn mount(root: Obj, steps: u8, level: u8, on_input: LvxEventCallback) -> Option<Self> {
        unsafe {
            let divider = Color::Background.resolve();
            let buttons = [
                (0, assets::SLIDER_MINUS, DECREASE),
                (WIDTH - BUTTON_WIDTH, assets::SLIDER_PLUS, INCREASE),
            ];
            let mut icons = [core::ptr::null_mut(); 2];
            for (x, art, action) in buttons {
                let hit = cards::container(root, x, 0, BUTTON_WIDTH, HEIGHT);
                if hit.is_null() {
                    return None;
                }
                let icon = lv::image(
                    hit,
                    art,
                    (BUTTON_WIDTH - ICON_SIZE) / 2,
                    (HEIGHT - ICON_SIZE) / 2,
                );
                if icon.is_null() {
                    return None;
                }
                lvx_object_clear_flag(icon, lv::CLICKABLE | lv::SCROLLABLE);
                lvx_object_add_flag(hit, lv::CLICKABLE);
                icons[action] = icon;
                lv::bind(hit, on_input, action);
            }
            for x in [BUTTON_WIDTH - 1, WIDTH - BUTTON_WIDTH - 1] {
                surface(root, x, 0, 2, HEIGHT, divider, 0)?;
            }

            let center = cards::container(root, BUTTON_WIDTH, 0, WIDTH - 2 * BUTTON_WIDTH, HEIGHT);
            if center.is_null() {
                return None;
            }
            let radius = (TRACK_HEIGHT / 2) as u32;
            let track = surface(
                center,
                TRACK_INSET,
                TRACK_Y,
                TRACK_WIDTH,
                TRACK_HEIGHT,
                Color::SecondaryText.resolve(),
                radius,
            )?;
            lv::style(track, lv::BG_OPA, 90);
            // 固定填充的圆角，仅用矩形窗口裁切，避免逐帧创建圆角遮罩层。
            let fill_clip = cards::container(track, 0, 0, 1, TRACK_HEIGHT);
            if fill_clip.is_null() {
                return None;
            }
            surface(
                fill_clip,
                0,
                0,
                TRACK_WIDTH,
                TRACK_HEIGHT,
                Color::Accent.resolve(),
                radius,
            )?;
            lvx_set_hidden(fill_clip, 1);
            for step in 1..steps {
                let x = TRACK_WIDTH * i32::from(step) / i32::from(steps);
                surface(track, x - 1, 0, 2, TRACK_HEIGHT, TRACK_DIVIDER, 0)?;
            }
            lvx_object_clear_flag(center, lv::SCROLL_CHAIN | lv::GESTURE_BUBBLE);
            lvx_object_add_flag(center, lv::CLICKABLE | PRESS_LOCK);
            for code in [EVENT_PRESSED, EVENT_PRESSING] {
                lvx_event_add(center, on_input, code, TRACK as Obj);
            }
            let [minus, plus] = icons;
            let mut slider = Self {
                center,
                fill_clip,
                minus,
                plus,
                steps,
                level,
                fill_width: 0,
                transition: None,
            };
            slider.finish();
            slider.paint_buttons();
            Some(slider)
        }
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    /// 将档位限制在 `0..=steps`，返回实际变更后的档位。
    pub fn set_level(&mut self, level: u8) -> Option<u8> {
        let level = level.min(self.steps);
        if self.level == level {
            return None;
        }
        self.level = level;
        self.finish();
        self.paint_buttons();
        Some(level)
    }

    /// 转交组件的 LVGL 事件，档位变化时返回新值。
    pub fn handle_event(&mut self, event: Obj) -> Option<u8> {
        if event.is_null() {
            return None;
        }
        match unsafe { lvx_event_get_user_data(event) } {
            DECREASE => self.animate_to(self.level.saturating_sub(1)),
            INCREASE => self.animate_to((self.level + 1).min(self.steps)),
            TRACK => {
                let [x, _] = lv::touch_point(event)?;
                let [left, ..] = unsafe { lvx_object_coords(self.center) };
                let offset = (x - left - TRACK_INSET).clamp(0, TRACK_WIDTH);
                let level = (offset * i32::from(self.steps) + TRACK_WIDTH / 2) / TRACK_WIDTH;
                self.set_level(level as u8)
            }
            _ => None,
        }
    }

    fn animate_to(&mut self, level: u8) -> Option<u8> {
        if self.level == level {
            return None;
        }
        self.level = level;
        self.paint_buttons();
        self.transition =
            (self.fill_width != self.target_width()).then(|| (self.fill_width, anim::now_ms()));
        Some(level)
    }

    /// 推进档位动画，返回是否仍需下一帧。
    pub fn draw(&mut self, now: u32) -> bool {
        let Some((from, started)) = self.transition else {
            return false;
        };
        let elapsed = now.wrapping_sub(started);
        if elapsed >= TRANSITION_MS {
            self.finish();
            return false;
        }
        let remaining = (TRANSITION_MS - elapsed) as i32;
        let duration_squared = (TRANSITION_MS * TRANSITION_MS) as i32;
        let progress = duration_squared - remaining * remaining;
        self.paint_fill(from + (self.target_width() - from) * progress / duration_squared);
        true
    }

    pub fn finish(&mut self) {
        self.transition = None;
        self.paint_fill(self.target_width());
    }

    fn target_width(&self) -> i32 {
        TRACK_WIDTH * i32::from(self.level) / i32::from(self.steps)
    }

    fn paint_fill(&mut self, width: i32) {
        if self.fill_width == width {
            return;
        }
        unsafe {
            if width > 0 {
                lvx_object_set_size(self.fill_clip, width, TRACK_HEIGHT);
            }
            if (self.fill_width == 0) != (width == 0) {
                lvx_set_hidden(self.fill_clip, u32::from(width == 0));
            }
        }
        self.fill_width = width;
    }

    fn paint_buttons(&self) {
        lv::style(self.minus, lv::OPA, if self.level == 0 { 102 } else { 255 });
        lv::style(
            self.plus,
            lv::OPA,
            if self.level == self.steps { 102 } else { 255 },
        );
    }
}

unsafe fn surface(
    parent: Obj,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
    radius: u32,
) -> Option<Obj> {
    unsafe {
        let object = lv::background(parent, w, h, color, radius);
        if object.is_null() {
            return None;
        }
        lvx_object_align(object, 1, x, y);
        lvx_object_clear_flag(object, lv::CLICKABLE | lv::SCROLLABLE);
        Some(object)
    }
}
