//! Header component: 336×94, four text layouts, optional glass action buttons.

use crate::assets::{BitmapAsset, BUTTON_ADD, BUTTON_BACK, BUTTON_CLOSE, BUTTON_SETTINGS, HEADER_BACKGROUND, HEADER_SHADOW_LEFT, HEADER_SHADOW_RIGHT};
use crate::geometry::Rect;

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
        let frame = Rect::new(0, 0, HEADER_WIDTH, HEADER_HEIGHT);
        let backdrop = Rect::new(
            (HEADER_WIDTH - BACKDROP_WIDTH) / 2,
            BACKDROP_TOP,
            BACKDROP_WIDTH,
            BACKDROP_HEIGHT,
        );

        let left_button = props
            .left
            .is_present()
            .then_some(Rect::new(PADDING, PADDING, BUTTON_SIZE, BUTTON_SIZE));
        let right_button = props.right.is_present().then_some(Rect::new(
            HEADER_WIDTH - PADDING - BUTTON_SIZE,
            PADDING,
            BUTTON_SIZE,
            BUTTON_SIZE,
        ));

        let (content, time_align, title_align, show_title) = match props.variant {
            HeaderVariant::Center => (
                Rect::new(
                    PADDING + BUTTON_SIZE,
                    PADDING,
                    HEADER_WIDTH - 2 * (PADDING + BUTTON_SIZE),
                    CONTENT_HEIGHT,
                ),
                TextAlign::Center,
                TextAlign::Center,
                true,
            ),
            HeaderVariant::Right => (
                Rect::new(
                    PADDING + BUTTON_SIZE,
                    PADDING,
                    HEADER_WIDTH - 2 * PADDING - BUTTON_SIZE,
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
                    HEADER_WIDTH - 2 * PADDING - BUTTON_SIZE - 10,
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
                    HEADER_WIDTH - 2 * PADDING - 10,
                    CONTENT_HEIGHT,
                ),
                TextAlign::End,
                TextAlign::End,
                false,
            ),
        };

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

        Self {
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
        }
    }

    pub const fn icon_box(button: Rect) -> Rect {
        Rect::new(
            button.x + (BUTTON_SIZE - ICON_SIZE) / 2,
            button.y + (BUTTON_SIZE - ICON_SIZE) / 2,
            ICON_SIZE,
            ICON_SIZE,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let layout = HeaderLayout::measure(HeaderProps::new(
            "09:28",
            "我的计时器",
            HeaderVariant::Left,
        ));
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
}
