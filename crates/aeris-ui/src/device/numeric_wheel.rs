//! 可配置范围的多列数字滚轮。

use core::cell::Cell;

use crate::{
    font::DetailLayout,
    geometry::Rect,
    numeric_wheel::{
        centered_position, columns, nearest_step, value_at, CENTER, COLUMN_WIDTH, HEIGHT, STEP,
    },
};
use canopus_target_private::*;

use super::{
    assets, font,
    lv::{self, Font, Obj},
};

const NULL: Obj = core::ptr::null_mut();
const WINDOW_HEIGHT: i32 = 158;
const SCROLL_RANGE: i32 = 2 * CENTER;
fn digit_px() -> i32 {
    super::display_profile()
        .viewport
        .apx(crate::Apx::from_halves(19))
}
const TRACK_Y: i32 = (WINDOW_HEIGHT - STEP) / 2 - STEP;
const GREEN: u32 = 0x4CE045;
const BORDER_COLOR: u32 = 49;
const SCROLL_ELASTIC: u32 = 1 << 5;
const SCROLL_MOMENTUM: u32 = 1 << 6;
const EVENT_PRESSED: u32 = 1;
const EVENT_SCROLL_END: u32 = 11;
const EVENT_SCROLL: u32 = 12;

#[derive(Clone, Copy)]
pub struct Column {
    pub min: u8,
    pub max: u8,
    pub initial: u8,
    pub label: &'static [u8],
    pub badge_width: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NumericWheelStyle {
    #[default]
    WithUnit,
    NumberOnly,
}

pub struct NumericWheel<const N: usize> {
    wheels: [Wheel; N],
    active: Cell<usize>,
    resetting: Cell<bool>,
}

struct Wheel {
    frame: Obj,
    badge: Obj,
    scroll: Obj,
    track: Obj,
    rows: [Row; 3],
    offset: Cell<i32>,
    settling: Cell<bool>,
    spec: Column,
}

struct Row {
    digits: [Obj; 2],
    label: Obj,
    value: Cell<i32>,
    width: i32,
}

impl Row {
    unsafe fn create(parent: Obj, y: i32, width: i32) -> Option<Self> {
        unsafe {
            let root = font::clip(parent, width, STEP, 0)?;
            lvx_object_align(root, 1, 0, y);
            let mut digits = [NULL; 2];
            let mut label = NULL;
            if assets::fonts_ready() {
                for digit in &mut digits {
                    *digit = lvx_image_create(root);
                    if digit.is_null() {
                        return None;
                    }
                    lvx_object_clear_flag(*digit, lv::CLICKABLE | lv::SCROLLABLE);
                }
            } else {
                label = lv::label(
                    root,
                    b"00\0",
                    0,
                    (STEP - 32) / 2,
                    width,
                    32,
                    0xFFFFFF,
                    Font::Demibold32,
                );
                if label.is_null() {
                    return None;
                }
                let base = Font::Demibold32.base_px();
                lv::text_px(label, width, 32, 2, Font::Demibold32, base);
                lv::inspect_text_apx(
                    label,
                    super::display_profile().viewport,
                    crate::Apx::from_halves(19),
                );
                let scale = ((digit_px() * 256 + base / 2) / base) as u32;
                for (property, value) in [
                    (lv::TRANSFORM_PIVOT_X, width as u32 / 2),
                    (lv::TRANSFORM_PIVOT_Y, 16),
                    (lv::TRANSFORM_SCALE_X, scale),
                    (lv::TRANSFORM_SCALE_Y, scale),
                ] {
                    lv::style(label, property, value);
                }
            }
            Some(Self {
                digits,
                label,
                value: Cell::new(-1),
                width,
            })
        }
    }

    fn paint(&self, value: i32) {
        let previous = self.value.replace(value);
        if previous != value {
            let text = [b'0' + (value / 10) as u8, b'0' + (value % 10) as u8, 0];
            unsafe {
                if !self.label.is_null() {
                    lvx_label_set_text(self.label, text.as_ptr());
                } else if let Some(layout) = DetailLayout::sized(&text[..2], digit_px()) {
                    for ((image, glyph), place) in self
                        .digits
                        .into_iter()
                        .zip(layout.glyphs.into_iter().flatten())
                        .zip([10, 1])
                    {
                        if previous < 0 || previous / place % 10 != value / place % 10 {
                            lv::set_image_source(image, font::detail_source(glyph));
                            lvx_image_set_scale(image, glyph.scale, glyph.scale);
                        }
                        let (source_w, source_h) = glyph.source_size();
                        let (w, h) = glyph.size();
                        lvx_object_align(
                            image,
                            1,
                            self.width / 2 + glyph.x + (w - source_w).div_euclid(2),
                            STEP / 2 + glyph.y + (h - source_h).div_euclid(2),
                        );
                    }
                }
            }
        }
    }
}

impl Wheel {
    unsafe fn create(
        parent: Obj,
        area: Rect,
        spec: Column,
        style: NumericWheelStyle,
        callback: LvxEventCallback,
    ) -> Option<Self> {
        let Rect { x, y, w: width, .. } = area;
        let window_y = y + match style {
            NumericWheelStyle::WithUnit => HEIGHT - WINDOW_HEIGHT,
            NumericWheelStyle::NumberOnly => (HEIGHT - WINDOW_HEIGHT) / 2,
        };
        unsafe {
            let badge = if style == NumericWheelStyle::WithUnit {
                let width_of_badge = spec.badge_width.clamp(1, width);
                let badge = lv::background(parent, width_of_badge, 36, GREEN, 10);
                if !badge.is_null() {
                    lvx_object_align(badge, 1, x + (width - width_of_badge) / 2, y);
                    lvx_object_clear_flag(badge, lv::CLICKABLE | lv::SCROLLABLE);
                    lv::label(
                        badge,
                        spec.label,
                        0,
                        2,
                        width_of_badge,
                        32,
                        0,
                        Font::Medium24,
                    );
                }
                badge
            } else {
                NULL
            };
            let viewport = font::clip(parent, width, WINDOW_HEIGHT, 22)?;
            lvx_object_align(viewport, 1, x, window_y);
            lv::style(viewport, LV_STYLE_CLIP_CORNER, 1);
            let track = font::clip(viewport, width, STEP * 3, 0)?;
            lvx_object_align(track, 1, 0, TRACK_Y);
            let rows = [
                Row::create(track, 0, width)?,
                Row::create(track, STEP, width)?,
                Row::create(track, STEP * 2, width)?,
            ];
            let shade = lv::image(parent, assets::EDITOR_WHEEL_SHADE, x, window_y);
            if !shade.is_null() {
                lvx_object_clear_flag(shade, lv::CLICKABLE | lv::SCROLLABLE);
                if width != COLUMN_WIDTH {
                    lv::style(shade, lv::TRANSFORM_PIVOT_X, 0);
                    lv::style(shade, lv::TRANSFORM_PIVOT_Y, 0);
                    lvx_image_set_scale(shade, width * 256 / COLUMN_WIDTH, 256);
                }
            }
            let frame = font::clip(parent, width, WINDOW_HEIGHT, 22)?;
            lvx_object_align(frame, 1, x, window_y);
            lv::style(frame, LV_STYLE_BORDER_WIDTH, 4);
            lv::style(frame, LV_STYLE_BORDER_OPA, 255);
            // 触摸层独立滚动，数字和边框留在固定的裁剪区域。
            let scroll = font::clip(parent, width, WINDOW_HEIGHT, 0)?;
            lvx_object_align(scroll, 1, x, window_y);
            for property in [16, 17, 18, 19] {
                lv::style(scroll, property, 0);
            }
            let spacer = font::clip(scroll, 1, WINDOW_HEIGHT + SCROLL_RANGE, 0)?;
            lvx_object_align(spacer, 1, 0, 0);
            lvx_object_add_flag(scroll, lv::CLICKABLE | lv::SCROLLABLE | SCROLL_MOMENTUM);
            lvx_object_clear_flag(scroll, SCROLL_ELASTIC);
            lvx_object_set_scroll_dir(scroll, 12);
            lvx_object_set_scrollbar_mode(scroll, 0);
            for code in [EVENT_PRESSED, EVENT_SCROLL, EVENT_SCROLL_END] {
                lvx_event_add(scroll, callback, code, scroll);
            }
            Some(Self {
                frame,
                badge,
                scroll,
                track,
                rows,
                offset: Cell::new(i32::MAX),
                settling: Cell::new(false),
                spec,
            })
        }
    }

    fn select(&self, active: bool) {
        lv::style(
            self.frame,
            BORDER_COLOR,
            if active { GREEN } else { 0xBFBFBF },
        );
        if !self.badge.is_null() {
            unsafe { lvx_set_hidden(self.badge, u32::from(!active)) };
        }
    }

    fn paint(&self, position: i32) {
        let center = nearest_step(position);
        let offset = center * STEP - position;
        if self.offset.replace(offset) != offset {
            lv::style(self.track, lv::TRANSLATE_Y, offset as u32);
        }
        for (row, relative) in self.rows.iter().zip(-1..=1) {
            row.paint(i32::from(value_at(
                center + relative,
                self.spec.min,
                self.spec.max,
            )));
        }
    }

    fn position(&self) -> i32 {
        unsafe { lvx_object_scroll_y(self.scroll) }
    }

    fn value(&self) -> u8 {
        value_at(nearest_step(self.position()), self.spec.min, self.spec.max)
    }
}

impl<const N: usize> NumericWheel<N> {
    /// 回调将收到的事件转交给同一实例；父对象须在实例使用期间有效。
    #[deprecated(note = "Use NumericWheel::mount_with_styles")]
    pub unsafe fn mount(
        parent: Obj,
        bounds: Rect,
        specs: [Column; N],
        active: usize,
        callback: LvxEventCallback,
    ) -> Option<Self> {
        unsafe {
            Self::mount_with_styles(
                parent,
                bounds,
                specs,
                [NumericWheelStyle::WithUnit; N],
                active,
                callback,
            )
        }
    }

    pub unsafe fn mount_with_styles(
        parent: Obj,
        bounds: Rect,
        specs: [Column; N],
        styles: [NumericWheelStyle; N],
        active: usize,
        callback: LvxEventCallback,
    ) -> Option<Self> {
        let areas = columns::<N>(Rect::new(0, 0, bounds.w, bounds.h))?;
        if active >= N
            || specs.iter().zip(styles).any(|(spec, style)| {
                spec.min > spec.max
                    || spec.max > 99
                    || (style == NumericWheelStyle::WithUnit && spec.label.last() != Some(&0))
            })
        {
            return None;
        }
        let root = unsafe { font::clip(parent, bounds.w, HEIGHT, 0) }?;
        unsafe { lv::inspect_name(root, c"AerisUI.NumericWheel") };
        unsafe { lvx_object_align(root, 1, bounds.x, bounds.y) };
        let mut wheels: [Option<Wheel>; N] = core::array::from_fn(|_| None);
        for (index, ((area, spec), style)) in areas.into_iter().zip(specs).zip(styles).enumerate() {
            let Some(wheel) = (unsafe { Wheel::create(root, area, spec, style, callback) }) else {
                unsafe { lvx_object_delete(root) };
                return None;
            };
            wheel.select(index == active);
            wheels[index] = Some(wheel);
        }
        let wheel = Self {
            wheels: wheels.map(Option::unwrap),
            active: Cell::new(active),
            resetting: Cell::new(false),
        };
        wheel.set(specs.map(|spec| spec.initial));
        Some(wheel)
    }

    pub fn set(&self, values: [u8; N]) {
        for (wheel, value) in self.wheels.iter().zip(values) {
            self.reset_scroll(wheel, value);
        }
    }

    pub fn finish(&self) -> [u8; N] {
        self.wheels.each_ref().map(|wheel| {
            let value = wheel.value();
            self.reset_scroll(wheel, value);
            value
        })
    }

    pub fn values(&self) -> [u8; N] {
        self.wheels.each_ref().map(Wheel::value)
    }

    pub fn handle_event(&self, event: Obj) {
        if event.is_null() || self.resetting.get() {
            return;
        }
        let scroll = unsafe { lvx_event_get_user_data(event) } as Obj;
        let Some(index) = self.wheels.iter().position(|wheel| wheel.scroll == scroll) else {
            return;
        };
        let wheel = &self.wheels[index];
        match unsafe { lvx_event_get_code(event) } {
            EVENT_PRESSED => {
                let previous = self.active.replace(index);
                if previous != index {
                    self.wheels[previous].select(false);
                    wheel.select(true);
                }
            }
            EVENT_SCROLL => wheel.paint(wheel.position()),
            EVENT_SCROLL_END => {
                let position = wheel.position();
                let target = nearest_step(position) * STEP;
                let settling = wheel.settling.replace(false);
                if target != position && !settling {
                    wheel.settling.set(true);
                    unsafe { lvx_object_scroll_by(scroll, 0, position - target, 1) };
                } else if target == position
                    && !(CENTER / 2..SCROLL_RANGE - CENTER / 2).contains(&position)
                {
                    self.reset_scroll(wheel, wheel.value());
                }
            }
            _ => {}
        }
    }

    fn reset_scroll(&self, wheel: &Wheel, value: u8) {
        self.resetting.set(true);
        wheel.settling.set(false);
        let position = centered_position(value, wheel.spec.min, wheel.spec.max);
        #[cfg(all(feature = "band-11", not(target_arch = "wasm32")))]
        unsafe {
            lvx_object_scroll_to_y(wheel.scroll, position, 0)
        };
        #[cfg(any(not(feature = "band-11"), target_arch = "wasm32"))]
        unsafe {
            generated_symbols::raw_lv_obj_scroll_to(wheel.scroll, 0, position, 0)
        };
        self.resetting.set(false);
        wheel.paint(wheel.position());
    }
}
