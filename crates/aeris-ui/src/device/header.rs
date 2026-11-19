//! 标题栏组件与计时器页面切换动画。

use crate::{
    geometry::{DisplayProfile, Rect, Viewport},
    header::{
        title_opacities, HeaderAction, HeaderLayout, HeaderProps, HeaderVariant,
        ShrinkingTitleLayout, TextAlign, BUTTON_SIZE, HEADER_HEIGHT, ICON_SIZE,
        LARGE_TITLE_SCROLL_PX,
    },
    theme::Color,
    Apx,
};
use canopus_target_private::*;

use super::{
    anim, assets, font,
    lv::{self, Font, Obj},
    page, round_button,
};

const NULL: Obj = core::ptr::null_mut();
const FRAME_MS: u32 = 16;
const TRANSITION_MS: u32 = 360;
const FULL: i32 = 1000;
const RIGHT_INSET: i32 = 10;
const TITLE_TRAVEL: i32 = 24;
const SMALL_TITLE_RISE_PX: u32 = 8;
const SHRINKING_SOURCE_PX: i32 = if cfg!(any(feature = "gallery", target_os = "emscripten")) {
    44
} else {
    28
};
const SHRINKING_SOURCE_HEIGHT: i32 = if SHRINKING_SOURCE_PX == 44 { 53 } else { 36 };

pub use crate::header::HeaderStyle;

#[derive(Clone, Copy, Default)]
pub struct HeaderOptions {
    pub style: HeaderStyle,
    pub large_title: bool,
    pub on_left: Option<LvxEventCallback>,
    pub on_right: Option<LvxEventCallback>,
}

/// A fixed header or footer with an optional large-title area in the scroll container.
pub struct HeaderBar {
    root: Obj,
    height: i32,
    small_title: Obj,
    large_title: Option<LargeTitle>,
    shrinking_title: Option<ShrinkingTitleLayout>,
}

struct LargeTitle {
    area: Obj,
    label: Obj,
}

impl LargeTitle {
    unsafe fn new(scroll: Obj, title: &str, width: i32) -> Option<Self> {
        let area = unsafe { font::clip(scroll, width, LARGE_TITLE_SCROLL_PX, 0) }?;
        let inset = if width < 220 { 8 } else { 24 };
        let label = unsafe {
            lvx_object_align(area, 1, 0, 0);
            lv::inspect_name(area, c"AerisUI.LargeTitleArea");
            super::cards::text_font(
                area,
                title,
                inset,
                8,
                width - 2 * inset,
                60,
                Color::Accent.resolve(),
                Font::Small24,
                44,
            )
        };
        if label.is_null() {
            unsafe { lvx_object_delete(area) };
            return None;
        }

        #[cfg(any(feature = "gallery", target_os = "emscripten"))]
        {
            super::linoval::apply_large_title(label);
            let px = crate::TextSize::LargeTitle.pixels(super::display_profile().viewport);
            let source_width = ((width - 2 * inset) * 44 + px - 1) / px;
            unsafe { lvx_object_set_size(label, source_width, 53) };
            lv::style(label, lv::TRANSFORM_PIVOT_X, 0);
            lv::style(label, lv::TRANSFORM_PIVOT_Y, 0);
            lv::style(label, lv::TRANSFORM_SCALE_X, ((px * 256 + 22) / 44) as u32);
            lv::style(label, lv::TRANSFORM_SCALE_Y, ((px * 256 + 22) / 44) as u32);
        }
        #[cfg(not(any(feature = "gallery", target_os = "emscripten")))]
        lv::text_apx(
            label,
            width - 2 * inset,
            60,
            1,
            Font::Small24,
            crate::TextSize::LargeTitle.apx(),
        );
        lv::inspect_text_apx(
            label,
            super::display_profile().viewport,
            crate::TextSize::LargeTitle.apx(),
        );
        unsafe { lvx_label_set_long_mode(label, 1) };
        Some(Self { area, label })
    }
}

impl HeaderBar {
    /// Creates a header. Pass the scroll container when enabling a large title.
    /// Use [`Self::content_top`] to place page content below the title area.
    ///
    /// # Safety
    /// Call on the UI thread. The page and optional scroll container must
    /// remain valid while using the resulting header.
    pub unsafe fn new(
        page_root: Obj,
        title: &str,
        scroll: Option<Obj>,
        options: HeaderOptions,
    ) -> Option<Self> {
        unsafe {
            Self::new_for_viewport(
                page_root,
                super::display_profile().viewport,
                title,
                scroll,
                options,
            )
        }
    }

    /// Places a header using the viewport's full edge and content geometry.
    ///
    /// # Safety
    /// Call on the UI thread with live page and scroll objects.
    pub unsafe fn new_for_viewport(
        page_root: Obj,
        viewport: Viewport,
        title: &str,
        scroll: Option<Obj>,
        options: HeaderOptions,
    ) -> Option<Self> {
        if viewport == DisplayProfile::BAND_11.viewport {
            return unsafe { Self::capsule(page_root, viewport, title, scroll, options) };
        }
        unsafe { Self::regular(page_root, viewport.content(), title, scroll, options) }
    }

    /// Places the bar at the top or bottom edge of the supplied content bounds.
    /// An optional scroll container for the large title must have the same width.
    ///
    /// # Safety
    /// The page and scroll objects must remain valid. Use this on the UI thread.
    pub unsafe fn new_in(
        page_root: Obj,
        bounds: Rect,
        title: &str,
        scroll: Option<Obj>,
        options: HeaderOptions,
    ) -> Option<Self> {
        let viewport = super::display_profile().viewport;
        if viewport == DisplayProfile::BAND_11.viewport
            && (bounds == viewport.bounds() || bounds == viewport.content())
        {
            return unsafe { Self::capsule(page_root, viewport, title, scroll, options) };
        }
        unsafe { Self::regular(page_root, bounds, title, scroll, options) }
    }

    unsafe fn regular(
        page_root: Obj,
        bounds: Rect,
        title: &str,
        scroll: Option<Obj>,
        options: HeaderOptions,
    ) -> Option<Self> {
        if bounds.h < HEADER_HEIGHT {
            return None;
        }
        let supports_large_title = options.large_title && options.style.supports_large_title();
        let scrolling_title = if supports_large_title {
            Some(scroll.filter(|object| !object.is_null())?)
        } else {
            None
        };
        let (variant, simple) = match options.style {
            HeaderStyle::Top | HeaderStyle::Bottom => (HeaderVariant::Center, false),
            HeaderStyle::Left => (HeaderVariant::Left, false),
            HeaderStyle::Right | HeaderStyle::Navigation => (HeaderVariant::Right, false),
            HeaderStyle::Simple => (HeaderVariant::Left, true),
            HeaderStyle::CenterSimple => (HeaderVariant::Hidden, false),
            HeaderStyle::HeaderButton | HeaderStyle::FooterButton => return None,
        };
        let mut props = HeaderProps::new("", "", variant);
        if simple {
            props.right = HeaderAction::None;
        }
        let layout = HeaderLayout::measure_in(props, bounds.w)?;
        let bottom = options.style == HeaderStyle::Bottom;
        let y = if bottom {
            bounds.bottom() - HEADER_HEIGHT
        } else {
            bounds.y
        };
        let stock_width = bounds.w == page::WIDTH;
        let root = unsafe { font::clip(page_root, bounds.w, HEADER_HEIGHT, 0) }?;
        unsafe {
            lvx_object_align(root, 1, bounds.x, y);
            lv::inspect_name(root, c"AerisUI.HeaderBar");
            if !stock_width {
                lv::style(root, lv::BG_COLOR, Color::Background.resolve());
                lv::style(root, lv::BG_OPA, 224);
            } else if bottom {
                background_half(root, assets::FOOTER_BACKGROUND, 0);
            } else {
                let left = if variant.uses_left_shadow() {
                    assets::HEADER_LEFT
                } else {
                    assets::HEADER_BACKGROUND
                };
                let right = if variant.uses_right_shadow() && !simple {
                    assets::HEADER_RIGHT
                } else {
                    assets::HEADER_BACKGROUND
                };
                background_half(root, left, 0);
                background_half(root, right, page::WIDTH / 2);
            }
            for (action, area, callback) in [
                (props.left, layout.left_button, options.on_left),
                (props.right, layout.right_button, options.on_right),
            ] {
                let Some(area) = area else { continue };
                let (style, icon) = match action {
                    HeaderAction::None => continue,
                    HeaderAction::Back => (lv::RoundButtonStyle::Neutral, assets::ICON_BACK),
                    HeaderAction::Close => (lv::RoundButtonStyle::Neutral, assets::ICON_X),
                    HeaderAction::Settings => {
                        (lv::RoundButtonStyle::Neutral, assets::ICON_SETTINGS)
                    }
                    HeaderAction::Add => (lv::RoundButtonStyle::Accent, assets::ICON_ADD),
                };
                let mut area = area;
                if bottom {
                    area.y += 14;
                }
                let button = if area.w == BUTTON_SIZE {
                    lv::round_button_with_style(root, area.x, area.y, icon, style).root
                } else {
                    compact_action(root, area, icon, style)
                };
                if let Some(callback) = callback {
                    lv::bind(button, callback, 0);
                } else if !button.is_null() {
                    lvx_object_clear_flag(button, lv::CLICKABLE);
                }
            }

            let mut clock = layout.time;
            let mut clock_align = layout.time_align;
            if bottom {
                clock.y += 14;
            }
            if simple {
                clock.x = 0;
                clock.w = bounds.w + if stock_width { RIGHT_INSET - 6 } else { -6 };
                clock_align = TextAlign::End;
            }
            if options.style == HeaderStyle::CenterSimple {
                clock.x = 0;
                clock.w = bounds.w;
                clock_align = TextAlign::Center;
            }
            let (clock_label, bitmap) = font::clock(
                root,
                &page::clock_bytes(),
                text_area(clock, clock_align),
                text_align(clock_align),
            );
            if !bitmap {
                lv::style(clock_label, 94, text_align(clock_align));
            }

            let small_title = if simple {
                NULL
            } else if let Some(area) = layout.title {
                let [x, y, w, h] = text_area(area, layout.title_align);
                let label = super::cards::text_font(
                    root,
                    title,
                    x,
                    y + if bottom { 14 } else { 0 },
                    w,
                    h,
                    Color::Accent.resolve(),
                    Font::Small24,
                    crate::TextSize::Body.pixels(super::display_profile().viewport),
                );
                lv::inspect_name(label, c"AerisUI.HeaderTitle");
                lv::style(label, 94, text_align(layout.title_align));
                lv::inspect_text_apx(
                    label,
                    super::display_profile().viewport,
                    crate::TextSize::Body.apx(),
                );
                if !stock_width && !label.is_null() {
                    lvx_label_set_long_mode(label, 1);
                }
                label
            } else {
                NULL
            };
            let large_title = if let Some(scroll) = scrolling_title {
                if small_title.is_null() {
                    lvx_object_delete(root);
                    return None;
                }
                let Some(large_title) = LargeTitle::new(scroll, title, bounds.w) else {
                    lvx_object_delete(root);
                    return None;
                };
                lvx_object_move_to_index(root, -1);
                Some(large_title)
            } else {
                None
            };
            let header = Self {
                root,
                height: HEADER_HEIGHT,
                small_title,
                large_title,
                shrinking_title: None,
            };
            header.update_scroll(scrolling_title.map_or(0, |scroll| lvx_object_scroll_y(scroll)));
            Some(header)
        }
    }

    unsafe fn capsule(
        parent: Obj,
        viewport: Viewport,
        title: &str,
        scroll: Option<Obj>,
        options: HeaderOptions,
    ) -> Option<Self> {
        let bounds = viewport.bounds();
        let safe = viewport.content();
        let edge = match options.style {
            HeaderStyle::HeaderButton => Some((
                round_button::Position::Top,
                round_button::Status::Neutral,
                round_button::Icon::Settings,
                options.on_left,
            )),
            HeaderStyle::FooterButton | HeaderStyle::Bottom => Some((
                round_button::Position::Bottom,
                round_button::Status::Primary,
                round_button::Icon::Settings,
                options.on_right.or(options.on_left),
            )),
            HeaderStyle::Left => Some((
                round_button::Position::Top,
                round_button::Status::Neutral,
                round_button::Icon::Close,
                options.on_right,
            )),
            HeaderStyle::Right => Some((
                round_button::Position::Top,
                round_button::Status::Neutral,
                round_button::Icon::Back,
                options.on_left,
            )),
            _ => None,
        };
        let height = height_in(viewport);
        let shrinking =
            (options.large_title && options.style.supports_large_title() && edge.is_none())
                .then(|| ShrinkingTitleLayout::new(viewport));
        let root = unsafe { font::clip(parent, bounds.w, height, 0) }?;
        let bottom = edge.is_some_and(|(position, ..)| position == round_button::Position::Bottom);
        let title_label = unsafe {
            lvx_object_align(root, 1, 0, if bottom { bounds.h - height } else { 0 });
            lv::inspect_name(root, c"AerisUI.HeaderBar");
            if edge.is_none() {
                // Reuse the translucent gradient, retaining content behind the header.
                for x in [0, bounds.w / 2] {
                    let image = background_half(root, assets::HEADER_BACKGROUND, x);
                    if !image.is_null() {
                        lv::style(image, lv::TRANSFORM_PIVOT_X, 0);
                        lv::style(image, lv::TRANSFORM_PIVOT_Y, 0);
                        lvx_image_set_scale(
                            image,
                            (bounds.w * 256 + page::WIDTH - 1) / page::WIDTH,
                            (height * 256 + HEADER_HEIGHT - 1) / HEADER_HEIGHT,
                        );
                    }
                }
            }
            if let Some((position, status, icon, action)) = edge {
                let slot = Rect::new(0, 4, bounds.w, height - 8);
                if round_button::create(
                    root,
                    slot,
                    icon,
                    round_button::Options {
                        position,
                        status,
                        ..Default::default()
                    },
                    action,
                    0,
                )
                .is_none()
                {
                    lvx_object_delete(root);
                    return None;
                }
                NULL
            } else {
                if arc_clock(root, viewport).is_none() {
                    lvx_object_delete(root);
                    return None;
                }
                if matches!(options.style, HeaderStyle::Top | HeaderStyle::Navigation) {
                    if let Some(layout) = shrinking {
                        // Keep a single high-resolution label for the whole transition.
                        // Its layout width is unscaled; both visual bounds and glyphs
                        // then shrink around the same horizontal centre.
                        let available = bounds.w - 2 * viewport.apx(Apx::new(2));
                        let width = (available * SHRINKING_SOURCE_PX + layout.expanded_px - 1)
                            / layout.expanded_px;
                        let source_font = if SHRINKING_SOURCE_PX == 44 {
                            Font::Small24
                        } else {
                            Font::Medium28
                        };
                        let label = lv::label_str(
                            root,
                            title,
                            Rect::new((bounds.w - width) / 2, 34, width, SHRINKING_SOURCE_HEIGHT),
                            Color::Accent.resolve(),
                            source_font,
                        );
                        if !label.is_null() {
                            #[cfg(any(feature = "gallery", target_os = "emscripten"))]
                            super::linoval::apply_large_title(label);
                            #[cfg(not(any(feature = "gallery", target_os = "emscripten")))]
                            lv::text_px(
                                label,
                                width,
                                SHRINKING_SOURCE_HEIGHT,
                                2,
                                source_font,
                                SHRINKING_SOURCE_PX,
                            );
                            lvx_object_set_size(label, width, SHRINKING_SOURCE_HEIGHT);
                            lv::style(label, lv::TRANSLATE_X, 0);
                            lv::style(label, lv::TRANSLATE_Y, 0);
                            lv::inspect_text_apx(
                                label,
                                viewport,
                                crate::TextSize::LargeTitle.apx(),
                            );
                            lv::inspect_name(label, c"AerisUI.ShrinkingTitle");
                            lvx_label_set_long_mode(label, 1);
                            lv::style(label, lv::TRANSFORM_PIVOT_X, (width / 2) as u32);
                            lv::style(label, lv::TRANSFORM_PIVOT_Y, 0);
                        }
                        label
                    } else {
                        let line = viewport.apx(Apx::from_halves(11));
                        let area = Rect::new(safe.x, 34, safe.w, line);
                        let label = lv::label_str(
                            root,
                            title,
                            area,
                            Color::Accent.resolve(),
                            Font::Small24,
                        );
                        lv::text_apx_in(
                            label,
                            viewport,
                            area.w,
                            area.h,
                            2,
                            Font::Small24,
                            Apx::from_halves(9),
                        );
                        if !label.is_null() {
                            lv::inspect_name(label, c"AerisUI.HeaderTitle");
                            lvx_label_set_long_mode(label, 1);
                        }
                        label
                    }
                } else {
                    NULL
                }
            }
        };
        let header = Self {
            root,
            height,
            small_title: title_label,
            large_title: None,
            shrinking_title: shrinking,
        };
        header.update_scroll(scroll.map_or(0, |scroll| unsafe { lvx_object_scroll_y(scroll) }));
        Some(header)
    }

    pub fn root(&self) -> Obj {
        self.root
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn collapse_distance(&self) -> i32 {
        self.shrinking_title
            .map_or(LARGE_TITLE_SCROLL_PX, |title| title.scroll_px)
    }

    /// Returns the dedicated large-title area within the scroll container.
    pub fn large_title_area(&self) -> Option<Obj> {
        self.large_title.as_ref().map(|title| title.area)
    }

    /// First available Y position for page content, relative to the scroll container.
    pub fn content_top(&self) -> i32 {
        if self.large_title.is_some() {
            LARGE_TITLE_SCROLL_PX
        } else {
            0
        }
    }

    /// Call with the scroll container's current vertical offset on each scroll event.
    pub fn update_scroll(&self, scroll_y: i32) {
        if let Some(title) = self.shrinking_title {
            let scale = title.scale(scroll_y, SHRINKING_SOURCE_PX);
            lv::style(self.small_title, lv::TRANSFORM_SCALE_X, scale);
            lv::style(self.small_title, lv::TRANSFORM_SCALE_Y, scale);
            return;
        }
        let Some(title) = &self.large_title else {
            return;
        };
        let (large, compact) = title_opacities(scroll_y);
        lv::style(title.label, lv::TEXT_OPA, u32::from(large));
        lv::style(self.small_title, lv::TEXT_OPA, u32::from(compact));
        let offset = (SMALL_TITLE_RISE_PX * u32::from(large) + 127) / 255;
        #[cfg(target_os = "emscripten")]
        let offset = if unsafe { gallery_prefers_reduced_motion() } {
            0
        } else {
            offset
        };
        lv::style(self.small_title, lv::TRANSLATE_Y, offset);
    }
}

impl page::PageBar for HeaderBar {
    fn scroll_inset(&self) -> i32 {
        self.height
    }
}

pub(super) fn height_in(viewport: Viewport) -> i32 {
    if viewport == DisplayProfile::BAND_11.viewport {
        86
    } else {
        HEADER_HEIGHT
    }
}

pub(super) unsafe fn clock_in(parent: Obj, viewport: Viewport) -> Option<Obj> {
    let screen = viewport.bounds();
    if viewport == DisplayProfile::BAND_11.viewport {
        return unsafe { arc_clock(parent, viewport) };
    }
    let wide = viewport == DisplayProfile::REDMI_WATCH.viewport;
    let safe = viewport.content();
    let inset = if wide { 32 } else { 16 };
    let right = safe.right().min(screen.w - inset);
    let y = if wide { 27 } else { safe.y + 8 };
    let (clock, _) = unsafe {
        font::clock(
            parent,
            &page::clock_bytes(),
            [safe.x, y, right - safe.x, 34],
            3,
        )
    };
    (!clock.is_null()).then_some(clock)
}

unsafe fn arc_clock(parent: Obj, viewport: Viewport) -> Option<Obj> {
    let face = unsafe { font::clip(parent, 86, 40, 0) }?;
    let time = page::clock_bytes();
    unsafe {
        lv::inspect_name(face, c"AerisUI.ArcClock");
        // One optical pixel compensates for the clock font's side bearings.
        lvx_object_align(face, 1, (viewport.bounds().w - 86) / 2 + 1, 0);
        for (digit, x, y, angle) in [
            (time[0], 7, 7, -160i32),
            (time[1], 21, 4, -70),
            (time[2], 32, 2, 0),
            (time[3], 43, 4, 70),
            (time[4], 57, 7, 160),
        ] {
            let glyph = lv::label(
                face,
                &[digit, 0],
                x,
                y,
                22,
                30,
                Color::PrimaryText.resolve(),
                Font::Small24,
            );
            if glyph.is_null() {
                lvx_object_delete(face);
                return None;
            }
            super::linoval::apply_clock(glyph);
            lvx_object_set_size(glyph, 22, 30);
            lv::style(glyph, lv::TRANSLATE_X, 0);
            lv::style(glyph, lv::TRANSLATE_Y, 0);
            lv::style(glyph, lv::TRANSFORM_SCALE_X, 256);
            lv::style(glyph, lv::TRANSFORM_SCALE_Y, 256);
            lv::inspect_text_apx(glyph, viewport, crate::TextSize::Body.apx());
            lv::style(glyph, lv::TRANSFORM_PIVOT_X, 11);
            lv::style(glyph, lv::TRANSFORM_PIVOT_Y, 14);
            lv::style(glyph, lv::TRANSFORM_ROTATION, angle as u32);
        }
    }
    Some(face)
}

unsafe fn compact_action(
    parent: Obj,
    area: Rect,
    glyph: &[u8],
    style: lv::RoundButtonStyle,
) -> Obj {
    let accent = style == lv::RoundButtonStyle::Accent;
    let color = if accent {
        Color::Accent
    } else {
        Color::PrimaryText
    };
    let button =
        unsafe { lv::background(parent, area.w, area.h, color.resolve(), (area.w / 2) as u32) };
    if !button.is_null() {
        unsafe {
            lvx_object_align(button, 1, area.x, area.y);
            lvx_object_clear_flag(button, lv::SCROLLABLE);
            lvx_object_add_flag(button, lv::CLICKABLE);
            let inset = (area.w - ICON_SIZE) / 2;
            lv::image_child(button, glyph, inset, inset);
            lv::inspect_name(button, c"AerisUI.HeaderButton");
            anim::attach(button, area.w / 2, area.h / 2, anim::Feedback::Scale);
        }
        lv::style(button, lv::BG_OPA, if accent { 255 } else { 40 });
    }
    button
}

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
    unsafe { lv::inspect_name(object, c"AerisUI.HeaderTitle") };
    set_title_style(object, width, height, layout.title_align);
    object
}

fn set_title_style(object: Obj, width: i32, height: i32, align: TextAlign) {
    let align = text_align(align);
    lv::style(object, 94, align);
    lv::text_apx(
        object,
        width,
        height,
        align,
        Font::Small24,
        crate::TextSize::Body.apx(),
    );
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
            if font::fixed_apx(
                root,
                font::CLOCK_DARK,
                &time,
                area,
                2,
                26,
                crate::TextSize::Body.apx(),
            )
            .is_none()
            {
                let [x, y, w, h] = area;
                let label = lv::label(root, &time, x, y, w, h, 0, Font::Small24);
                lv::text_apx(label, w, h, 2, Font::Small24, crate::TextSize::Body.apx());
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
