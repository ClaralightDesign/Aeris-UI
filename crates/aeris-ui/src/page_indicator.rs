//! 分页指示点的尺寸与居中布局。

use crate::{
    geometry::{DisplayProfile, Rect, Viewport},
    Apx,
};

pub const INDICATOR_WIDTH: i32 = 336;
pub const INDICATOR_HEIGHT: i32 = 94;
pub const INDICATOR_INACTIVE_SIZE: i32 = 10;
pub const INDICATOR_ACTIVE_SIZE: i32 = 14;
pub const INDICATOR_GAP: i32 = 10;
pub const MAX_DOTS: usize = 8;

#[derive(Clone, Copy, Debug)]
pub struct FooterLayout {
    pub background: Rect,
    pub dots: Rect,
    pub curved: bool,
    pub content_inset: i32,
}

impl FooterLayout {
    pub fn new(viewport: Viewport) -> Self {
        let screen = viewport.bounds();
        let safe = viewport.content();
        let curved = viewport == DisplayProfile::BAND_11.viewport;
        let height = if curved { 100 } else { INDICATOR_HEIGHT };
        let offset = viewport.apx(Apx::new(if curved { 4 } else { 2 }));
        let background = Rect::new(0, screen.h - height + offset, screen.w, height);
        let dots = if curved {
            Rect::new(0, screen.h - 100, screen.w, 100)
        } else {
            Rect::new(safe.x, safe.bottom() - viewport.apx(Apx::new(3)), safe.w, 0)
        };
        Self {
            background,
            dots,
            curved,
            content_inset: screen.h - background.y,
        }
    }
}

/// Curved, moving five-dot window from the Band 11 footer (212 × 100).
/// `position` is the continuous page offset, in thousandths of a page.
/// Coordinates stay in half pixels until the final dot bounds are resolved.
pub fn capsule_dot(frame: Rect, index: usize, position: i32) -> Option<(Rect, u32, u8)> {
    let distance = index as i64 * 1000 - i64::from(position);
    let amount = distance.abs();
    if amount > 2000 {
        return None;
    }
    let segment = (amount / 1000).min(1) as usize;
    let progress = (amount - segment as i64 * 1000) as i32;
    let interpolate = |values: [i32; 3]| {
        values[segment] + (values[segment + 1] - values[segment]) * progress / 1000
    };
    let offset = interpolate([0, 40, 75]) * distance.signum() as i32;
    let y = interpolate([179, 173, 166]);
    let size = interpolate([11_000, 9_000, 6_000]);
    let diameter = (size + 500) / 1000;
    let half_pixel = |value: i32| (value + 1).div_euclid(2);
    let dot = Rect::new(
        frame.center_x() + half_pixel(offset - diameter),
        frame.y + half_pixel(y - diameter),
        diameter,
        diameter,
    );
    let opacity = interpolate([255, 102, 102]) as u8;
    Some((dot, 0xFFFFFF, opacity))
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PageIndicatorState {
    pub active: usize,
    pub count: usize,
}

impl PageIndicatorState {
    pub const fn new(active: usize, count: usize) -> Self {
        Self { active, count }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PageIndicatorLayout {
    pub frame: Rect,
    pub dots: [Rect; MAX_DOTS],
    pub count: usize,
}

impl PageIndicatorLayout {
    pub fn measure(state: PageIndicatorState) -> Self {
        Self::measure_in(Rect::new(0, 0, INDICATOR_WIDTH, INDICATOR_HEIGHT), state)
    }

    pub fn measure_in(frame: Rect, state: PageIndicatorState) -> Self {
        let count = state.count.min(MAX_DOTS);
        let active = if count == 0 {
            0
        } else {
            state.active.min(count - 1)
        };

        let mut sizes = [0i32; MAX_DOTS];
        for (index, size) in sizes.iter_mut().enumerate().take(count) {
            *size = if index == active {
                INDICATOR_ACTIVE_SIZE
            } else {
                INDICATOR_INACTIVE_SIZE
            };
        }

        let mut dots = [Rect::new(0, 0, 0, 0); MAX_DOTS];
        let mut x = dots_left(frame, &sizes[..count]);
        for (index, size) in sizes.iter().enumerate().take(count) {
            let size = *size;
            dots[index] = Rect::new(x, frame.center_y() - size / 2, size, size);
            x += size + INDICATOR_GAP;
        }

        Self { frame, dots, count }
    }

    pub fn dot(&self, index: usize) -> Option<Rect> {
        (index < self.count).then_some(self.dots[index])
    }
}

pub(crate) fn dots_left(frame: Rect, sizes: &[i32]) -> i32 {
    let gaps = sizes.len().saturating_sub(1) as i32 * INDICATOR_GAP;
    frame.x + (frame.w - sizes.iter().sum::<i32>() - gaps) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_content_insets_follow_the_visible_gradient_bounds() {
        let pro = FooterLayout::new(DisplayProfile::BAND_10_PRO.viewport);
        assert_eq!(pro.background, Rect::new(0, 399, 336, 94));
        assert_eq!(pro.dots.center_y(), 461);
        assert_eq!(pro.content_inset, 81);
        let band = FooterLayout::new(DisplayProfile::BAND_11.viewport);
        assert_eq!(band.background, Rect::new(0, 442, 212, 100));
        assert_eq!(band.content_inset, 78);
    }

    #[test]
    fn capsule_footer_matches_the_five_svg_circles() {
        let frame = Rect::new(0, 420, 212, 100);
        let expected = [
            (Rect::new(66, 500, 6, 6), 0xFFFFFF, 102),
            (Rect::new(82, 502, 9, 9), 0xFFFFFF, 102),
            (Rect::new(101, 504, 11, 11), 0xFFFFFF, 255),
            (Rect::new(122, 502, 9, 9), 0xFFFFFF, 102),
            (Rect::new(141, 500, 6, 6), 0xFFFFFF, 102),
        ];
        for (index, expected) in expected.into_iter().enumerate() {
            assert_eq!(capsule_dot(frame, index, 2000), Some(expected));
        }
        assert!(capsule_dot(frame, 5, 2000).is_none());
    }

    #[test]
    fn capsule_window_moves_with_fractional_page_offsets() {
        let frame = Rect::new(0, 420, 212, 100);
        let (left, _, left_opacity) = capsule_dot(frame, 1, 1500).unwrap();
        let (right, _, right_opacity) = capsule_dot(frame, 2, 1500).unwrap();
        assert_eq!(left.w, right.w);
        assert_eq!(left.y, right.y);
        assert_eq!(left_opacity, right_opacity);
        assert!(left.center_x() < frame.center_x());
        assert!(right.center_x() > frame.center_x());
        assert!(frame.contains(left) && frame.contains(right));
    }

    #[test]
    fn two_dots_are_centred_with_a_ten_pixel_gap() {
        for active in 0..2 {
            let layout = PageIndicatorLayout::measure(PageIndicatorState::new(active, 2));
            let first = layout.dot(0).unwrap();
            let second = layout.dot(1).unwrap();
            let total = first.w + second.w + INDICATOR_GAP;
            assert_eq!(first.x, (INDICATOR_WIDTH - total) / 2);
            assert_eq!(second.x, first.right() + INDICATOR_GAP);
            assert_eq!(first.center_y(), INDICATOR_HEIGHT / 2);
            assert_eq!(second.center_y(), INDICATOR_HEIGHT / 2);
            assert_eq!(layout.frame.contains(first), true);
            assert_eq!(layout.frame.contains(second), true);
        }
    }

    #[test]
    fn active_dot_is_fourteen_and_inactive_is_ten() {
        let first = PageIndicatorLayout::measure(PageIndicatorState::new(0, 2));
        assert_eq!(first.dot(0).unwrap().w, 14);
        assert_eq!(first.dot(1).unwrap().w, 10);

        let second = PageIndicatorLayout::measure(PageIndicatorState::new(1, 2));
        assert_eq!(second.dot(0).unwrap().w, 10);
        assert_eq!(second.dot(1).unwrap().w, 14);
    }

    #[test]
    fn active_is_clamped_to_the_available_pages() {
        let layout = PageIndicatorLayout::measure(PageIndicatorState::new(9, 2));
        assert_eq!(layout.dot(1).unwrap().w, 14);
        assert_eq!(layout.count, 2);
    }

    #[test]
    fn empty_state_has_no_dots() {
        let layout = PageIndicatorLayout::measure(PageIndicatorState::new(0, 0));
        assert_eq!(layout.count, 0);
        assert_eq!(layout.dot(0), None);
    }

    #[test]
    fn dots_remain_inside_narrow_and_wide_safe_areas() {
        for frame in [Rect::new(48, 400, 240, 36), Rect::new(16, 430, 448, 40)] {
            let layout = PageIndicatorLayout::measure_in(frame, PageIndicatorState::new(1, 3));
            let first = layout.dot(0).unwrap();
            let last = layout.dot(2).unwrap();
            assert_eq!(first.x - frame.x, frame.right() - last.right());
            assert!(layout.dots[..layout.count]
                .iter()
                .all(|&dot| frame.contains(dot)));
        }
    }
}
