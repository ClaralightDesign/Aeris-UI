//! 可放入指定区域的滑动确认控件。

use canopus_target_private::*;

use crate::{
    confirm_slider::{Drag, SliderLayout},
    geometry::Rect,
};

use super::{
    assets, cards,
    lv::{self, Obj},
    page,
};

const EVENT_PRESSED: u32 = 1;
const EVENT_PRESSING: u32 = 2;
const EVENT_PRESS_LOST: u32 = 3;
const EVENT_RELEASED: u32 = 8;
const PRESS_LOCK: u32 = 1 << 13;
const TRACK_WIDTH: i32 = 328;

pub const STOCK_BOUNDS: Rect = Rect::new(0, 388, page::WIDTH, 88);

pub struct Slider {
    thumb: Obj,
    caption: Obj,
    layout: SliderLayout,
    drag: Drag,
}

impl Slider {
    /// 素材须为 NUL 结尾的静态资源路径；父对象须在控件使用期间有效。
    pub unsafe fn create(
        parent: Obj,
        area: Rect,
        thumb_art: &'static [u8],
        caption: impl FnOnce(Obj, Rect) -> Obj,
        on_drag: LvxEventCallback,
    ) -> Option<Self> {
        let layout = SliderLayout::measure(area)?;
        unsafe {
            let track = lv::image(parent, assets::STOP_TRACK, layout.track.x, layout.track.y);
            if track.is_null() {
                return None;
            }
            lvx_object_clear_flag(track, lv::CLICKABLE | lv::SCROLLABLE);
            if layout.track.w != TRACK_WIDTH {
                lv::style(track, lv::TRANSFORM_PIVOT_X, 0);
                lv::style(track, lv::TRANSFORM_PIVOT_Y, 0);
                lvx_image_set_scale(track, layout.track.w * 256 / TRACK_WIDTH, 256);
            }
            let caption = caption(parent, layout.caption);
            let thumb = lv::image(parent, thumb_art, layout.thumb.x, layout.thumb.y);
            if caption.is_null() || thumb.is_null() {
                for object in [track, caption, thumb] {
                    if !object.is_null() {
                        lvx_object_delete(object);
                    }
                }
                return None;
            }
            lvx_object_clear_flag(caption, lv::CLICKABLE | lv::SCROLLABLE);
            lvx_object_clear_flag(
                thumb,
                lv::SCROLLABLE | lv::SCROLL_CHAIN | lv::GESTURE_BUBBLE,
            );
            lvx_object_add_flag(thumb, lv::CLICKABLE | PRESS_LOCK);
            for event in [
                EVENT_PRESSED,
                EVENT_PRESSING,
                EVENT_PRESS_LOST,
                EVENT_RELEASED,
            ] {
                lvx_event_add(thumb, on_drag, event, core::ptr::null_mut());
            }
            Some(Self {
                thumb,
                caption,
                layout,
                drag: Drag::default(),
            })
        }
    }

    pub fn reset(&mut self) {
        self.drag.press(None);
        self.paint();
    }

    pub fn drag(&mut self, event: Obj) -> bool {
        if event.is_null() {
            return false;
        }
        let x = lv::touch_point(event).map(|point| point[0]);
        let mut confirmed = false;
        match unsafe { lvx_event_get_code(event) } {
            EVENT_PRESSED => self.drag.press(x),
            EVENT_PRESSING => self.drag.move_to(x, self.layout.travel),
            EVENT_RELEASED => confirmed = self.drag.release(x, self.layout.travel),
            EVENT_PRESS_LOST => self.drag.cancel(),
            _ => return false,
        }
        self.paint();
        confirmed
    }

    fn paint(&self) {
        let distance = self.drag.distance;
        unsafe {
            lvx_object_align(
                self.thumb,
                1,
                self.layout.thumb.x + distance,
                self.layout.thumb.y,
            )
        };
        let opacity = 255 * (self.layout.travel - distance) as u32 / self.layout.travel as u32;
        lv::style(self.caption, lv::OPA, opacity);
    }
}

pub unsafe fn caption_label(parent: Obj, area: Rect, text: &[u8], icon: &'static [u8]) -> Obj {
    unsafe {
        let caption = cards::container(parent, area.x, area.y, area.w, area.h);
        if !caption.is_null() {
            let label = lv::label(caption, text, 0, 3, 55, 32, 0x6C6C6C, lv::Font::Medium24);
            lv::style(label, 94, 3);
            lv::image_child(caption, icon, 64, 0);
        }
        caption
    }
}

pub unsafe fn caption_image(parent: Obj, area: Rect, art: &'static [u8]) -> Obj {
    unsafe { lv::image(parent, art, area.x, area.y) }
}
