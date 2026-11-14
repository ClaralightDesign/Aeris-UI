//! 固件组件所用的 LVGL 调用在浏览器中的绑定。

use core::ffi::c_void;

pub type Obj = *mut c_void;
pub type LvxEventCallback = extern "C" fn(Obj);

pub type LvxTimerCallback = extern "C" fn(Obj);
pub const EVENT_CLICKED: u32 = 7;
pub const LV_LABEL_LONG_SCROLL_CIRCULAR: u32 = 3;

pub const LV_STYLE_CLIP_CORNER: u32 = 45;
pub const LV_STYLE_BORDER_WIDTH: u32 = 48;
pub const LV_STYLE_BORDER_OPA: u32 = 50;

unsafe extern "C" {
    pub fn gallery_panic(file: *const u8, length: usize, line: u32);
    pub fn gallery_screen() -> Obj;
    pub fn gallery_capture_backdrop(root: Obj) -> *const c_void;
    pub fn gallery_set_crop_height(height: i32);
    pub fn gallery_sync_refresh_to_frame();
    pub(crate) fn devbox_scene_id(value: *mut u8, capacity: usize) -> usize;
    pub fn gallery_now_ms() -> u32;
    pub fn gallery_prefers_reduced_motion() -> bool;
    pub fn gallery_apply_font(object: Obj, family: u32, px: i32) -> i32;
    pub fn gallery_asset(name: *const u8, name_len: usize, bytes: *mut u8, len: usize) -> bool;
    pub fn gallery_font(font: u32, px: i32, bytes: *const u8);
    pub fn gallery_time(text: *mut u8);
    pub(crate) fn malloc(size: usize) -> *mut c_void;
    pub(crate) fn free(data: *mut c_void);

    pub fn lvx_object_create(parent: Obj) -> Obj;
    pub fn lvx_label_create(parent: Obj) -> Obj;
    pub fn lvx_label_set_text_align_center(object: Obj);
    pub fn lvx_label_set_long_mode(object: Obj, mode: u32);
    pub fn lvx_object_set_local_style_u32(object: Obj, property: u32, value: u32, selector: u32);
    pub fn lvx_object_set_scroll_snap_x(object: Obj, snap: u32);
    pub fn lvx_object_scroll_x(object: Obj) -> i32;
    pub fn lvx_object_scroll_to_y(object: Obj, y: i32, animated: u32);
    pub fn lvx_object_scroll_to_x(object: Obj, x: i32, animated: u32);
    pub fn lvx_object_move_to_index(object: Obj, index: i32);
    pub fn lvx_timer_create(callback: LvxTimerCallback, ms: u32, user: Obj) -> Obj;
    pub fn lvx_timer_delete(timer: Obj);
    pub fn devbox_timer_user_data(timer: Obj) -> Obj;
    pub fn devbox_timer_pause(timer: Obj);
    pub fn devbox_timer_resume(timer: Obj);
    pub fn devbox_timer_reset(timer: Obj);
    pub fn devbox_storage_read(key: *const u8, buffer: *mut u8, capacity: usize) -> i32;
    pub fn devbox_storage_write(key: *const u8, buffer: *const u8, length: usize) -> i32;
    pub fn devbox_storage_remove(key: *const u8) -> i32;
    fn gallery_coords(object: Obj, points: *mut i32);
    pub fn lvx_object_align(object: Obj, align: u32, x: i32, y: i32);
    pub fn lvx_object_set_size(object: Obj, w: i32, h: i32);
    pub fn lvx_object_add_flag(object: Obj, flags: u32);
    pub fn lvx_object_clear_flag(object: Obj, flags: u32);
    pub fn lvx_object_delete(object: Obj);
    pub fn lvx_set_hidden(object: Obj, hidden: u32);
    pub fn lvx_image_create(parent: Obj) -> Obj;
    pub fn lvx_image_set_src(object: Obj, src: *const c_void);
    pub fn lvx_image_set_scale(object: Obj, sx: i32, sy: i32);
    pub fn lvx_label_set_text(object: Obj, text: *const u8);
    pub fn lvx_object_set_scroll_dir(object: Obj, direction: u32);
    pub fn lvx_object_set_scrollbar_mode(object: Obj, mode: u32);
    pub fn lvx_object_scroll_y(object: Obj) -> i32;
    pub fn lvx_object_scroll_by(object: Obj, x: i32, y: i32, animated: u32);
    pub fn lvx_event_add(object: Obj, callback: LvxEventCallback, code: u32, data: Obj);
    pub fn lvx_event_get_user_data(event: Obj) -> usize;
    pub fn lvx_event_get_code(event: Obj) -> u32;
}

pub unsafe fn lvx_object_coords(object: Obj) -> [i32; 4] {
    let mut points = [0; 4];
    unsafe { gallery_coords(object, points.as_mut_ptr()) };
    points
}

pub mod generated_symbols {
    use super::Obj;
    unsafe extern "C" {
        pub fn raw_lv_event_get_indev(event: Obj) -> Obj;
        pub fn raw_lv_indev_get_point(device: Obj, point: *mut i32);
        pub fn raw_lv_obj_scroll_to(object: Obj, x: i32, y: i32, animated: u32);
    }
}
