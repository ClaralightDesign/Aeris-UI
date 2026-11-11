//! 确认滑块的可用区域与拖动状态。

use crate::geometry::Rect;

const THUMB_WIDTH: i32 = 104;
const END_SLOP: i32 = 5;

pub(crate) struct SliderLayout {
    pub track: Rect,
    pub caption: Rect,
    pub thumb: Rect,
    pub travel: i32,
}

impl SliderLayout {
    pub fn measure(area: Rect) -> Option<Self> {
        if area.h < 88 || area.w < 160 {
            return None;
        }
        let track = Rect::new(area.x + 4, area.bottom() - 88, area.w - 8, 88);
        Some(Self {
            track,
            caption: Rect::new(area.right() - 133, track.y + 24, 106, 40),
            thumb: Rect::new(area.x + 10, track.y + 6, THUMB_WIDTH, 76),
            travel: area.w - THUMB_WIDTH - 20,
        })
    }
}

#[derive(Default)]
pub(crate) struct Drag {
    start: Option<i32>,
    pub distance: i32,
}

impl Drag {
    pub fn press(&mut self, x: Option<i32>) {
        self.start = x;
        self.distance = 0;
    }

    pub fn move_to(&mut self, x: Option<i32>, travel: i32) {
        if let (Some(x), Some(start)) = (x, self.start) {
            self.distance = x.saturating_sub(start).clamp(0, travel);
        }
    }

    pub fn release(&mut self, x: Option<i32>, travel: i32) -> bool {
        if self.start.is_none() {
            return false;
        }
        self.move_to(x, travel);
        self.start = None;
        let confirmed = self.distance >= travel - END_SLOP;
        self.distance = if confirmed { travel } else { 0 };
        confirmed
    }

    pub fn cancel(&mut self) {
        if self.start.is_some() {
            self.press(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_keeps_both_endpoints_in_the_assigned_area() {
        for area in [
            Rect::new(0, 388, 336, 88),
            Rect::new(50, 300, 236, 88),
            Rect::new(12, 400, 456, 100),
        ] {
            let layout = SliderLayout::measure(area).unwrap();
            assert!(area.contains(layout.track));
            assert!(area.contains(layout.caption));
            assert!(area.contains(Rect::new(
                layout.thumb.x + layout.travel,
                layout.thumb.y,
                layout.thumb.w,
                layout.thumb.h
            )));
        }
        let stock = SliderLayout::measure(Rect::new(0, 388, 336, 88)).unwrap();
        assert_eq!((stock.thumb.x, stock.thumb.y, stock.travel), (10, 394, 212));
    }

    #[test]
    fn short_drag_and_cancel_reset_and_release_confirms_once() {
        let mut drag = Drag::default();
        drag.press(Some(10));
        assert!(!drag.release(Some(216), 212));
        drag.press(Some(10));
        drag.move_to(Some(222), 212);
        drag.cancel();
        assert!(!drag.release(None, 212));
        drag.press(Some(10));
        assert!(drag.release(Some(217), 212));
        assert!(!drag.release(Some(222), 212));
        assert_eq!(drag.distance, 212);
    }
}
