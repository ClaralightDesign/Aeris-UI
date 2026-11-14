//! 浏览器预览的 LVGL 背景快照。

use crate::{gallery_capture_backdrop, Obj};

pub const IMAGE_SCALE: i32 = 4 * 256;

pub unsafe fn capture(root: Obj) -> *const core::ffi::c_void {
    unsafe { gallery_capture_backdrop(root) }
}

pub fn retire() {}
