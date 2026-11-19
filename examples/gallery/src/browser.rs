use std::{cell::Cell, ptr::null_mut};

use aeris_ui::device::{anim, assets, dialog, lv::Obj, page};
use canopus_target_private::{
    lvx_object_delete, lvx_set_hidden, lvx_timer_create, lvx_timer_delete,
};

use crate::{confirmation_demo, gallery, inputs};

#[derive(Clone, Copy)]
struct Pages {
    host: Obj,
    gallery: Obj,
    inputs: Obj,
}

thread_local! {
    static PAGES: Cell<Pages> = const { Cell::new(Pages {
        host: null_mut(), gallery: null_mut(), inputs: null_mut(),
    }) };
    static NAVIGATING: Cell<bool> = const { Cell::new(false) };
}

pub fn start(host: Obj) -> bool {
    if !assets::ensure_files(confirmation_demo::ASSETS) {
        return false;
    }
    unsafe {
        let container = page::screen_in(host, aeris_ui::device::display_profile().viewport);
        if container.is_null() {
            return false;
        }
        if gallery::create(container) != 0 {
            lvx_object_delete(container);
            return false;
        }
        PAGES.set(Pages {
            host,
            gallery: container,
            inputs: null_mut(),
        });
    }
    true
}

pub extern "C" fn open_inputs(_: Obj) {
    queue_navigation(show_inputs);
}

pub extern "C" fn close_inputs(_: Obj) {
    queue_navigation(show_gallery);
}

pub extern "C" fn close_gallery(event: Obj) {
    dialog::dismiss(event);
}

#[unsafe(no_mangle)]
pub extern "C" fn aeris_devbox_back() {
    close_inputs(null_mut());
}

fn queue_navigation(callback: extern "C" fn(Obj)) {
    if NAVIGATING.replace(true) {
        return;
    }
    if unsafe { lvx_timer_create(callback, 1, null_mut()) }.is_null() {
        NAVIGATING.set(false);
    }
}

extern "C" fn show_inputs(timer: Obj) {
    unsafe { lvx_timer_delete(timer) };
    NAVIGATING.set(false);
    let mut pages = PAGES.get();
    if !pages.inputs.is_null() {
        return;
    }
    unsafe {
        let container = page::screen_in(pages.host, aeris_ui::device::display_profile().viewport);
        if container.is_null() {
            return;
        }
        if inputs::create(container, close_inputs) != 0 {
            lvx_object_delete(container);
            return;
        }
        anim::settle();
        dialog::hide_confirm();
        gallery::pause();
        lvx_set_hidden(pages.gallery, 1);
        pages.inputs = container;
        PAGES.set(pages);
    }
}

extern "C" fn show_gallery(timer: Obj) {
    unsafe { lvx_timer_delete(timer) };
    NAVIGATING.set(false);
    let mut pages = PAGES.get();
    if pages.inputs.is_null() {
        close_gallery(null_mut());
        return;
    }
    unsafe {
        anim::settle();
        inputs::pause();
        inputs::destroy();
        lvx_object_delete(pages.inputs);
        lvx_set_hidden(pages.gallery, 0);
    }
    pages.inputs = null_mut();
    PAGES.set(pages);
}
