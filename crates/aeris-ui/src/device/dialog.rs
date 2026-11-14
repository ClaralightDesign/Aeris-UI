//! 对话框的模糊背景、过渡和计时器上限提示。

use crate::{
    capsule::CapsuleKind,
    theme::{self, Color},
};
use canopus_target_private::*;

use super::anim::now_ms;
use super::lv::{self, Font, Obj};
use super::{
    backdrop,
    confirmation_overlay::{FirstStageTap, Motion, DETAIL_DURATION_MS},
};

const BUTTON_WIDTH: i32 = 330;
const BUTTON_GAP: i32 = 6;
const BUTTON_BOTTOM: i32 = 3;
const ALIGN_CENTER: u32 = 9;
const LINE_HEIGHT: i32 = 34;
/// 对话框字号。
const DIALOG_TEXT_PX: i32 = 26;
const TEXT_INSET: i32 = 26;
const FRAME_MS: u32 = 16;
const BACKDROP_MS: u32 = 220;
const CONTENT_DELAY_MS: u32 = 24;
const CONTENT_MS: u32 = 240;
const CONTENT_OFFSET: u32 = 18;
const CLOSE_CONTENT_MS: u32 = 180;
const CLOSE_BACKDROP_DELAY_MS: u32 = 24;
const CLOSE_BACKDROP_MS: u32 = 196;
#[derive(Copy, Clone, PartialEq, Eq)]
enum Phase {
    Hidden,
    Capturing,
    Opening,
    Open,
    Expanding,
    Detail,
    Closing,
}

#[derive(Copy, Clone)]
struct Frame {
    backdrop: u32,
    content: u32,
    icon: u32,
    detail: u32,
}

impl Frame {
    const HIDDEN: Self = Self {
        backdrop: 0,
        content: 0,
        icon: 0,
        detail: 0,
    };
    const OPEN: Self = Self {
        backdrop: 255,
        content: 255,
        icon: 255,
        detail: 0,
    };

    fn content_offset(self, fade_only: bool) -> u32 {
        if fade_only {
            0
        } else {
            CONTENT_OFFSET * (255 - self.content) / 255
        }
    }
}

struct Dialog {
    surface: Obj,
    backdrop: Obj,
    has_blur: bool,
    fade_only: bool,
    confirmation: Option<Motion>,
    content: Obj,
    timer: Obj,
    started_at: u32,
    phase: Phase,
    frame: Option<Frame>,
    closing_from: Frame,
}

const EMPTY: Dialog = Dialog {
    surface: core::ptr::null_mut(),
    backdrop: core::ptr::null_mut(),
    has_blur: false,
    fade_only: false,
    confirmation: None,
    content: core::ptr::null_mut(),
    timer: core::ptr::null_mut(),
    started_at: 0,
    phase: Phase::Hidden,
    frame: None,
    closing_from: Frame::HIDDEN,
};

static mut DIALOG: Dialog = EMPTY;
static mut LIMIT: Option<Layers> = None;

#[derive(Clone, Copy)]
pub struct Layers {
    pub surface: Obj,
    backdrop: Obj,
    pub content: Obj,
}

pub unsafe fn layers(parent: Obj, width: i32, height: i32) -> Option<Layers> {
    let dialog = unsafe { lv::background(parent, width, height, 0x000000, 40) };
    if dialog.is_null() {
        return None;
    }
    unsafe {
        lvx_set_hidden(dialog, 1);
        lvx_object_add_flag(dialog, lv::CLICKABLE);
        lvx_object_clear_flag(
            dialog,
            lv::SCROLLABLE | lv::SCROLL_CHAIN | lv::GESTURE_BUBBLE,
        );

        let backdrop = lvx_image_create(dialog);
        if !backdrop.is_null() {
            lvx_object_clear_flag(backdrop, lv::CLICKABLE | lv::SCROLLABLE);
            lvx_set_hidden(backdrop, 1);
        }

        let content = lv::background(dialog, width, height, 0x000000, 0);
        if content.is_null() {
            lvx_object_delete(dialog);
            return None;
        }
        lvx_object_clear_flag(content, lv::CLICKABLE | lv::SCROLLABLE);
        lv::style(content, lv::BG_OPA, 0);
        Some(Layers {
            surface: dialog,
            backdrop,
            content,
        })
    }
}

pub unsafe fn confirm(
    parent: Obj,
    title: &str,
    message: &[&str],
    action: &str,
    cancel_text: &str,
    on_confirm: LvxEventCallback,
) -> Obj {
    let (width, height) = (super::page::WIDTH, super::page::HEIGHT);
    let Some(layers) = (unsafe { layers(parent, width, height) }) else {
        return core::ptr::null_mut();
    };
    let Layers {
        surface: dialog,
        content,
        ..
    } = layers;
    unsafe {
        let clock = lv::label_right(
            content,
            &super::page::clock_bytes(),
            132,
            6,
            width - 148,
            LINE_HEIGHT,
            Color::PrimaryText.resolve(),
        );
        lv::text_px(
            clock,
            width - 148,
            LINE_HEIGHT,
            3,
            Font::Small24,
            DIALOG_TEXT_PX,
        );

        let text_width = width - 2 * TEXT_INSET;
        let title = lv::label(
            content,
            title.as_bytes(),
            TEXT_INSET,
            88,
            text_width,
            LINE_HEIGHT,
            Color::Accent.resolve(),
            Font::Medium24,
        );
        lv::text_px(
            title,
            text_width,
            LINE_HEIGHT,
            2,
            Font::Medium24,
            DIALOG_TEXT_PX,
        );
        for (index, line) in message.iter().enumerate() {
            let label = lv::label(
                content,
                line.as_bytes(),
                TEXT_INSET,
                126 + index as i32 * LINE_HEIGHT,
                text_width,
                LINE_HEIGHT,
                Color::PrimaryText.resolve(),
                Font::Medium24,
            );
            lv::text_px(
                label,
                text_width,
                LINE_HEIGHT,
                2,
                Font::Medium24,
                DIALOG_TEXT_PX,
            );
        }

        let x = (width - BUTTON_WIDTH) / 2;
        let button_height = CapsuleKind::Large.height();
        let primary_y = height - BUTTON_BOTTOM - button_height;
        let secondary_y = primary_y - BUTTON_GAP - button_height;
        let secondary = lv::capsule(content, x, secondary_y, BUTTON_WIDTH, CapsuleKind::Large);
        let primary = lv::capsule(
            content,
            x,
            primary_y,
            BUTTON_WIDTH,
            CapsuleKind::LargePrimary,
        );
        if secondary.is_null() || primary.is_null() {
            lvx_object_delete(dialog);
            return core::ptr::null_mut();
        }

        let label_y = (button_height - LINE_HEIGHT) / 2;
        let action_color = theme::primary_button_foreground();
        for (button, label, color) in [
            (
                secondary,
                cancel_text,
                Color::SecondaryText.resolve_or(0xDDDDDD),
            ),
            (primary, action, action_color),
        ] {
            let text = lv::label(
                button,
                label.as_bytes(),
                0,
                label_y,
                BUTTON_WIDTH,
                LINE_HEIGHT,
                color,
                Font::Medium24,
            );
            lv::text_px(
                text,
                BUTTON_WIDTH,
                LINE_HEIGHT,
                2,
                Font::Medium24,
                DIALOG_TEXT_PX,
            );
        }
        lvx_event_add(secondary, dismiss, EVENT_CLICKED, dialog);
        lvx_event_add(primary, on_confirm, EVENT_CLICKED, dialog);
        LIMIT = Some(layers);
    }
    dialog
}

pub extern "C" fn show(event: Obj) {
    let root = unsafe { lvx_event_get_user_data(event) } as Obj;
    if let Some(layers) = unsafe { LIMIT } {
        open(root, layers);
    }
}

pub fn open(root: Obj, layers: Layers) {
    let _ = open_with_transition(root, layers, false, None);
}

pub fn open_fade(root: Obj, layers: Layers) {
    let _ = open_with_transition(root, layers, true, None);
}

pub(super) fn busy() -> bool {
    unsafe { DIALOG.phase != Phase::Hidden }
}

pub(super) fn open_confirmation(root: Obj, layers: Layers, motion: Motion) -> bool {
    open_with_transition(root, layers, false, Some(motion))
}

fn open_with_transition(
    root: Obj,
    layers: Layers,
    fade_only: bool,
    confirmation: Option<Motion>,
) -> bool {
    let Layers {
        surface,
        backdrop: image,
        content,
    } = layers;
    {
        let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
        if root.is_null() || state.phase != Phase::Hidden {
            if confirmation.is_some() {
                unsafe { lvx_object_delete(surface) };
            }
            return false;
        }
        *state = Dialog {
            surface,
            backdrop: image,
            content,
            phase: Phase::Capturing,
            fade_only,
            confirmation,
            ..EMPTY
        };
    }
    let source = if image.is_null() {
        core::ptr::null()
    } else {
        unsafe { backdrop::capture(root) }
    };
    let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
    if state.surface != surface || state.phase != Phase::Capturing {
        return false;
    }
    state.has_blur = !source.is_null();
    if state.has_blur {
        lv::style(surface, lv::BG_OPA, 0);
        unsafe {
            lvx_image_set_src(image, source);
            lvx_image_set_scale(image, backdrop::IMAGE_SCALE, backdrop::IMAGE_SCALE);
            lvx_object_align(image, ALIGN_CENTER, 0, 0);
            lvx_set_hidden(image, 0);
        }
    } else if !image.is_null() {
        unsafe { lvx_set_hidden(image, 1) };
    }
    render(state, Frame::HIDDEN);
    unsafe {
        lvx_object_move_to_index(state.surface, -1);
        lvx_set_hidden(state.surface, 0);
    }
    start(state, Phase::Opening);
    state.surface == surface
}

pub extern "C" fn dismiss(_event: Obj) {
    if let Some(layers) = unsafe { LIMIT } {
        close(layers);
    }
}

pub fn is_active(layers: Layers) -> bool {
    unsafe { DIALOG.surface == layers.surface }
}

pub fn close(layers: Layers) -> bool {
    let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
    if state.surface != layers.surface || !matches!(state.phase, Phase::Opening | Phase::Open) {
        return false;
    }
    begin_close(state);
    true
}

pub(super) fn dismiss_confirmation(root: Obj) -> bool {
    let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
    if !state.confirmation.is_some_and(|motion| motion.root == root)
        || !matches!(
            state.phase,
            Phase::Opening | Phase::Open | Phase::Expanding | Phase::Detail
        )
    {
        return false;
    }
    begin_close(state);
    true
}

pub(super) fn tap_confirmation(root: Obj) {
    let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
    if let Some(motion) = &mut state.confirmation {
        if motion.root == root
            && motion.has_detail()
            && matches!(motion.first_stage_tap, FirstStageTap::ShowDetails)
        {
            match state.phase {
                Phase::Opening => {
                    motion.first_stage_ms = 0;
                    return;
                }
                Phase::Open => {
                    start(state, Phase::Expanding);
                    return;
                }
                _ => {}
            }
        }
    }
    dismiss_confirmation(root);
}

pub(super) fn hide_confirmation(root: Obj) {
    let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
    if state.confirmation.is_some_and(|motion| motion.root == root) {
        remove_confirmation(state);
    }
}

pub(super) fn confirmation_deleted(surface: Obj) {
    let state = core::ptr::addr_of_mut!(DIALOG);
    if unsafe { (*state).surface == surface && (*state).confirmation.is_some() } {
        reset(unsafe { &mut *state });
    }
}

fn begin_close(state: &mut Dialog) {
    state.closing_from = state.frame.unwrap_or(Frame::HIDDEN);
    start(state, Phase::Closing);
}

pub fn dismiss_active(layers: Layers) -> bool {
    if !is_active(layers) {
        return false;
    }
    close(layers);
    true
}

pub fn cancel(layers: Layers) {
    let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
    if state.surface == layers.surface {
        reset(state);
    }
}

pub fn hide(layers: Layers) {
    unsafe { lvx_set_hidden(layers.surface, 1) };
    cancel(layers);
}

pub fn hide_confirm() {
    if let Some(layers) = unsafe { LIMIT } {
        if is_active(layers) {
            hide(layers);
        }
    }
}

pub fn confirming() -> bool {
    let state = unsafe { &*core::ptr::addr_of!(DIALOG) };
    unsafe { LIMIT }.is_some_and(|layers| {
        state.surface == layers.surface && matches!(state.phase, Phase::Opening | Phase::Open)
    })
}

pub fn stop() {
    unsafe { LIMIT = None };
    let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
    if state.confirmation.is_some() {
        remove_confirmation(state);
    } else {
        reset(state);
    }
}

fn remove_confirmation(state: &mut Dialog) {
    let surface = state.surface;
    reset(state);
    unsafe { lvx_object_delete(surface) };
}

fn reset(state: &mut Dialog) {
    cancel_timer(state);
    *state = EMPTY;
    backdrop::retire();
}

fn start(state: &mut Dialog, phase: Phase) {
    state.phase = phase;
    state.started_at = now_ms();
    if state.timer.is_null() {
        state.timer = unsafe { lvx_timer_create(tick, FRAME_MS, core::ptr::null_mut()) };
    }
    if state.timer.is_null()
        || state
            .confirmation
            .is_some_and(|motion| motion.reduced_motion)
    {
        finish(state);
    }
}

fn cancel_timer(state: &mut Dialog) {
    if !state.timer.is_null() {
        unsafe { lvx_timer_delete(state.timer) };
        state.timer = core::ptr::null_mut();
    }
}

extern "C" fn tick(_timer: Obj) {
    let state = unsafe { &mut *core::ptr::addr_of_mut!(DIALOG) };
    let elapsed = now_ms().wrapping_sub(state.started_at);
    let animation_time = if state.confirmation.is_some() {
        elapsed - elapsed / 4
    } else {
        elapsed
    };
    let (frame, duration) = match state.phase {
        Phase::Opening if state.fade_only => {
            let opacity = ease_out(elapsed, BACKDROP_MS);
            (
                Frame {
                    backdrop: opacity,
                    content: opacity,
                    icon: 255,
                    detail: 0,
                },
                BACKDROP_MS,
            )
        }
        Phase::Opening => (
            Frame {
                backdrop: ease_out(animation_time, BACKDROP_MS),
                content: ease_out(animation_time.saturating_sub(CONTENT_DELAY_MS), CONTENT_MS),
                icon: state
                    .confirmation
                    .map_or(255, |_| Motion::icon_progress(animation_time)),
                detail: 0,
            },
            state
                .confirmation
                .map_or(CONTENT_DELAY_MS + CONTENT_MS, |_| {
                    (CONTENT_DELAY_MS + CONTENT_MS).max(Motion::ICON_END_MS)
                }),
        ),
        Phase::Closing => (
            Frame {
                backdrop: fade_out(
                    state.closing_from.backdrop,
                    animation_time.saturating_sub(CLOSE_BACKDROP_DELAY_MS),
                    CLOSE_BACKDROP_MS,
                ),
                content: fade_out(state.closing_from.content, animation_time, CLOSE_CONTENT_MS),
                icon: state.closing_from.icon,
                detail: state.closing_from.detail,
            },
            CLOSE_BACKDROP_DELAY_MS + CLOSE_BACKDROP_MS,
        ),
        Phase::Open | Phase::Detail => {
            let Some(motion) = state.confirmation else {
                return;
            };
            if state.phase == Phase::Open && motion.has_detail() {
                if elapsed >= motion.first_stage_ms {
                    start(state, Phase::Expanding);
                }
            } else if motion.timeout_ms.is_some_and(|timeout| elapsed >= timeout) {
                begin_close(state);
            }
            return;
        }
        Phase::Expanding => {
            if animation_time >= DETAIL_DURATION_MS {
                finish(state);
            } else {
                render(
                    state,
                    Frame {
                        detail: ease_out(animation_time, DETAIL_DURATION_MS),
                        ..Frame::OPEN
                    },
                );
            }
            return;
        }
        _ => return,
    };
    if animation_time >= duration {
        finish(state);
    } else {
        render(state, frame);
    }
}

fn finish(state: &mut Dialog) {
    match state.phase {
        Phase::Opening => {
            render(state, Frame::OPEN);
            state.phase = Phase::Open;
            state.started_at = now_ms();
            if state
                .confirmation
                .is_some_and(|motion| motion.has_detail() || motion.timeout_ms.is_some())
            {
                if state.timer.is_null() {
                    remove_confirmation(state);
                }
            } else {
                cancel_timer(state);
            }
        }
        Phase::Expanding => {
            render(
                state,
                Frame {
                    detail: 255,
                    ..Frame::OPEN
                },
            );
            state.phase = Phase::Detail;
            state.started_at = now_ms();
            if state
                .confirmation
                .is_some_and(|motion| motion.timeout_ms.is_none())
            {
                cancel_timer(state);
            }
        }
        _ if state.confirmation.is_some() => remove_confirmation(state),
        _ => {
            unsafe { lvx_set_hidden(state.surface, 1) };
            reset(state);
        }
    }
}

fn render(state: &mut Dialog, frame: Frame) {
    if state
        .frame
        .is_none_or(|previous| previous.backdrop != frame.backdrop)
    {
        if state.has_blur {
            lv::style(state.backdrop, lv::OPA, frame.backdrop);
        } else {
            lv::style(state.surface, lv::BG_OPA, frame.backdrop);
        }
    }
    if state
        .frame
        .is_none_or(|previous| previous.content != frame.content)
    {
        lv::style(state.content, lv::OPA, frame.content);
    }
    if let Some(motion) = state.confirmation {
        if state.frame.is_none_or(|previous| {
            previous.content != frame.content
                || previous.icon != frame.icon
                || previous.detail != frame.detail
        }) {
            motion.paint(frame.content, frame.icon, frame.detail);
        }
        state.frame = Some(frame);
        return;
    }
    let offset = frame.content_offset(state.fade_only);
    if state
        .frame
        .is_none_or(|previous| previous.content_offset(state.fade_only) != offset)
    {
        lv::style(state.content, lv::TRANSLATE_Y, offset);
    }
    state.frame = Some(frame);
}

fn fade_out(from: u32, elapsed: u32, duration: u32) -> u32 {
    let progress = 255 * elapsed.min(duration) / duration;
    from * (255 - progress * progress / 255) / 255
}

// 三次减速曲线，输出可直接用于 LVGL 透明度。
pub(super) fn ease_out(elapsed: u32, duration: u32) -> u32 {
    let remaining = 255 * duration.saturating_sub(elapsed) / duration;
    255 - remaining * remaining * remaining / (255 * 255)
}
