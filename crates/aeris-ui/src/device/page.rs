//! 屏幕、滚动容器、返回标题栏及按钮模板。

use super::{
    header,
    lv::{self, Font, Obj},
};
use crate::{
    capsule::CapsuleKind,
    geometry::{Rect, Viewport},
    theme::{self, Color},
};
use canopus_target_private::*;

pub const WIDTH: i32 = Viewport::BAND_10_PRO.bounds().w;
pub const HEIGHT: i32 = Viewport::BAND_10_PRO.bounds().h;
pub const PAGE_TOP: i32 = 82;
pub const PAGE_BACKGROUND: u32 = Color::Background.default_rgb();

#[cfg(not(target_arch = "wasm32"))]
pub fn clock_bytes() -> [u8; 6] {
    let mut time = stock_timespec_t {
        tv_sec: -1,
        tv_nsec: 0,
    };
    if unsafe { canopus_fw_clock_gettime(0, &mut time) } != 0 || time.tv_sec < 0 {
        return *b"--:--\0";
    }
    // 使用固件日历的时区转换和原生时间结构。
    type LocalTime = unsafe extern "C" fn(i64, *mut [i32; 11]) -> *mut [i32; 11];
    let local_time: LocalTime =
        unsafe { core::mem::transmute(generated_symbols::CALENDAR_LOCALTIME_CALLABLE) };
    let mut local = [0; 11];
    if unsafe { local_time(time.tv_sec, &mut local) }.is_null() {
        return *b"--:--\0";
    }
    let [_, minute, hour, ..] = local;
    if !(0..24).contains(&hour) || !(0..60).contains(&minute) {
        return *b"--:--\0";
    }
    [
        b'0' + (hour / 10) as u8,
        b'0' + (hour % 10) as u8,
        b':',
        b'0' + (minute / 10) as u8,
        b'0' + (minute % 10) as u8,
        0,
    ]
}

#[cfg(target_arch = "wasm32")]
pub fn clock_bytes() -> [u8; 6] {
    let mut clock = *b"--:--\0";
    unsafe { gallery_time(clock.as_mut_ptr()) };
    clock
}

pub unsafe fn right_header(root: Obj, title: &str, on_back: LvxEventCallback) {
    unsafe { header(root, title, on_back, false) };
}

pub unsafe fn header(root: Obj, title: &str, on_back: LvxEventCallback, about: bool) {
    unsafe { header::back(root, title.as_bytes(), on_back, about) };
}

pub unsafe fn screen(parent: Obj) -> Obj {
    unsafe { screen_in(parent, Viewport::BAND_10_PRO) }
}

pub unsafe fn screen_in(parent: Obj, viewport: Viewport) -> Obj {
    let Rect { x, y, w, h } = viewport.bounds();
    let root = unsafe { lv::background(parent, w, h, Color::Background.resolve(), 0) };
    if !root.is_null() {
        unsafe {
            lv::inspect_name(root, c"AerisUI.Screen");
            lvx_object_align(root, 1, x, y);
            lvx_object_clear_flag(root, lv::SCROLLABLE | lv::CLICKABLE);
        };
    }
    root
}

pub unsafe fn scroll(parent: Obj) -> Obj {
    unsafe { scroll_in(parent, Viewport::BAND_10_PRO.content()) }
}

pub unsafe fn scroll_in(parent: Obj, bounds: Rect) -> Obj {
    let area = unsafe { lv::page(parent, bounds.x, bounds.w, bounds.h) };
    if !area.is_null() {
        unsafe {
            lv::inspect_name(area, c"AerisUI.ScrollArea");
            lvx_object_align(area, 1, bounds.x, bounds.y);
            lvx_object_clear_flag(area, lv::SCROLL_CHAIN);
            lvx_object_add_flag(area, lv::CLICKABLE);
        }
    }
    area
}

pub unsafe fn button(
    parent: Obj,
    y: i32,
    title: &str,
    kind: CapsuleKind,
    action: LvxEventCallback,
) -> Obj {
    unsafe { button_with_data(parent, y, title, kind, action, 0) }
}

pub unsafe fn button_with_data(
    parent: Obj,
    y: i32,
    title: &str,
    kind: CapsuleKind,
    action: LvxEventCallback,
    data: usize,
) -> Obj {
    unsafe {
        button_in(
            parent,
            Rect::new(0, y, WIDTH, kind.height()),
            title,
            kind,
            action,
            data,
        )
    }
}

pub unsafe fn button_in(
    parent: Obj,
    bounds: Rect,
    title: &str,
    kind: CapsuleKind,
    action: LvxEventCallback,
    data: usize,
) -> Obj {
    let Some(area) = bounds
        .horizontal_inset(3)
        .filter(|area| area.h >= kind.height())
    else {
        return core::ptr::null_mut();
    };
    unsafe {
        let button = lv::capsule(parent, area.x, area.y, area.w, kind);
        if !button.is_null() {
            lv::inspect_name(button, c"AerisUI.Button");
            let color = if kind == CapsuleKind::LargePrimary {
                theme::primary_button_foreground()
            } else {
                Color::PrimaryText.resolve()
            };
            let label = lv::label(
                button,
                title.as_bytes(),
                0,
                (kind.height() - 32) / 2,
                area.w,
                32,
                color,
                Font::Medium24,
            );
            lv::text_px(label, area.w, 32, 2, Font::Medium24, 26);
            lv::bind(button, action, data);
        }
        button
    }
}
