//! 数字滚轮与确认滑块展项。

use aeris_ui::{
    device::{
        assets,
        confirm_slider::{self, Slider},
        header::{HeaderBar, HeaderOptions, HeaderStyle},
        lv::{self, Font, Obj},
        numeric_wheel::{Column, NumericWheel, NumericWheelStyle},
        page as ui,
    },
    geometry::Rect,
    header::HEADER_HEIGHT,
};
use canopus_target_private::*;

struct Inputs {
    wheel: NumericWheel<2>,
    slider: Slider,
    feedback: Obj,
}

static mut INPUTS: Option<Inputs> = None;

pub unsafe fn create(parent: Obj, on_back: LvxEventCallback) -> i32 {
    let viewport = aeris_ui::device::display_profile().viewport;
    let safe = viewport.content();
    unsafe {
        let root = ui::screen_in(parent, viewport);
        if root.is_null() {
            return -1;
        }
        let content = ui::scroll_in(root, Rect::new(safe.x, safe.y + HEADER_HEIGHT, safe.w, safe.h - HEADER_HEIGHT));
        if content.is_null() {
            lvx_object_delete(root);
            return -1;
        }
        let columns = [
            Column {
                min: 1,
                max: 12,
                initial: 3,
                label: b"Month\0",
                badge_width: 84,
            },
            Column {
                min: 1,
                max: 31,
                initial: 15,
                label: b"Day\0",
                badge_width: 60,
            },
        ];
        let Some(wheel) = NumericWheel::mount_with_styles(
            content,
            Rect::new(0, 2, safe.w, 200),
            columns,
            [NumericWheelStyle::WithUnit; 2],
            0,
            wheel_event,
        ) else {
            lvx_object_delete(root);
            return -1;
        };
        let feedback = lv::label(
            content,
            b"Slide to confirm\0",
            4,
            214,
            safe.w - 8,
            32,
            0xFFFFFF,
            Font::Medium24,
        );
        let Some(slider) = Slider::create(
            content,
            Rect::new(0, (safe.h - HEADER_HEIGHT - 92).max(252), safe.w, 88),
            assets::STOP_FORE,
            |parent, area| {
                confirm_slider::caption_label(parent, area, b"Slide\0", assets::STOP_CAPTION_ICON)
            },
            slider_event,
        ) else {
            lvx_object_delete(root);
            return -1;
        };
        if HeaderBar::new_in(root, safe, "Input Controls", None, HeaderOptions {
            style: HeaderStyle::Right, on_left: Some(on_back), ..Default::default()
        }).is_none() {
            lvx_object_delete(root);
            return -1;
        }
        INPUTS = Some(Inputs {
            wheel,
            slider,
            feedback,
        });
    }
    0
}

extern "C" fn wheel_event(event: Obj) {
    if let Some(input) = unsafe { (&*core::ptr::addr_of!(INPUTS)).as_ref() } {
        input.wheel.handle_event(event);
    }
}

extern "C" fn slider_event(event: Obj) {
    let Some(input) = (unsafe { (&mut *core::ptr::addr_of_mut!(INPUTS)).as_mut() }) else {
        return;
    };
    let pressed = unsafe { lvx_event_get_code(event) } == 1;
    let confirmed = input.slider.drag(event);
    if input.feedback.is_null() {
        return;
    }
    if confirmed {
        let [month, day] = input.wheel.values();
        let mut label = *b"Confirmed: 00 / 00\0";
        for (value, index) in [(month, 11), (day, 16)] {
            label[index] += value / 10;
            label[index + 1] += value % 10;
        }
        unsafe { lvx_label_set_text(input.feedback, label.as_ptr()) };
    } else if pressed {
        unsafe { lvx_label_set_text(input.feedback, b"Slide to confirm\0".as_ptr()) };
    }
}

pub fn pause() {
    if let Some(input) = unsafe { (&mut *core::ptr::addr_of_mut!(INPUTS)).as_mut() } {
        input.slider.reset();
    }
}

pub fn destroy() {
    unsafe { INPUTS = None };
}
