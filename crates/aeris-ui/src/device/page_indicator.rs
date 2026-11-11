//! 原生横滑分页指示器。

use crate::{
    geometry::Rect,
    motion::dot_size,
    page_indicator::{dots_left, INDICATOR_GAP, INDICATOR_HEIGHT},
};
use canopus_target_private::*;

use super::{
    assets,
    lv::{self, Obj},
    page,
};

const FULL: i32 = 1000;

pub struct Indicator<const N: usize> {
    dots: [Obj; N],
    bounds: Rect,
    position: i32,
}

pub fn position(pager: Obj, count: usize) -> i32 {
    position_in(pager, count, page::WIDTH)
}

pub fn position_in(pager: Obj, count: usize, page_width: i32) -> i32 {
    if pager.is_null() || page_width <= 0 {
        return 0;
    }
    let max = count.saturating_sub(1) as i32 * page_width;
    unsafe { lvx_object_scroll_x(pager) }.clamp(0, max) * FULL / page_width
}

pub unsafe fn footer_background(root: Obj) {
    let background = unsafe {
        lv::image(
            root,
            assets::FOOTER_BACKGROUND,
            0,
            page::HEIGHT - INDICATOR_HEIGHT,
        )
    };
    if !background.is_null() {
        unsafe { lvx_object_clear_flag(background, lv::CLICKABLE | lv::SCROLLABLE) };
    }
}

impl<const N: usize> Indicator<N> {
    pub unsafe fn new(root: Obj, center_y: i32, select: LvxEventCallback) -> Self {
        unsafe { Self::new_in(root, Rect::new(0, center_y, page::WIDTH, 0), select) }
    }

    pub unsafe fn new_in(root: Obj, bounds: Rect, select: LvxEventCallback) -> Self {
        Self {
            dots: core::array::from_fn(|index| {
                let dot = unsafe { lv::dot(root, 0, 0, 10, 102) };
                lv::bind(dot, select, index);
                dot
            }),
            bounds,
            position: -1,
        }
    }

    pub fn paint(&mut self, position: i32) {
        if self.position == position {
            return;
        }
        self.position = position;
        let first = (position / FULL) as usize;
        let progress = position % FULL;
        let sizes = core::array::from_fn::<_, N, _>(|index| {
            if index == first {
                dot_size(false, progress)
            } else if index == first + 1 {
                dot_size(true, progress)
            } else {
                dot_size(true, 0)
            }
        });
        let mut x = dots_left(self.bounds, &sizes);
        for (index, (dot, size)) in self.dots.into_iter().zip(sizes).enumerate() {
            let amount = if index == first {
                FULL - progress
            } else if index == first + 1 {
                progress
            } else {
                0
            };
            if !dot.is_null() {
                unsafe {
                    lvx_object_set_size(dot, size, size);
                    lvx_object_align(dot, 1, x, self.bounds.center_y() - size / 2);
                }
                lv::style(dot, 12, (size / 2) as u32);
                lv::style(dot, lv::BG_OPA, 102 + (148 * amount / FULL) as u32);
            }
            x += size + INDICATOR_GAP;
        }
    }
}
