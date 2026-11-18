//! 展厅固件页面注册与生命周期。

use core::{
    ffi::c_void,
    ptr::null_mut,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

use aeris_ui::device::{anim, assets, dialog, lv::Obj};
use canopus_abi::{AppEntryV1, ContextV1};
use canopus_runtime::launcher;
use canopus_target_private::{activity_finish, activity_navigate, firmware_page_descriptor};

use crate::{app_assets, confirmation_demo, gallery, inputs, APP_NAME};

const NAMES: [&[u8]; 2] = [b"aeris_gallery\0", b"aeris_gallery_inputs\0"];
const HOME: usize = 0;
const INPUTS: usize = 1;
static APP_ID: AtomicU32 = AtomicU32::new(0);
static READY: AtomicBool = AtomicBool::new(false);
static mut PAGES: [firmware_page_descriptor; NAMES.len()] = unsafe { core::mem::zeroed() };
static mut LIVE: [*mut firmware_page_descriptor; NAMES.len()] = [null_mut(); NAMES.len()];

pub fn app_id() -> u32 {
    APP_ID.load(Ordering::Acquire)
}

pub unsafe fn install_stage(ctx: *const ContextV1, stage: u32) -> Result<(), i32> {
    match stage {
        1 if app_id() == 0 => {
            let pages = core::ptr::addr_of_mut!(PAGES).cast::<firmware_page_descriptor>();
            let entries: [Obj; NAMES.len()] = core::array::from_fn(|index| unsafe {
                let page = pages.add(index);
                (*page).page_name = NAMES[index].as_ptr() as Obj;
                (*page).page_id = index as u16;
                (*page).on_signal = signal as *const () as Obj;
                (*page).on_create = create as *const () as Obj;
                (*page).on_resume = resume as *const () as Obj;
                (*page).on_pause = pause as *const () as Obj;
                (*page).on_ui_destroy = destroy as *const () as Obj;
                page.cast()
            });
            let icon = if app_assets::ensure_icon() {
                app_assets::APPICON.as_ptr().cast()
            } else {
                core::ptr::null()
            };
            let entry = AppEntryV1 {
                struct_size: core::mem::size_of::<AppEntryV1>() as u32,
                label: APP_NAME.as_ptr(),
                icon,
                pages: entries.as_ptr(),
                page_count: entries.len() as u32,
            };
            let id = unsafe { launcher::register_app(ctx.cast(), &entry) }?;
            APP_ID.store(u32::from(id), Ordering::Release);
            Ok(())
        }
        1 => Ok(()),
        2 if app_id() != 0 => {
            READY.store(true, Ordering::Release);
            Ok(())
        }
        2 => Err(-1),
        _ => Err(-22),
    }
}

pub extern "C" fn open_inputs(_event: Obj) {
    if READY.load(Ordering::Acquire) {
        unsafe { activity_navigate((app_id() << 16) | INPUTS as u32, 0, 0, 0) };
    }
}

pub extern "C" fn close_gallery(_event: Obj) {
    close(HOME);
}

pub extern "C" fn close_inputs(_event: Obj) {
    close(INPUTS);
}

fn close(index: usize) {
    let page = unsafe { (*core::ptr::addr_of!(LIVE))[index] };
    if !page.is_null() {
        unsafe { activity_finish(page) };
    }
}

fn id(page: *mut firmware_page_descriptor) -> usize {
    if page.is_null() {
        usize::MAX
    } else {
        usize::from(unsafe { (*page).page_id })
    }
}

extern "C" fn signal(
    _page: *mut firmware_page_descriptor,
    _event: u32,
    _payload: *mut c_void,
) -> i32 {
    0
}

extern "C" fn create(page: *mut firmware_page_descriptor, root: Obj, _data: Obj) -> i32 {
    let index = id(page);
    if root.is_null() || index >= NAMES.len() || !assets::ensure() {
        return -1;
    }
    assets::ensure_fonts();
    let result = unsafe {
        match index {
            HOME if assets::ensure_files(confirmation_demo::ASSETS) => gallery::create(root),
            INPUTS => inputs::create(root, close_inputs),
            _ => -1,
        }
    };
    if result == 0 {
        unsafe { (*core::ptr::addr_of_mut!(LIVE))[index] = page };
    }
    result
}

extern "C" fn resume(page: *mut firmware_page_descriptor) -> i32 {
    let index = id(page);
    if index >= NAMES.len() {
        return -1;
    }
    unsafe { (*core::ptr::addr_of_mut!(LIVE))[index] = page };
    0
}

extern "C" fn pause(page: *mut firmware_page_descriptor) -> i32 {
    unsafe { anim::settle() };
    match id(page) {
        HOME => {
            dialog::hide_confirm();
            gallery::pause();
        }
        INPUTS => inputs::pause(),
        _ => {}
    }
    0
}

extern "C" fn destroy(page: *mut firmware_page_descriptor) -> i32 {
    let index = id(page);
    if index >= NAMES.len() {
        return -1;
    }
    unsafe {
        anim::stop();
        (*core::ptr::addr_of_mut!(LIVE))[index] = null_mut();
    }
    match index {
        HOME => {
            dialog::stop();
            gallery::destroy();
        }
        INPUTS => inputs::destroy(),
        _ => {}
    }
    0
}
