//! Header component: 336×94, four text layouts, optional glass action buttons.

use crate::assets::{
    BitmapAsset, BUTTON_ADD, BUTTON_BACK, BUTTON_CLOSE, BUTTON_SETTINGS, HEADER_BACKGROUND,
    HEADER_SHADOW_LEFT, HEADER_SHADOW_RIGHT,
};
use crate::{
    geometry::{Rect, Viewport},
    Apx,
};

pub const HEADER_WIDTH: i32 = 336;
pub const HEADER_HEIGHT: i32 = 94;
pub const PADDING: i32 = 6;
pub const BUTTON_SIZE: i32 = 68;
pub const ICON_SIZE: i32 = 40;
pub const CONTENT_HEIGHT: i32 = 68;

pub const TIME_FONT_PX: i32 = 26;
pub const TIME_LINE_HEIGHT: i32 = 30;
pub const TITLE_FONT_PX: i32 = 26;
pub const TITLE_LINE_HEIGHT: i32 = 32;

/// Figma's footer-background layer: 407×98, centred, starting 34 px above.
pub const BACKDROP_WIDTH: i32 = 407;
pub const BACKDROP_HEIGHT: i32 = 98;
pub const BACKDROP_TOP: i32 = -34;
pub const BACKDROP_BLUR: i32 = 20;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HeaderVariant {
    /// Gear on the left, time/title centered, orange add on the right.
    Center,
    /// Back on the left, time/title right-aligned.
    Right,
    /// Time/title left-aligned, close on the right.
    Left,
    /// 仅显示右对齐的时钟。
    Hidden,
}

impl HeaderVariant {
    /// Center and right variants have a left button; its shadow is baked into
    /// the transparent `leftshadow` bitmap.
    pub const fn uses_left_shadow(self) -> bool {
        matches!(self, HeaderVariant::Center | HeaderVariant::Right)
    }

    /// Center and left variants have a right button; its shadow is baked into
    /// the transparent `rightshadow` bitmap.
    pub const fn uses_right_shadow(self) -> bool {
        matches!(self, HeaderVariant::Center | HeaderVariant::Left)
    }

    /// Left 168×94 half-strip. The shadow variant already contains the
    /// gradient, so it must not be layered over `HEADER_BACKGROUND`.
    pub const fn left_half(self) -> BitmapAsset {
        if self.uses_left_shadow() {
            HEADER_SHADOW_LEFT
        } else {
            HEADER_BACKGROUND
        }
    }

    /// Right 168×94 half-strip.
    pub const fn right_half(self) -> BitmapAsset {
        if self.uses_right_shadow() {
            HEADER_SHADOW_RIGHT
        } else {
            HEADER_BACKGROUND
        }
    }
}

/// Named layouts of the Header & Footer component.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum HeaderStyle {
    #[default]
    Top,
    /// Device navigation: a back header on wide displays, a title and arc clock
    /// on a narrow capsule display, where navigation uses the system gesture.
    Navigation,
    Bottom,
    Left,
    Right,
    Simple,
    CenterSimple,
    HeaderButton,
    FooterButton,
}

impl HeaderStyle {
    #[allow(non_upper_case_globals)]
    #[deprecated(note = "Use HeaderStyle::CenterSimple")]
    pub const ClockOnly: Self = Self::CenterSimple;

    pub const fn supports_large_title(self) -> bool {
        matches!(
            self,
            Self::Top | Self::Navigation | Self::Left | Self::Right
        )
    }
}

pub const LARGE_TITLE_SCROLL_PX: i32 = 91;

/// One title, continuously scaled from 7 apx to 4.5 apx over 12 apx of scroll.
#[derive(Clone, Copy, Debug)]
pub struct ShrinkingTitleLayout {
    pub expanded_px: i32,
    pub compact_px: i32,
    pub scroll_px: i32,
}

impl ShrinkingTitleLayout {
    pub const fn new(viewport: Viewport) -> Self {
        Self {
            expanded_px: viewport.apx(Apx::new(7)),
            compact_px: viewport.apx(Apx::from_halves(9)),
            scroll_px: viewport.apx(Apx::new(12)),
        }
    }

    /// LVGL's 1/256 transform scale; retains fractional font sizes while moving.
    pub fn scale(self, scroll_y: i32, source_px: i32) -> u32 {
        let offset = scroll_y.clamp(0, self.scroll_px);
        let pixels =
            self.expanded_px * self.scroll_px - (self.expanded_px - self.compact_px) * offset;
        ((pixels * 256 + source_px * self.scroll_px / 2) / (source_px * self.scroll_px)) as u32
    }
}

/// Large and compact title opacity at the current vertical scroll offset.
pub fn title_opacities(scroll_y: i32) -> (u8, u8) {
    let compact = (scroll_y.clamp(0, LARGE_TITLE_SCROLL_PX) * 255 / LARGE_TITLE_SCROLL_PX) as u8;
    (255 - compact, compact)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HeaderAction {
    None,
    Settings,
    Add,
    Back,
    Close,
}

impl HeaderAction {
    pub const fn is_present(self) -> bool {
        !matches!(self, HeaderAction::None)
    }

    /// The Figma-exported 68×68 button face, icon included.
    pub const fn asset(self) -> Option<BitmapAsset> {
        match self {
            HeaderAction::None => None,
            HeaderAction::Settings => Some(BUTTON_SETTINGS),
            HeaderAction::Add => Some(BUTTON_ADD),
            HeaderAction::Back => Some(BUTTON_BACK),
            HeaderAction::Close => Some(BUTTON_CLOSE),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Start,
    Center,
    End,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HeaderProps<'a> {
    pub time: &'a str,
    pub title: &'a str,
    pub variant: HeaderVariant,
    pub left: HeaderAction,
    pub right: HeaderAction,
}

impl<'a> HeaderProps<'a> {
    /// The default variant actions from the Figma component set.
    pub const fn new(time: &'a str, title: &'a str, variant: HeaderVariant) -> Self {
        let (left, right) = match variant {
            HeaderVariant::Center => (HeaderAction::Settings, HeaderAction::Add),
            HeaderVariant::Right => (HeaderAction::Back, HeaderAction::None),
            HeaderVariant::Left => (HeaderAction::None, HeaderAction::Close),
            HeaderVariant::Hidden => (HeaderAction::None, HeaderAction::None),
        };
        Self {
            time,
            title,
            variant,
            left,
            right,
        }
    }
}

/// Measured pixel rectangles for one header instance.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HeaderLayout {
    pub frame: Rect,
    pub backdrop: Rect,
    pub left_button: Option<Rect>,
    pub right_button: Option<Rect>,
    pub content: Rect,
    pub time: Rect,
    pub title: Option<Rect>,
    pub left_icon: Option<Rect>,
    pub right_icon: Option<Rect>,
    pub time_align: TextAlign,
    pub title_align: TextAlign,
}

impl HeaderLayout {
    pub fn measure(props: HeaderProps<'_>) -> Self {
        Self::measure_in(props, HEADER_WIDTH).expect("the default header width fits")
    }

    /// Measures a header in local coordinates at the assigned width.
    pub fn measure_in(props: HeaderProps<'_>, width: i32) -> Option<Self> {
        const MIN_TEXT_WIDTH: i32 = 80;
        const MIN_BUTTON_SIZE: i32 = 44;
        if width < 104 {
            return None;
        }
        let button_count = i32::from(props.left.is_present()) + i32::from(props.right.is_present());
        let button_size = if button_count == 0 {
            BUTTON_SIZE
        } else {
            ((width - 2 * PADDING - MIN_TEXT_WIDTH) / button_count).min(BUTTON_SIZE)
        };
        if button_size < MIN_BUTTON_SIZE {
            return None;
        }
        let frame = Rect::new(0, 0, width, HEADER_HEIGHT);
        let backdrop = Rect::new(
            (width - BACKDROP_WIDTH) / 2,
            BACKDROP_TOP,
            BACKDROP_WIDTH,
            BACKDROP_HEIGHT,
        );

        let left_button = props.left.is_present().then_some(Rect::new(
            PADDING,
            PADDING,
            button_size,
            button_size,
        ));
        let right_button = props.right.is_present().then_some(Rect::new(
            width - PADDING - button_size,
            PADDING,
            button_size,
            button_size,
        ));

        let (content, time_align, title_align, show_title) = match props.variant {
            HeaderVariant::Center => (
                Rect::new(
                    PADDING + button_size,
                    PADDING,
                    width - 2 * (PADDING + button_size),
                    CONTENT_HEIGHT,
                ),
                TextAlign::Center,
                TextAlign::Center,
                true,
            ),
            HeaderVariant::Right => (
                Rect::new(
                    PADDING + button_size,
                    PADDING,
                    width - 2 * PADDING - button_size,
                    CONTENT_HEIGHT,
                ),
                TextAlign::End,
                TextAlign::End,
                true,
            ),
            HeaderVariant::Left => (
                Rect::new(
                    PADDING + 10,
                    PADDING,
                    width - 2 * PADDING - button_size - 10,
                    CONTENT_HEIGHT,
                ),
                TextAlign::Start,
                TextAlign::Start,
                true,
            ),
            HeaderVariant::Hidden => (
                Rect::new(
                    PADDING + 10,
                    PADDING,
                    width - 2 * PADDING - 10,
                    CONTENT_HEIGHT,
                ),
                TextAlign::End,
                TextAlign::End,
                false,
            ),
        };
        if content.w <= 0 {
            return None;
        }

        // 所有布局共用时钟的纵向位置。
        let time_y = content.y + (content.h - TIME_LINE_HEIGHT - TITLE_LINE_HEIGHT) / 2;
        let time = Rect::new(content.x, time_y, content.w, TIME_LINE_HEIGHT);
        let title = show_title.then_some(Rect::new(
            content.x,
            time_y + TIME_LINE_HEIGHT,
            content.w,
            TITLE_LINE_HEIGHT,
        ));

        let (left_icon, right_icon) = (
            left_button.map(HeaderLayout::icon_box),
            right_button.map(HeaderLayout::icon_box),
        );

        Some(Self {
            frame,
            backdrop,
            left_button,
            right_button,
            content,
            time,
            title,
            left_icon,
            right_icon,
            time_align,
            title_align,
        })
    }

    pub const fn icon_box(button: Rect) -> Rect {
        Rect::new(
            button.x + (button.w - ICON_SIZE) / 2,
            button.y + (button.h - ICON_SIZE) / 2,
            ICON_SIZE,
            ICON_SIZE,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capsule_title_scales_continuously_and_clamps_at_both_ends() {
        let layout = ShrinkingTitleLayout::new(crate::geometry::DisplayProfile::BAND_11.viewport);
        assert_eq!(
            (layout.expanded_px, layout.compact_px, layout.scroll_px),
            (39, 25, 66)
        );
        assert_eq!(layout.scale(-10, 44), layout.scale(0, 44));
        assert_eq!(layout.scale(200, 44), layout.scale(66, 44));
        assert!((0..66).all(|y| layout.scale(y, 44) > layout.scale(y + 1, 44)));
    }

    #[test]
    fn large_title_follows_scroll_in_both_directions() {
        for offset in [-20, 0] {
            assert_eq!(title_opacities(offset), (255, 0));
        }
        let middle = title_opacities(45);
        assert!(middle.0 > 0 && middle.1 > 0);
        assert_eq!(u16::from(middle.0) + u16::from(middle.1), 255);
        for offset in [LARGE_TITLE_SCROLL_PX, 200] {
            assert_eq!(title_opacities(offset), (0, 255));
        }
        assert_eq!(title_opacities(45), middle);
        assert_eq!(title_opacities(0), (255, 0));
        assert!(HeaderStyle::Top.supports_large_title());
        assert!(HeaderStyle::Left.supports_large_title());
        assert!(HeaderStyle::Right.supports_large_title());
        assert!(!HeaderStyle::Bottom.supports_large_title());
        assert!(!HeaderStyle::Simple.supports_large_title());
        assert!(!HeaderStyle::CenterSimple.supports_large_title());
    }

    const ALL: [HeaderVariant; 4] = [
        HeaderVariant::Center,
        HeaderVariant::Right,
        HeaderVariant::Left,
        HeaderVariant::Hidden,
    ];

    #[test]
    fn every_variant_fits_the_336x94_frame() {
        for variant in ALL {
            let layout = HeaderLayout::measure(HeaderProps::new("09:28", "我的计时器", variant));
            assert_eq!(layout.frame, Rect::new(0, 0, 336, 94));
            assert!(layout.frame.contains(layout.content));
            if let Some(button) = layout.left_button {
                assert!(layout.frame.contains(button));
                assert_eq!(button.w, 68);
            }
            if let Some(button) = layout.right_button {
                assert!(layout.frame.contains(button));
                assert_eq!(button.w, 68);
            }
            if let Some(title) = layout.title {
                assert!(layout.frame.contains(title));
            }
        }
    }

    #[test]
    fn center_layout_puts_buttons_at_the_edges_and_text_in_the_middle() {
        let layout = HeaderLayout::measure(HeaderProps::new(
            "09:28",
            "我的计时器",
            HeaderVariant::Center,
        ));
        assert_eq!(layout.left_button, Some(Rect::new(6, 6, 68, 68)));
        assert_eq!(layout.right_button, Some(Rect::new(262, 6, 68, 68)));
        assert_eq!(layout.content, Rect::new(74, 6, 188, 68));
        assert_eq!(layout.content.center_x(), 168);
        assert_eq!(layout.time_align, TextAlign::Center);
        assert_eq!(layout.title_align, TextAlign::Center);
    }

    #[test]
    fn right_layout_has_only_a_back_button_and_right_aligned_text() {
        let layout = HeaderLayout::measure(HeaderProps::new(
            "09:28",
            "我的计时器",
            HeaderVariant::Right,
        ));
        assert_eq!(layout.left_button, Some(Rect::new(6, 6, 68, 68)));
        assert_eq!(layout.right_button, None);
        assert_eq!(layout.time_align, TextAlign::End);
        assert_eq!(layout.title_align, TextAlign::End);
        assert_eq!(layout.content.right(), 330);
    }

    #[test]
    fn left_layout_has_only_a_close_button_and_left_aligned_text() {
        let layout =
            HeaderLayout::measure(HeaderProps::new("09:28", "我的计时器", HeaderVariant::Left));
        assert_eq!(layout.left_button, None);
        assert_eq!(layout.right_button, Some(Rect::new(262, 6, 68, 68)));
        assert_eq!(layout.time_align, TextAlign::Start);
        assert_eq!(layout.content.x, 16);
    }

    #[test]
    fn all_variants_share_the_same_clock_position() {
        for variant in ALL {
            let layout = HeaderLayout::measure(HeaderProps::new("09:28", "我的计时器", variant));
            assert_eq!((layout.time.y, layout.time.h), (9, TIME_LINE_HEIGHT));
            if let Some(title) = layout.title {
                assert_eq!((title.y, title.h), (39, TITLE_LINE_HEIGHT));
            }
        }
    }

    #[test]
    fn hidden_layout_has_only_the_time() {
        let layout = HeaderLayout::measure(HeaderProps::new(
            "09:28",
            "我的计时器",
            HeaderVariant::Hidden,
        ));
        assert_eq!(layout.left_button, None);
        assert_eq!(layout.right_button, None);
        assert_eq!(layout.title, None);
        assert_eq!(layout.time_align, TextAlign::End);
    }

    #[test]
    fn icon_boxes_are_centred_in_their_buttons() {
        let button = Rect::new(6, 6, 68, 68);
        assert_eq!(HeaderLayout::icon_box(button), Rect::new(20, 20, 40, 40));
    }

    #[test]
    fn compact_center_layout_keeps_both_actions_and_text_inside_the_bar() {
        let layout = HeaderLayout::measure_in(
            HeaderProps::new("09:28", "Aeris", HeaderVariant::Center),
            188,
        )
        .unwrap();
        let left = layout.left_button.unwrap();
        let right = layout.right_button.unwrap();
        assert!(left.w >= 44);
        assert!(layout.content.w >= 80);
        assert!(layout.frame.contains(left) && layout.frame.contains(right));
        assert!(layout.frame.contains(layout.content));
        assert!(!layout.content.overlaps(left) && !layout.content.overlaps(right));
        assert!(left.contains(layout.left_icon.unwrap()));
        assert!(right.contains(layout.right_icon.unwrap()));
    }
}
