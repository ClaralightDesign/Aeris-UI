//! 原生横滑分页指示器。

use crate::{
    geometry::{Rect, Viewport},
    motion::dot_size,
    page_indicator::{capsule_dot, dots_left, FooterLayout, INDICATOR_GAP, INDICATOR_HEIGHT},
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
    curved: bool,
    content_inset: i32,
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
    unsafe { footer_background_in(root, super::display_profile().viewport) };
}

/// Reuses the existing transparent footer gradient at the screen's bottom edge.
pub unsafe fn footer_background_in(root: Obj, viewport: Viewport) {
    let bounds = FooterLayout::new(viewport).background;
    let background = unsafe { lv::image(root, assets::FOOTER_BACKGROUND, 0, bounds.y) };
    if !background.is_null() {
        unsafe {
            lvx_object_clear_flag(background, lv::CLICKABLE | lv::SCROLLABLE);
            lv::inspect_name(background, c"AerisUI.PageIndicatorBackground");
            lv::style(background, lv::TRANSFORM_PIVOT_X, 0);
            lv::style(background, lv::TRANSFORM_PIVOT_Y, 0);
            lvx_image_set_scale(
                background,
                (bounds.w * 256 + page::WIDTH - 1) / page::WIDTH,
                (bounds.h * 256 + INDICATOR_HEIGHT - 1) / INDICATOR_HEIGHT,
            );
        }
    }
}

impl<const N: usize> Indicator<N> {
    /// Anchors the Band 11 arc to the full screen, independent of its safe area.
    pub unsafe fn new_for_viewport(
        root: Obj,
        viewport: Viewport,
        select: LvxEventCallback,
    ) -> Self {
        let layout = FooterLayout::new(viewport);
        unsafe { footer_background_in(root, viewport) };
        let mut indicator = unsafe { Self::new_in(root, layout.dots, select) };
        indicator.curved = layout.curved;
        indicator.content_inset = layout.content_inset;
        indicator
    }

    pub unsafe fn new(root: Obj, center_y: i32, select: LvxEventCallback) -> Self {
        unsafe { Self::new_in(root, Rect::new(0, center_y, page::WIDTH, 0), select) }
    }

    pub unsafe fn new_in(root: Obj, bounds: Rect, select: LvxEventCallback) -> Self {
        Self {
            dots: core::array::from_fn(|index| {
                let dot = unsafe { lv::dot(root, 0, 0, 10, 102) };
                if !dot.is_null() {
                    unsafe {
                        lv::inspect_name(dot, c"AerisUI.PageIndicatorDot");
                        lvx_object_clear_flag(dot, lv::SCROLLABLE);
                    }
                }
                lv::bind(dot, select, index);
                dot
            }),
            bounds,
            position: -1,
            curved: false,
            content_inset: bounds.h.max(14),
        }
    }

    pub fn paint(&mut self, position: i32) {
        let position = position.clamp(0, N.saturating_sub(1) as i32 * FULL);
        if self.position == position {
            return;
        }
        self.position = position;
        if self.curved {
            for (index, dot) in self.dots.into_iter().enumerate() {
                if dot.is_null() {
                    continue;
                }
                let geometry = capsule_dot(self.bounds, index, position);
                unsafe { lvx_set_hidden(dot, u32::from(geometry.is_none())) };
                if let Some((area, color, opacity)) = geometry {
                    unsafe {
                        lvx_object_set_size(dot, area.w, area.h);
                        lvx_object_align(dot, 1, area.x, area.y);
                    }
                    lv::style(dot, lv::RADIUS, 0x7fff);
                    lv::style(dot, lv::BG_COLOR, color);
                    lv::style(dot, lv::BG_OPA, u32::from(opacity));
                }
            }
            return;
        }
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
                lv::style(dot, lv::RADIUS, 0x7fff);
                lv::style(dot, lv::BG_OPA, 102 + (148 * amount / FULL) as u32);
            }
            x += size + INDICATOR_GAP;
        }
    }
}

impl<const N: usize> page::PageBar for Indicator<N> {
    fn scroll_inset(&self) -> i32 {
        self.content_inset
    }
}
