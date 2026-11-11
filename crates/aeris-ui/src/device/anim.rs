//! 按压反馈与松手回弹。

use canopus_target_private::*;

use super::lv::{self, Obj};

const SCALE_100: i32 = 256;
const SCALE_PRESSED: i32 = 246;
const FRAME_MS: u32 = 16;
const PRESS_MS: u32 = 130;
const RELEASE_MS: u32 = 240;
const FULL: i32 = 1000;
const EVENT_PRESSED: u32 = 1;
const EVENT_PRESS_LOST: u32 = 3;
const EVENT_RELEASED: u32 = 8;
const TRANSFORM_SCALE_X: u32 = 108;
const TRANSFORM_SCALE_Y: u32 = 109;
const TRANSFORM_PIVOT_X: u32 = 111;
const TRANSFORM_PIVOT_Y: u32 = 112;
const IMAGE_OPA: u32 = 68;
const CARD_IDLE_OPA: i32 = 170;

#[derive(Copy, Clone)]
pub enum Feedback {
    Scale,
    Card,
    Capsule,
}

#[derive(Copy, Clone)]
struct Entry {
    object: Obj,
    feedback: Feedback,
    from: i32,
    to: i32,
    started_at: u32,
    moving: bool,
}

const EMPTY: Entry = Entry {
    object: core::ptr::null_mut(),
    feedback: Feedback::Scale,
    from: 0,
    to: 0,
    started_at: 0,
    moving: false,
};

static mut ENTRIES: [Entry; 8] = [EMPTY; 8];
static mut TIMER: Obj = core::ptr::null_mut();

pub fn now_ms() -> u32 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let context = super::context();
        unsafe { ((*(*context).time).monotonic_ms)(context.cast()) }
    }
    #[cfg(target_arch = "wasm32")]
    unsafe { gallery_now_ms() }
}

pub unsafe fn attach(object: Obj, pivot_x: i32, pivot_y: i32, feedback: Feedback) {
    if object.is_null() {
        return;
    }
    let press_callback = match feedback {
        Feedback::Scale => press_scale,
        Feedback::Card => press_card,
        Feedback::Capsule => press_capsule,
    };
    unsafe {
        lvx_object_add_flag(object, lv::CLICKABLE);
        lv::style(object, TRANSFORM_PIVOT_X, pivot_x as u32);
        lv::style(object, TRANSFORM_PIVOT_Y, pivot_y as u32);
        render(object, feedback, 0);
        lvx_event_add(object, press_callback, EVENT_PRESSED, object);
        lvx_event_add(object, release, EVENT_RELEASED, object);
        lvx_event_add(object, release, EVENT_PRESS_LOST, object);
    }
}

extern "C" fn press_scale(event: Obj) {
    unsafe { retarget(event, Some(Feedback::Scale)) };
}

extern "C" fn press_card(event: Obj) {
    unsafe { retarget(event, Some(Feedback::Card)) };
}

extern "C" fn press_capsule(event: Obj) {
    unsafe { retarget(event, Some(Feedback::Capsule)) };
}

extern "C" fn release(event: Obj) {
    unsafe { retarget(event, None) };
}

unsafe fn retarget(event: Obj, press: Option<Feedback>) {
    if event.is_null() {
        return;
    }
    let object = unsafe { lvx_event_get_user_data(event) } as Obj;
    if object.is_null() {
        return;
    }
    let entries = unsafe { &mut *core::ptr::addr_of_mut!(ENTRIES) };
    let index = match entries.iter().position(|entry| entry.object == object) {
        Some(index) => index,
        None => {
            let Some(feedback) = press else { return };
            let Some(index) = entries.iter().position(|entry| entry.object.is_null()) else {
                return;
            };
            entries[index] = Entry {
                object,
                feedback,
                ..EMPTY
            };
            index
        }
    };
    let entry = &mut entries[index];
    let to = if press.is_some() { FULL } else { 0 };
    if entry.to == to {
        return;
    }

    let now = now_ms();
    let from = sample(entry, now).0;
    entry.from = from;
    entry.to = to;
    entry.started_at = now;
    entry.moving = true;
    render(entry.object, entry.feedback, from);

    unsafe {
        if TIMER.is_null() {
            TIMER = lvx_timer_create(tick, FRAME_MS, core::ptr::null_mut());
        }
        if TIMER.is_null() {
            finish(entry);
        }
    }
}

fn sample(entry: &Entry, now: u32) -> (i32, bool) {
    let duration = if entry.to == FULL {
        PRESS_MS
    } else {
        RELEASE_MS
    };
    let elapsed = now.wrapping_sub(entry.started_at);
    if !entry.moving || elapsed >= duration {
        return (entry.to, true);
    }
    let progress = elapsed as i32 * FULL / duration as i32;
    let remaining = FULL - progress;
    let eased = if entry.to == FULL {
        FULL - remaining * remaining / FULL
    } else {
        // 松手略微过冲，亮度仍限制在正常范围内。
        let remaining = i64::from(remaining);
        FULL + (remaining * remaining * (1701 - 2701 * remaining / 1000) / 1_000_000) as i32
    };
    (entry.from + (entry.to - entry.from) * eased / FULL, false)
}

fn render(object: Obj, feedback: Feedback, amount: i32) {
    let scale = (SCALE_100 * FULL - (SCALE_100 - SCALE_PRESSED) * amount + FULL / 2) / FULL;
    let amount = amount.clamp(0, FULL);
    lv::style(object, TRANSFORM_SCALE_X, scale as u32);
    lv::style(object, TRANSFORM_SCALE_Y, scale as u32);
    match feedback {
        Feedback::Scale => {}
        Feedback::Card => lv::style(
            object,
            IMAGE_OPA,
            (CARD_IDLE_OPA + (255 - CARD_IDLE_OPA) * amount / FULL) as u32,
        ),
        Feedback::Capsule => lv::style(object, lv::OPA, (255 - 35 * amount / FULL) as u32),
    }
}

fn finish(entry: &mut Entry) {
    render(entry.object, entry.feedback, entry.to);
    entry.moving = false;
    if entry.to == 0 {
        *entry = EMPTY;
    }
}

extern "C" fn tick(_timer: Obj) {
    let now = now_ms();
    let entries = unsafe { &mut *core::ptr::addr_of_mut!(ENTRIES) };
    let mut moving = false;
    for entry in entries.iter_mut().filter(|entry| entry.moving) {
        let (amount, done) = sample(entry, now);
        if done {
            finish(entry);
        } else {
            render(entry.object, entry.feedback, amount);
            moving = true;
        }
    }
    if !moving {
        unsafe { cancel_timer() };
    }
}

unsafe fn cancel_timer() {
    unsafe {
        if !TIMER.is_null() {
            lvx_timer_delete(TIMER);
            TIMER = core::ptr::null_mut();
        }
    }
}

pub unsafe fn settle() {
    unsafe {
        for entry in &*core::ptr::addr_of!(ENTRIES) {
            if !entry.object.is_null() {
                render(entry.object, entry.feedback, 0);
            }
        }
        stop();
    }
}

pub unsafe fn stop() {
    unsafe {
        cancel_timer();
        ENTRIES = [EMPTY; 8];
    }
}
