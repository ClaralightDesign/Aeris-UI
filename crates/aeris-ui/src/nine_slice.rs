//! Five-piece nine-slice card.
//!
//! The Figma export provides four 38×38 corners and one 38×38 uniform center.
//! Edges and the middle are tiled from `center.png`; this is the usual
//! nine-slice behaviour without requiring separate edge assets.

use crate::geometry::Rect;

pub const CARD_WIDTH: i32 = 408;
pub const CARD_HEIGHT: i32 = 98;
pub const CORNER: i32 = 38;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NineSliceLayout {
    pub width: i32,
    pub height: i32,
    pub top_left: Rect,
    pub top: Rect,
    pub top_right: Rect,
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
    pub bottom_left: Rect,
    pub bottom: Rect,
    pub bottom_right: Rect,
}

impl NineSliceLayout {
    pub fn measure(width: i32, height: i32) -> Self {
        let width = width.max(2);
        let height = height.max(2);
        let corner = CORNER.min(width / 2).min(height / 2);
        let middle_w = width - 2 * corner;
        let middle_h = height - 2 * corner;

        Self {
            width,
            height,
            top_left: Rect::new(0, 0, corner, corner),
            top: Rect::new(corner, 0, middle_w, corner),
            top_right: Rect::new(width - corner, 0, corner, corner),
            left: Rect::new(0, corner, corner, middle_h),
            center: Rect::new(corner, corner, middle_w, middle_h),
            right: Rect::new(width - corner, corner, corner, middle_h),
            bottom_left: Rect::new(0, height - corner, corner, corner),
            bottom: Rect::new(corner, height - corner, middle_w, corner),
            bottom_right: Rect::new(width - corner, height - corner, corner, corner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_card_is_408_by_98_with_38_pixel_corners() {
        let layout = NineSliceLayout::measure(CARD_WIDTH, CARD_HEIGHT);
        assert_eq!(layout.top_left, Rect::new(0, 0, 38, 38));
        assert_eq!(layout.top_right, Rect::new(370, 0, 38, 38));
        assert_eq!(layout.bottom_left, Rect::new(0, 60, 38, 38));
        assert_eq!(layout.bottom_right, Rect::new(370, 60, 38, 38));
        assert_eq!(layout.top, Rect::new(38, 0, 332, 38));
        assert_eq!(layout.bottom, Rect::new(38, 60, 332, 38));
        assert_eq!(layout.left, Rect::new(0, 38, 38, 22));
        assert_eq!(layout.right, Rect::new(370, 38, 38, 22));
        assert_eq!(layout.center, Rect::new(38, 38, 332, 22));
    }

    #[test]
    fn every_slice_fits_the_card() {
        let layout = NineSliceLayout::measure(336, 98);
        let card = Rect::new(0, 0, 336, 98);
        for part in [
            layout.top_left,
            layout.top,
            layout.top_right,
            layout.left,
            layout.center,
            layout.right,
            layout.bottom_left,
            layout.bottom,
            layout.bottom_right,
        ] {
            assert!(card.contains(part));
        }
    }

    #[test]
    fn corners_shrink_for_tiny_cards() {
        let layout = NineSliceLayout::measure(40, 30);
        assert_eq!(layout.top_left.w, 15);
        assert_eq!(layout.top_left.h, 15);
        assert_eq!(layout.center.w, 10);
        assert_eq!(layout.center.h, 0);
    }
}
