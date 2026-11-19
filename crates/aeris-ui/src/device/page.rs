//! 屏幕、滚动容器、返回标题栏及按钮模板。

use super::{
    header,
    lv::{self, Font, Obj},
};
use crate::{
    capsule::CapsuleKind,
    geometry::{DisplayProfile, Rect, Viewport},
    theme::{self, Color},
    Apx,
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
    #[cfg(feature = "band-10-pro")]
    let local_time = generated_symbols::CALENDAR_LOCALTIME_CALLABLE;
    #[cfg(all(feature = "band-11", not(feature = "band-10-pro")))]
    let local_time = canopus_target_generated::CANOPUS_FW_CALENDAR_LOCALTIME_CALLABLE;
    let local_time: LocalTime = unsafe { core::mem::transmute(local_time) };
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

#[deprecated(note = "Use header::HeaderBar with HeaderStyle::Right")]
pub unsafe fn right_header(root: Obj, title: &str, on_back: LvxEventCallback) {
    unsafe { header(root, title, on_back, false) };
}

pub unsafe fn header(root: Obj, title: &str, on_back: LvxEventCallback, about: bool) {
    unsafe { header::back(root, title.as_bytes(), on_back, about) };
}

pub unsafe fn screen(parent: Obj) -> Obj {
    unsafe { screen_in(parent, super::display_profile().viewport) }
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
    unsafe { scroll_in(parent, super::display_profile().viewport.bounds()) }
}

/// A mounted header or footer supplies its actual visible height to scrolling
/// content. Callers do not need to duplicate device-specific bar measurements.
pub trait PageBar {
    fn scroll_inset(&self) -> i32;
}

/// Applies the default content padding from the mounted bars. This also works
/// for pages in a horizontal pager. Child positions start at the padded origin;
/// LVGL includes the bottom inset when calculating the final scroll position.
pub fn scroll_bars(scroll: Obj, header: Option<&dyn PageBar>, footer: Option<&dyn PageBar>) {
    lv::style(
        scroll,
        16,
        header.map_or(0, |bar| bar.scroll_inset()) as u32,
    );
    lv::style(
        scroll,
        17,
        footer.map_or(0, |bar| bar.scroll_inset()) as u32,
    );
}

/// A full-screen scrolling surface with automatic padding for the supplied bars.
pub unsafe fn scroll_with_bars(
    parent: Obj,
    viewport: Viewport,
    header: Option<&dyn PageBar>,
    footer: Option<&dyn PageBar>,
) -> Obj {
    let scroll = unsafe { scroll_in(parent, viewport.bounds()) };
    scroll_bars(scroll, header, footer);
    scroll
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonStyle {
    #[default]
    Regular,
    Secondary,
    Primary,
}

impl ButtonStyle {
    pub const fn height(self) -> i32 {
        match self {
            Self::Regular => 68,
            Self::Secondary => 80,
            Self::Primary => 82,
        }
    }

    pub fn height_in(self, viewport: Viewport) -> i32 {
        if viewport == DisplayProfile::BAND_11.viewport {
            viewport.apx(Apx::new(if self == Self::Regular { 11 } else { 13 }))
        } else if viewport == DisplayProfile::REDMI_WATCH.viewport {
            if self == Self::Regular {
                76
            } else {
                90
            }
        } else {
            self.height()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonOptions {
    pub style: ButtonStyle,
    pub dimmed: bool,
}

pub use crate::theme::ButtonTheme;

macro_rules! button_slices {
    ($name:literal) => {
        [
            concat!("button-", $name, "-left.bin\0").as_bytes(),
            concat!("button-", $name, "-center.bin\0").as_bytes(),
            concat!("button-", $name, "-right.bin\0").as_bytes(),
        ]
    };
}

pub(super) const NORMAL_BUTTON_SLICES: [&[u8]; 3] = button_slices!("normal");
pub(super) const SECONDARY_BUTTON_SLICES: [&[u8]; 3] = button_slices!("secondary");

pub unsafe fn button_with_options(
    parent: Obj,
    bounds: Rect,
    title: &str,
    action: LvxEventCallback,
    data: usize,
    options: ButtonOptions,
) -> Obj {
    unsafe {
        button_with_theme(
            parent,
            super::display_profile().viewport,
            bounds,
            title,
            action,
            data,
            options,
            ButtonTheme::default(),
        )
    }
}

/// Creates a sized action with the selected face. Dimmed actions remain clickable.
///
/// # Safety
/// Call on the UI thread with a live parent.
pub unsafe fn button_with_theme(
    parent: Obj,
    viewport: Viewport,
    bounds: Rect,
    title: &str,
    action: LvxEventCallback,
    data: usize,
    options: ButtonOptions,
    theme: ButtonTheme,
) -> Obj {
    let style = options.style;
    let height = style.height_in(viewport);
    let Some(mut area) = bounds.horizontal_inset(3).filter(|area| area.h >= height) else {
        return core::ptr::null_mut();
    };
    area.h = height;
    let primary = style == ButtonStyle::Primary;
    let opacity = if options.dimmed { 128 } else { 255 };
    let (slices, source_height, face_opacity) = match (style, theme, options.dimmed) {
        (ButtonStyle::Regular, _, _) => (
            button_slices!("regular"),
            68,
            if options.dimmed { 15 } else { 31 },
        ),
        (ButtonStyle::Secondary, _, _) => (
            SECONDARY_BUTTON_SLICES,
            80,
            if options.dimmed { 15 } else { 31 },
        ),
        (_, ButtonTheme::Normal, false) => (NORMAL_BUTTON_SLICES, 80, 255),
        (_, ButtonTheme::Normal, true) => (button_slices!("normal-dimmed"), 80, 255),
        _ => (
            [
                super::assets::CAPSULE_PRIMARY_LEFT,
                super::assets::CAPSULE_PRIMARY_CENTER_WIDE,
                super::assets::CAPSULE_PRIMARY_RIGHT,
            ],
            82,
            opacity,
        ),
    };
    unsafe {
        let button = lv::capsule_face(
            parent,
            area,
            slices,
            source_height,
            face_opacity,
            primary && theme == ButtonTheme::Glass,
        );
        if button.is_null() {
            return button;
        }
        lv::inspect_name(button, c"AerisUI.Button");
        let text_px = viewport.apx(Apx::new(4));
        let line_height = viewport.apx(Apx::new(5));
        let font = if text_px >= 28 {
            Font::Medium28
        } else {
            Font::Medium24
        };
        let inset = viewport.apx(Apx::new(2)).min((area.w - 1) / 2);
        let text = Rect::new(
            inset,
            (height - line_height) / 2,
            area.w - 2 * inset,
            line_height,
        );
        let color = if primary {
            theme::accent_button().map_or(0xD7E8FF, |button| button.foreground)
        } else {
            Color::PrimaryText.resolve()
        };
        let label = lv::label_str(button, title, text, color, font);
        if !label.is_null() {
            lv::text_apx_in(
                label,
                viewport,
                text.w,
                text.h,
                2,
                font,
                crate::TextSize::Body.apx(),
            );
            lvx_label_set_long_mode(label, 1);
            lv::style(label, lv::TEXT_OPA, opacity);
        }
        lv::bind(button, action, data);
        button
    }
}

#[deprecated(note = "Use page::button_with_options and ButtonOptions")]
pub unsafe fn button(
    parent: Obj,
    y: i32,
    title: &str,
    kind: CapsuleKind,
    action: LvxEventCallback,
) -> Obj {
    unsafe {
        build_button(
            parent,
            Rect::new(0, y, WIDTH, kind.height()),
            title,
            kind,
            kind.height(),
            action,
            0,
            false,
        )
    }
}

#[deprecated(note = "Use page::button_with_options and ButtonOptions")]
pub unsafe fn button_with_data(
    parent: Obj,
    y: i32,
    title: &str,
    kind: CapsuleKind,
    action: LvxEventCallback,
    data: usize,
) -> Obj {
    unsafe {
        build_button(
            parent,
            Rect::new(0, y, WIDTH, kind.height()),
            title,
            kind,
            kind.height(),
            action,
            data,
            false,
        )
    }
}

#[deprecated(note = "Use page::button_with_options and ButtonOptions")]
pub unsafe fn button_in(
    parent: Obj,
    bounds: Rect,
    title: &str,
    kind: CapsuleKind,
    action: LvxEventCallback,
    data: usize,
) -> Obj {
    unsafe {
        build_button(
            parent,
            bounds,
            title,
            kind,
            kind.height(),
            action,
            data,
            false,
        )
    }
}

unsafe fn build_button(
    parent: Obj,
    bounds: Rect,
    title: &str,
    kind: CapsuleKind,
    height: i32,
    action: LvxEventCallback,
    data: usize,
    dimmed: bool,
) -> Obj {
    let Some(area) = bounds.horizontal_inset(3).filter(|area| area.h >= height) else {
        return core::ptr::null_mut();
    };
    unsafe {
        let opacity = if dimmed { 128 } else { 255 };
        let button = lv::build_capsule(parent, area.x, area.y, area.w, height, kind, opacity);
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
                (height - 32) / 2,
                area.w,
                32,
                color,
                Font::Medium24,
            );
            lv::text_apx(
                label,
                area.w,
                32,
                2,
                Font::Medium24,
                crate::TextSize::Body.apx(),
            );
            if dimmed {
                lv::style(label, lv::TEXT_OPA, opacity);
            }
            lv::bind(button, action, data);
        }
        button
    }
}
