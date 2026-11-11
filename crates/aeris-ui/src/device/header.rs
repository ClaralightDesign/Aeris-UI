//! 标题栏组件与计时器页面切换动画。

use crate::{
    geometry::Rect,
    header::{HeaderLayout, HeaderProps, HeaderVariant, TextAlign, TIME_FONT_PX, TITLE_FONT_PX},
    theme::Color,
};
use canopus_target_private::*;

use super::{
    anim, assets, font,
    lv::{self, Font, Obj},
    page,
};

const NULL: Obj = core::ptr::null_mut();
const FRAME_MS: u32 = 16;
const TRANSITION_MS: u32 = 360;
const FULL: i32 = 1000;
const RIGHT_INSET: i32 = 10;
const TITLE_TRAVEL: i32 = 24;

struct PaintedFrame {
    offsets: [i32; 4],
    opacity: [u8; 7],
    visible: [bool; 7],
    clickable: [bool; 3],
}

struct Header {
    settings: Obj,
    back: Obj,
    add: Obj,
    right: Obj,
    right_shadow: Obj,
    clock: Obj,
    clock_center_x: i32,
    clock_edge_x: i32,
    primary: Obj,
    secondary: Obj,
    quick_first: bool,
    progress: i32,
    from: i32,
    to: i32,
    started: u32,
    timer: Obj,
    painted: Option<PaintedFrame>,
}

static mut HEADER: Option<Header> = None;
static mut SECONDARY_HEADER: Option<Header> = None;

unsafe fn state() -> &'static mut Option<Header> {
    unsafe { &mut *core::ptr::addr_of_mut!(HEADER) }
}

unsafe fn secondary_state() -> &'static mut Option<Header> {
    unsafe { &mut *core::ptr::addr_of_mut!(SECONDARY_HEADER) }
}

fn layout(has_add: bool) -> HeaderLayout {
    let variant = if has_add {
        HeaderVariant::Center
    } else {
        HeaderVariant::Right
    };
    HeaderLayout::measure(HeaderProps::new("", "", variant))
}

fn text_area(mut rect: Rect, align: TextAlign) -> [i32; 4] {
    if align == TextAlign::End {
        rect.x += RIGHT_INSET;
        rect.w -= 2 * RIGHT_INSET;
    }
    [rect.x, rect.y, rect.w, rect.h]
}

fn text_align(align: TextAlign) -> u32 {
    match align {
        TextAlign::Start => 1,
        TextAlign::Center => 2,
        TextAlign::End => 3,
    }
}

unsafe fn new_title(root: Obj, title: &[u8], layout: HeaderLayout) -> Obj {
    let [x, y, width, height] = text_area(layout.title.unwrap(), layout.title_align);
    let object = unsafe {
        lv::label(
            root,
            title,
            x,
            y,
            width,
            height,
            Color::Accent.resolve(),
            Font::Small24,
        )
    };
    set_title_style(object, width, height, layout.title_align);
    object
}

fn set_title_style(object: Obj, width: i32, height: i32, align: TextAlign) {
    let align = text_align(align);
    lv::style(object, 94, align);
    lv::text_px(object, width, height, align, Font::Small24, TITLE_FONT_PX);
}

unsafe fn place_title(object: Obj, title: &[u8], layout: HeaderLayout) {
    if object.is_null() {
        return;
    }
    let [x, y, width, height] = text_area(layout.title.unwrap(), layout.title_align);
    unsafe {
        lvx_label_set_text(object, title.as_ptr());
        lvx_object_set_size(object, width, height);
        lvx_object_align(object, 1, x, y);
    }
    set_title_style(object, width, height, layout.title_align);
}

pub unsafe fn back(root: Obj, title: &[u8], on_back: LvxEventCallback, about: bool) {
    let layout = layout(false);
    let left = layout.left_button.unwrap();
    unsafe {
        if !about {
            background_half(root, assets::HEADER_LEFT, 0);
            background_half(root, assets::HEADER_BACKGROUND, page::WIDTH / 2);
        }
        let face = if about {
            assets::BUTTON_ABOUT
        } else {
            assets::BUTTON_BLACK
        };
        lv::bind(
            lv::header_button(root, face, assets::ICON_BACK, left.x, left.y).root,
            on_back,
            0,
        );
        right_clock(root);
        new_title(root, title, layout);
    }
}

pub unsafe fn right_clock(root: Obj) {
    let layout = layout(false);
    unsafe {
        font::clock(
            root,
            &page::clock_bytes(),
            text_area(layout.time, layout.time_align),
            text_align(layout.time_align),
        );
    }
}

pub unsafe fn timer_clock(root: Obj, dark: bool) {
    let layout = HeaderLayout::measure(HeaderProps::new("", "", HeaderVariant::Hidden));
    let time = page::clock_bytes();
    let area = [0, layout.time.y, layout.frame.w, layout.time.h];
    unsafe {
        if dark {
            if font::fixed(root, font::CLOCK_DARK, &time, area, 2).is_none() {
                let [x, y, w, h] = area;
                let label = lv::label(root, &time, x, y, w, h, 0, Font::Small24);
                lv::text_px(label, w, h, 2, Font::Small24, TIME_FONT_PX);
            }
        } else {
            font::clock(root, &time, area, 2);
        }
    }
}

pub unsafe fn build(
    root: Obj,
    on_settings: LvxEventCallback,
    on_back: LvxEventCallback,
    on_add: LvxEventCallback,
) {
    let centered = layout(true);
    let left = centered.left_button.unwrap();
    let right_button = centered.right_button.unwrap();
    unsafe {
        background_half(root, assets::HEADER_LEFT, 0);
        let right = background_half(root, assets::HEADER_BACKGROUND, page::WIDTH / 2);
        let right_shadow = background_half(root, assets::HEADER_RIGHT, page::WIDTH / 2);
        let settings = lv::header_button(
            root,
            assets::BUTTON_BLACK,
            assets::ICON_SETTINGS,
            left.x,
            left.y,
        )
        .root;
        let back = lv::header_button(
            root,
            assets::BUTTON_BLACK,
            assets::ICON_BACK,
            left.x,
            left.y,
        )
        .root;
        let add = lv::header_button(
            root,
            assets::BUTTON_PRIMARY,
            assets::ICON_ADD_ORANGE,
            right_button.x,
            right_button.y,
        )
        .root;
        lv::bind(settings, on_settings, 0);
        lv::bind(back, on_back, 0);
        lv::bind(add, on_add, root as usize);

        let (clock, clock_center_x, clock_edge_x) = moving_clock(root, centered);
        let primary = new_title(root, b"\0", centered);
        let secondary = new_title(root, b"\0", centered);
        *state() = Some(Header {
            settings,
            back,
            add,
            right,
            right_shadow,
            clock,
            clock_center_x,
            clock_edge_x,
            primary,
            secondary,
            quick_first: false,
            progress: 0,
            from: 0,
            to: 0,
            started: 0,
            timer: NULL,
            painted: None,
        });
    }
}

pub unsafe fn build_secondary(
    root: Obj,
    titles: [&[u8]; 2],
    on_back: LvxEventCallback,
    action_icon: &[u8],
    on_action: LvxEventCallback,
) {
    let centered = layout(true);
    let left = centered.left_button.unwrap();
    let right_button = centered.right_button.unwrap();
    unsafe {
        background_half(root, assets::HEADER_LEFT, 0);
        let right = background_half(root, assets::HEADER_BACKGROUND, page::WIDTH / 2);
        let right_shadow = background_half(root, assets::HEADER_RIGHT, page::WIDTH / 2);
        let back = lv::header_button(
            root,
            assets::BUTTON_BLACK,
            assets::ICON_BACK,
            left.x,
            left.y,
        );
        let action = lv::header_button(
            root,
            assets::BUTTON_PRIMARY,
            action_icon,
            right_button.x,
            right_button.y,
        );
        lv::bind(back.root, on_back, 0);
        lv::bind(action.root, on_action, 0);

        let (clock, clock_center_x, clock_edge_x) = moving_clock(root, centered);
        let primary = new_title(root, titles[0], centered);
        let secondary = new_title(root, titles[1], layout(false));
        let mut header = Header {
            settings: NULL,
            back: NULL,
            add: action.root,
            right,
            right_shadow,
            clock,
            clock_center_x,
            clock_edge_x,
            primary,
            secondary,
            quick_first: false,
            progress: 0,
            from: 0,
            to: 0,
            started: 0,
            timer: NULL,
            painted: None,
        };
        header.finish();
        *secondary_state() = Some(header);
    }
}

unsafe fn moving_clock(root: Obj, centered: HeaderLayout) -> (Obj, i32, i32) {
    let time = page::clock_bytes();
    let (clock, bitmap) = unsafe {
        font::clock(
            root,
            &time,
            [0, centered.time.y, centered.time.w, centered.time.h],
            3,
        )
    };
    let clock_width = if bitmap {
        core::str::from_utf8(&time[..5])
            .ok()
            .and_then(|text| font::CLOCK.atlas.width(text))
            .unwrap_or(66)
    } else {
        66
    };
    (
        clock,
        centered.time.center_x() + clock_width / 2 - centered.time.w,
        layout(false).time.right() - RIGHT_INSET - centered.time.w,
    )
}

unsafe fn background_half(root: Obj, asset: &[u8], x: i32) -> Obj {
    unsafe {
        let image = lv::image(root, asset, x, 0);
        if !image.is_null() {
            lvx_object_clear_flag(image, lv::CLICKABLE | lv::SCROLLABLE);
        }
        image
    }
}

pub fn configure(quick_first: bool, mine: &[u8], quick: &[u8]) {
    let Some(header) = (unsafe { state() }).as_mut() else {
        return;
    };
    header.quick_first = quick_first;
    let (primary, secondary) = if quick_first {
        (quick, mine)
    } else {
        (mine, quick)
    };
    unsafe {
        place_title(header.primary, primary, layout(!quick_first));
        place_title(header.secondary, secondary, layout(quick_first));
    }
    header.painted = None;
}

pub fn show(secondary: bool, animated: bool) {
    if let Some(header) = (unsafe { state() }).as_mut() {
        header.show(secondary, animated, tick);
    }
}

pub fn show_secondary(secondary: bool) {
    if let Some(header) = (unsafe { secondary_state() }).as_mut() {
        header.show(secondary, true, tick_secondary);
    }
}

pub fn settle_secondary() {
    if let Some(header) = (unsafe { secondary_state() }).as_mut() {
        header.finish();
    }
}

pub fn stop_secondary() {
    if let Some(mut header) = (unsafe { secondary_state() }).take() {
        header.stop_timer();
    }
}

pub fn settle() {
    if let Some(header) = unsafe { state() }.as_mut() {
        header.finish();
    }
}

pub fn stop() {
    if let Some(mut header) = unsafe { state() }.take() {
        header.stop_timer();
    }
}

extern "C" fn tick(_timer: Obj) {
    if let Some(header) = (unsafe { state() }).as_mut() {
        header.advance();
    }
}

extern "C" fn tick_secondary(_timer: Obj) {
    if let Some(header) = (unsafe { secondary_state() }).as_mut() {
        header.advance();
    }
}

impl Header {
    fn show(&mut self, secondary: bool, animated: bool, tick: extern "C" fn(Obj)) {
        self.to = if secondary { FULL } else { 0 };
        if animated && self.progress != self.to {
            self.from = self.progress;
            self.started = anim::now_ms();
            if self.timer.is_null() {
                self.timer = unsafe { lvx_timer_create(tick, FRAME_MS, NULL) };
            }
            if !self.timer.is_null() {
                self.paint(false);
                return;
            }
        }
        self.finish();
    }

    fn advance(&mut self) {
        let elapsed = anim::now_ms().wrapping_sub(self.started);
        if elapsed >= TRANSITION_MS {
            self.finish();
        } else {
            let remaining = FULL - (elapsed * FULL as u32 / TRANSITION_MS) as i32;
            let p = FULL - remaining * remaining / FULL;
            self.progress = self.from + (self.to - self.from) * p / FULL;
            self.paint(false);
        }
    }

    fn finish(&mut self) {
        self.stop_timer();
        self.progress = self.to;
        self.paint(true);
    }

    fn stop_timer(&mut self) {
        if !self.timer.is_null() {
            unsafe { lvx_timer_delete(self.timer) };
            self.timer = NULL;
        }
    }

    fn paint(&mut self, settled: bool) {
        let p = self.progress;
        let primary = FULL - p;
        let add = if self.quick_first { p } else { primary };
        let travel = if self.quick_first {
            -TITLE_TRAVEL
        } else {
            TITLE_TRAVEL
        };
        let fade = [primary, p, FULL - add, add, primary, p, add];
        let frame = PaintedFrame {
            offsets: [
                self.clock_center_x
                    + (self.clock_edge_x - self.clock_center_x) * (FULL - add) / FULL,
                travel * p / FULL,
                -travel * primary / FULL,
                TITLE_TRAVEL * (FULL - add) / FULL,
            ],
            opacity: fade.map(|amount| (255 * amount / FULL) as u8),
            visible: fade.map(|amount| amount != 0),
            clickable: [primary, p, add].map(|amount| settled && amount == FULL),
        };
        let previous = self.painted.as_ref();
        for (index, object) in [self.clock, self.primary, self.secondary, self.add]
            .into_iter()
            .enumerate()
        {
            let offset = frame.offsets[index];
            if previous.is_none_or(|old| old.offsets[index] != offset) {
                lv::style(object, lv::TRANSLATE_X, offset as u32);
            }
        }
        for (index, object) in [
            self.primary,
            self.secondary,
            self.right,
            self.right_shadow,
            self.settings,
            self.back,
            self.add,
        ]
        .into_iter()
        .enumerate()
        {
            if object.is_null() {
                continue;
            }
            let opacity = frame.opacity[index];
            if previous.is_none_or(|old| old.opacity[index] != opacity) {
                lv::style(object, lv::OPA, u32::from(opacity));
            }
            let visible = frame.visible[index];
            if previous.is_none_or(|old| old.visible[index] != visible) {
                unsafe { lvx_set_hidden(object, u32::from(!visible)) };
            }
        }
        for (index, object) in [self.settings, self.back, self.add].into_iter().enumerate() {
            let clickable = frame.clickable[index];
            if !object.is_null() && previous.is_none_or(|old| old.clickable[index] != clickable) {
                unsafe {
                    if clickable {
                        lvx_object_add_flag(object, lv::CLICKABLE);
                    } else {
                        lvx_object_clear_flag(object, lv::CLICKABLE);
                    }
                }
            }
        }
        self.painted = Some(frame);
    }
}
