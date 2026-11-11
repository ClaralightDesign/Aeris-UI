//! 数字滚轮的布局与循环刻度。

use crate::geometry::Rect;

pub(crate) const HEIGHT: i32 = 200;
pub(crate) const COLUMN_WIDTH: i32 = 92;
pub(crate) const STEP: i32 = 74;
pub(crate) const CENTER: i32 = 120 * STEP;

pub(crate) fn columns<const N: usize>(bounds: Rect) -> Option<[Rect; N]> {
    if !(1..=3).contains(&N) || bounds.h < HEIGHT {
        return None;
    }
    let gap = 22.min((bounds.w - 72 * N as i32) / (N as i32 - 1).max(1));
    if gap < 0 {
        return None;
    }
    let width = COLUMN_WIDTH.min((bounds.w - gap * (N as i32 - 1)) / N as i32);
    if width < 72 {
        return None;
    }
    let start = bounds.x + (bounds.w - N as i32 * width - (N as i32 - 1) * gap) / 2;
    Some(core::array::from_fn(|index| {
        Rect::new(
            start + index as i32 * (width + gap),
            bounds.y,
            width,
            HEIGHT,
        )
    }))
}

pub(crate) fn nearest_step(position: i32) -> i32 {
    (position + STEP / 2).div_euclid(STEP)
}

pub(crate) fn value_at(step: i32, min: u8, max: u8) -> u8 {
    let count = i32::from(max) - i32::from(min) + 1;
    min + (step - CENTER / STEP).rem_euclid(count) as u8
}

pub(crate) fn centered_position(value: u8, min: u8, max: u8) -> i32 {
    let offset = i32::from(value.clamp(min, max) - min);
    let count = i32::from(max) - i32::from(min) + 1;
    // 旧时分秒布局保留原来的起始滚动坐标。
    let offset = if min == 0 && matches!(max, 9 | 59) {
        offset
    } else if offset > count / 2 {
        offset - count
    } else {
        offset
    };
    CENTER + offset * STEP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_fit_stock_narrow_and_wide_areas() {
        let stock = columns::<3>(Rect::new(0, 82, 336, 200)).unwrap();
        assert_eq!(stock.map(|column| column.x), [8, 122, 236]);
        for area in [Rect::new(38, 120, 242, 200), Rect::new(16, 112, 448, 300)] {
            let cols = columns::<3>(area).unwrap();
            assert!(cols.iter().all(|&column| area.contains(column)));
            assert!(cols.windows(2).all(|pair| !pair[0].overlaps(pair[1])));
        }
        assert!(columns::<3>(Rect::new(0, 0, 210, 200)).is_none());
    }

    #[test]
    fn arbitrary_ranges_wrap_and_recentering_preserves_the_value() {
        let center = CENTER / STEP;
        assert_eq!(value_at(center - 1, 7, 18), 18);
        assert_eq!(value_at(center + 12, 7, 18), 7);
        for (min, max) in [(0, 9), (0, 59), (7, 18), (0, 99), (7, 7)] {
            for value in min..=max {
                let position = centered_position(value, min, max);
                assert_eq!(value_at(nearest_step(position), min, max), value);
            }
        }
    }
}
