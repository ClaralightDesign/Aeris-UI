//! 分页指示点的尺寸与居中布局。

use crate::geometry::Rect;

pub const INDICATOR_WIDTH: i32 = 336;
pub const INDICATOR_HEIGHT: i32 = 94;
pub const INDICATOR_INACTIVE_SIZE: i32 = 10;
pub const INDICATOR_ACTIVE_SIZE: i32 = 14;
pub const INDICATOR_GAP: i32 = 10;
pub const MAX_DOTS: usize = 8;

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
