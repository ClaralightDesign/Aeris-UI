//! 屏幕边界与组件布局区域，单位为像素。

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn right(self) -> i32 {
        self.x + self.w
    }

    pub const fn bottom(self) -> i32 {
        self.y + self.h
    }

    pub const fn center_x(self) -> i32 {
        self.x + self.w / 2
    }

    pub const fn center_y(self) -> i32 {
        self.y + self.h / 2
    }

    pub const fn horizontal_inset(self, inset: i32) -> Option<Self> {
        if inset < 0 || self.w <= 0 || self.h <= 0 || inset > (self.w - 1) / 2 {
            return None;
        }
        Some(Self::new(
            self.x + inset,
            self.y,
            self.w - 2 * inset,
            self.h,
        ))
    }

    pub const fn contains(self, other: Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    pub const fn overlaps(self, other: Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}

/// 屏幕边界与由设备提供的矩形安全区，坐标相对页面父容器。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Viewport {
    bounds: Rect,
    safe: Rect,
}

impl Viewport {
    pub const BAND_10_PRO: Self = Self {
        bounds: Rect::new(0, 0, 336, 480),
        safe: Rect::new(0, 0, 336, 480),
    };

    pub const fn new(bounds: Rect, safe: Rect) -> Option<Self> {
        if bounds.w > 0 && bounds.h > 0 && safe.w > 0 && safe.h > 0 && bounds.contains(safe) {
            Some(Self { bounds, safe })
        } else {
            None
        }
    }

    pub const fn bounds(self) -> Rect {
        self.bounds
    }

    /// 返回相对屏幕根对象的安全区。
    pub const fn content(self) -> Rect {
        Rect::new(
            self.safe.x - self.bounds.x,
            self.safe.y - self.bounds.y,
            self.safe.w,
            self.safe.h,
        )
    }
}

/// 方形按钮或图标区域。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Square {
    pub x: i32,
    pub y: i32,
    pub size: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_resolves_the_safe_area_relative_to_its_root() {
        let narrow =
            Viewport::new(Rect::new(10, 20, 320, 480), Rect::new(50, 88, 240, 344)).unwrap();
        let wide = Viewport::new(Rect::new(0, 0, 480, 480), Rect::new(12, 12, 456, 456)).unwrap();
        assert_eq!(narrow.content(), Rect::new(40, 68, 240, 344));
        assert_eq!(
            wide.content().horizontal_inset(3),
            Some(Rect::new(15, 12, 450, 456))
        );
        assert_eq!(Viewport::BAND_10_PRO.content(), Rect::new(0, 0, 336, 480));
        assert!(Viewport::new(wide.bounds(), Rect::new(-1, 0, 480, 480)).is_none());
    }
}
