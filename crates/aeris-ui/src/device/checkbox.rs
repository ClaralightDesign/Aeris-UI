//! 支持未选、部分选中和已选状态的复选卡片。

use canopus_target_private::*;

use crate::{card::TRANSITION_MS, geometry::Rect, motion::ease_out_cubic, theme::Color};

use super::{
    assets, cards,
    lv::{self, Font, Obj},
};

pub const HEIGHT: i32 = 100;
const CARD_RADIUS: u32 = 32;
const INDICATOR_SIZE: i32 = 40;
const INDICATOR_RADIUS: u32 = 10;
const ICON_SIZE: i32 = 32;
const ICON_INSET: i32 = (INDICATOR_SIZE - ICON_SIZE) / 2;
const SIDE_INSET: i32 = 18;
const TEXT_INSET: i32 = SIDE_INSET + 4;
const TEXT_GAP: i32 = 2;
const CONTROL_GAP: i32 = 12;
const MIN_WIDTH: i32 = TEXT_INSET + 4 + CONTROL_GAP + INDICATOR_SIZE + SIDE_INSET + 1;
const STOCK_ACCENT: u32 = 0x0088FF;
const SCALE_FULL: i32 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Unchecked,
    Mixed,
    Checked,
}

impl State {
    fn toggled(self) -> Self {
        if self == Self::Checked {
            Self::Unchecked
        } else {
            Self::Checked
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Appearance {
    premultiplied_rgb: [u32; 3],
    opacity: u32,
    icon_scales: [i32; 2],
}

impl Appearance {
    fn for_state(state: State) -> Self {
        let accent = Color::Accent.resolve_or(STOCK_ACCENT);
        let (color, opacity) = match state {
            State::Unchecked => (Color::PrimaryText.resolve(), 26),
            State::Mixed => (accent, 128),
            State::Checked => (accent, 255),
        };
        let [_, red, green, blue] = color.to_be_bytes();
        Self {
            premultiplied_rgb: [red, green, blue].map(|channel| u32::from(channel) * opacity),
            opacity,
            icon_scales: [State::Mixed, State::Checked].map(|shown| {
                if shown == state {
                    SCALE_FULL
                } else {
                    0
                }
            }),
        }
    }

    fn blend(self, target: Self, progress: u32) -> Self {
        let mix = |from, to| (from * (1000 - progress) + to * progress + 500) / 1000;
        Self {
            premultiplied_rgb: core::array::from_fn(|index| {
                mix(
                    self.premultiplied_rgb[index],
                    target.premultiplied_rgb[index],
                )
            }),
            opacity: mix(self.opacity, target.opacity),
            icon_scales: core::array::from_fn(|index| {
                mix(
                    self.icon_scales[index] as u32,
                    target.icon_scales[index] as u32,
                ) as i32
            }),
        }
    }
}

struct Motion {
    from: Appearance,
    to: Appearance,
    started: u32,
}

pub struct Checkbox {
    root: Obj,
    background: Obj,
    icons: [Obj; 2],
    state: State,
    appearance: Appearance,
    motion: Option<Motion>,
}

impl Checkbox {
    /// 在有效的父对象中创建卡片，父对象负责销毁其子对象。
    pub unsafe fn create(
        parent: Obj,
        area: Rect,
        title: &str,
        subtitle: &str,
        state: State,
    ) -> Option<Self> {
        if parent.is_null() || area.w < MIN_WIDTH || area.h < HEIGHT {
            return None;
        }

        let indicator_x = area.w - SIDE_INSET - INDICATOR_SIZE;
        let text_width = indicator_x - CONTROL_GAP - 4 - TEXT_INSET;
        unsafe {
            let primary = Color::PrimaryText.resolve();
            let root = lv::background(parent, area.w, area.h, primary, CARD_RADIUS);
            if root.is_null() {
                return None;
            }
            lv::inspect_name(root, c"AerisUI.Checkbox");
            lvx_object_align(root, 1, area.x, area.y);
            lvx_object_clear_flag(root, lv::SCROLLABLE);
            lvx_object_add_flag(root, lv::CLICKABLE);
            lv::style(root, lv::BG_OPA, 31);

            let text_y = (area.h - 2 * cards::LINE_HEIGHT - TEXT_GAP) / 2;
            let title = cards::text_font(
                root,
                title,
                TEXT_INSET,
                text_y,
                text_width,
                cards::LINE_HEIGHT,
                primary,
                Font::Medium24,
                cards::CARD_TEXT_PX,
            );
            let subtitle = cards::text_font(
                root,
                subtitle,
                TEXT_INSET,
                text_y + cards::LINE_HEIGHT + TEXT_GAP,
                text_width,
                cards::LINE_HEIGHT,
                Color::SecondaryText.resolve_or(primary),
                Font::Small24,
                cards::CARD_TEXT_PX,
            );
            let background = lv::background(
                root,
                INDICATOR_SIZE,
                INDICATOR_SIZE,
                primary,
                INDICATOR_RADIUS,
            );
            if [title, subtitle, background]
                .into_iter()
                .any(|object| object.is_null())
            {
                lvx_object_delete(root);
                return None;
            }
            lv::style(subtitle, lv::TEXT_OPA, 140);
            lvx_label_set_long_mode(title, 1);
            lvx_label_set_long_mode(subtitle, 1);
            let indicator_y = (area.h - INDICATOR_SIZE) / 2;
            lvx_object_align(background, 1, indicator_x, indicator_y);
            lvx_object_clear_flag(background, lv::CLICKABLE | lv::SCROLLABLE);

            // 图标与底板同级，缩放只作用于图标。
            let icons = [assets::CHECKBOX_MINUS, assets::CHECKBOX_CHECK].map(|art| {
                lv::image(
                    root,
                    art,
                    indicator_x + ICON_INSET,
                    indicator_y + ICON_INSET,
                )
            });
            if icons.iter().any(|icon| icon.is_null()) {
                lvx_object_delete(root);
                return None;
            }
            for icon in icons {
                lvx_object_clear_flag(icon, lv::CLICKABLE | lv::SCROLLABLE);
                lv::style(icon, lv::TRANSFORM_PIVOT_X, (ICON_SIZE / 2) as u32);
                lv::style(icon, lv::TRANSFORM_PIVOT_Y, (ICON_SIZE / 2) as u32);
            }
            let checkbox = Self {
                root,
                background,
                icons,
                state,
                appearance: Appearance::for_state(state),
                motion: None,
            };
            checkbox.paint();
            Some(checkbox)
        }
    }

    pub fn root(&self) -> Obj {
        self.root
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
        self.motion = None;
        self.appearance = Appearance::for_state(state);
        self.paint();
    }

    /// 点按时切换选中状态，部分选中会变为已选。
    pub fn toggle(&mut self) -> State {
        self.set_state(self.state.toggled());
        self.state
    }

    /// 从当前画面过渡到目标状态。
    pub fn animate_to(&mut self, state: State, now: u32) {
        if self.state == state {
            return;
        }
        self.draw(now);
        self.state = state;
        let to = Appearance::for_state(state);
        self.motion = (self.appearance != to).then_some(Motion {
            from: self.appearance,
            to,
            started: now,
        });
    }

    /// 带动画地切换选中状态，部分选中会变为已选。
    pub fn animate_toggle(&mut self, now: u32) -> State {
        self.animate_to(self.state.toggled(), now);
        self.state
    }

    /// 按未选、部分选中、已选的顺序循环，并返回目标状态。
    pub fn animate_cycle(&mut self, now: u32) -> State {
        let next = match self.state {
            State::Unchecked => State::Mixed,
            State::Mixed => State::Checked,
            State::Checked => State::Unchecked,
        };
        self.animate_to(next, now);
        next
    }

    /// 刷新动画，返回是否仍需下一帧。
    pub fn draw(&mut self, now: u32) -> bool {
        let Some(motion) = &self.motion else {
            return false;
        };
        let elapsed = now.wrapping_sub(motion.started).min(TRANSITION_MS);
        let progress = ease_out_cubic((elapsed * 1000 / TRANSITION_MS) as i32);
        let appearance = motion.from.blend(motion.to, progress as u32);
        if appearance != self.appearance {
            self.appearance = appearance;
            self.paint();
        }
        if elapsed == TRANSITION_MS {
            self.motion = None;
        }
        self.motion.is_some()
    }

    fn paint(&self) {
        let opacity = self.appearance.opacity;
        // 先插值预乘颜色，再还原色值，避免低透明度底色在过渡中变亮。
        let [red, green, blue] = self
            .appearance
            .premultiplied_rgb
            .map(|channel| ((channel + opacity / 2) / opacity).min(255));
        lv::style(
            self.background,
            lv::BG_COLOR,
            (red << 16) | (green << 8) | blue,
        );
        lv::style(self.background, lv::BG_OPA, opacity);
        for (icon, scale) in self.icons.into_iter().zip(self.appearance.icon_scales) {
            unsafe {
                lvx_set_hidden(icon, u32::from(scale == 0));
                if scale != 0 {
                    lvx_image_set_scale(icon, scale, scale);
                }
            }
        }
    }
}
