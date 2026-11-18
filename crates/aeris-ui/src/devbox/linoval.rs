//! 将预览字形交给 LVGL 标签。

use super::lv::{Font, Obj};

pub(super) fn apply(label: Obj, font: Font, px: i32) -> i32 {
    unsafe { crate::gallery_apply_font(label, font as u32, px) }
}

pub(super) fn apply_large_title(label: Obj) {
    unsafe { crate::gallery_apply_font(label, 5, 44) };
}

pub(super) fn apply_clock(label: Obj) {
    unsafe { crate::gallery_apply_font(label, 6, 22) };
}
