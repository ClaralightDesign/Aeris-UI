//! Three-piece capsule button.
//!
//! Each Figma slice is 80×70 or 80×82 logical pixels. The left and right caps
//! stay fixed; only the middle slice stretches, so the capsule can grow from
//! 160 px up to full width while keeping its round ends.

use crate::geometry::Rect;

pub const CAPSULE_WIDTH: i32 = 240;
pub const CAPSULE_SLICE_WIDTH: i32 = 80;
pub const CAPSULE_HEIGHT: i32 = 70;
pub const CAPSULE_LARGE_HEIGHT: i32 = 82;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CapsuleKind {
    Regular,
    Large,
    LargePrimary,
}

impl CapsuleKind {
    pub const fn height(self) -> i32 {
        match self {
            CapsuleKind::Regular => CAPSULE_HEIGHT,
            CapsuleKind::Large | CapsuleKind::LargePrimary => CAPSULE_LARGE_HEIGHT,
        }
    }

    pub const fn radius(self) -> i32 {
        self.height() / 2
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CapsuleLayout {
    pub frame: Rect,
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
}

impl CapsuleLayout {
    pub fn measure(width: i32, height: i32) -> Self {
        let width = width.max(2);
        let height = height.max(2);
        let slice = CAPSULE_SLICE_WIDTH.min(width / 2);
        Self {
            frame: Rect::new(0, 0, width, height),
            left: Rect::new(0, 0, slice, height),
            center: Rect::new(slice, 0, width - 2 * slice, height),
            right: Rect::new(width - slice, 0, slice, height),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_capsule_is_three_80px_slices() {
        let layout = CapsuleLayout::measure(CAPSULE_WIDTH, CAPSULE_HEIGHT);
        assert_eq!(layout.left, Rect::new(0, 0, 80, 70));
        assert_eq!(layout.center, Rect::new(80, 0, 80, 70));
        assert_eq!(layout.right, Rect::new(160, 0, 80, 70));
        assert_eq!(CapsuleKind::Regular.radius(), 35);
    }

    #[test]
    fn large_capsule_uses_its_own_height_and_radius() {
        let layout = CapsuleLayout::measure(CAPSULE_WIDTH, CAPSULE_LARGE_HEIGHT);
        assert_eq!(layout.left, Rect::new(0, 0, 80, 82));
        assert_eq!(layout.center, Rect::new(80, 0, 80, 82));
        assert_eq!(CapsuleKind::Large.radius(), 41);
        assert_eq!(CapsuleKind::LargePrimary.radius(), 41);
    }

    #[test]
    fn only_the_center_stretches() {
        let layout = CapsuleLayout::measure(320, CAPSULE_HEIGHT);
        assert_eq!(layout.left.w, 80);
        assert_eq!(layout.right.w, 80);
        assert_eq!(layout.center.w, 160);
        assert_eq!(layout.center.x, 80);
        assert_eq!(layout.right.x, 240);
    }

    #[test]
    fn capsule_slices_fit_narrow_and_wide_safe_areas() {
        for width in [240, 336, 456] {
            let area = Rect::new(12, 88, width, CAPSULE_LARGE_HEIGHT)
                .horizontal_inset(3)
                .unwrap();
            let layout = CapsuleLayout::measure(area.w, area.h);
            assert_eq!(layout.frame.w, width - 6);
            assert_eq!(layout.left.w, CAPSULE_SLICE_WIDTH);
            assert_eq!(layout.right.w, CAPSULE_SLICE_WIDTH);
            assert_eq!(layout.right.right(), area.w);
        }
    }

    #[test]
    fn tiny_capsule_clamps_the_caps() {
        let layout = CapsuleLayout::measure(100, CAPSULE_HEIGHT);
        assert_eq!(layout.left.w, 50);
        assert_eq!(layout.right.w, 50);
        assert_eq!(layout.center.w, 0);
    }
}
