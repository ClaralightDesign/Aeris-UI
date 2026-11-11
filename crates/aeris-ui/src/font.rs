//! Linoval Sans 字形度量与详情页圆内排版。

mod data {
    use super::{Atlas, Glyph, Sprite};
    include!("font_data.rs");
}

pub use data::{CARD, CLOCK, OPTION, REGULAR};

#[derive(Clone, Copy)]
pub struct Glyph {
    code: u16,
    pub x: u16,
    pub y: u16,
    pub width: u8,
    pub height: u8,
    pub left: i16,
    pub top: i16,
    pub advance: u8,
}

impl Glyph {
    const fn new(
        code: u16,
        x: u16,
        y: u16,
        width: u8,
        height: u8,
        left: i16,
        top: i16,
        advance: u8,
    ) -> Self {
        Self {
            code,
            x,
            y,
            width,
            height,
            left,
            top,
            advance,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Atlas {
    pub ink_center: i32,
    glyphs: &'static [Glyph],
}

impl Atlas {
    const fn new(ink_center: i32, glyphs: &'static [Glyph]) -> Self {
        Self { ink_center, glyphs }
    }

    pub fn glyph(self, ch: char) -> Option<Glyph> {
        let code = u16::try_from(u32::from(ch)).ok()?;
        let index = self
            .glyphs
            .binary_search_by_key(&code, |glyph| glyph.code)
            .ok()?;
        Some(self.glyphs[index])
    }

    pub fn width(self, text: &str) -> Option<i32> {
        text.chars().try_fold(0, |width, ch| {
            Some(width + i32::from(self.glyph(ch)?.advance))
        })
    }
}

#[derive(Clone, Copy)]
struct Sprite {
    width: u8,
    height: u8,
    left: i8,
    top: i16,
    advance: u8,
}

impl Sprite {
    const fn new(width: u8, height: u8, left: i8, top: i16, advance: u8) -> Self {
        Self {
            width,
            height,
            left,
            top,
            advance,
        }
    }
}

pub const DETAIL_SLOTS: usize = 8;
pub const DETAIL_MIN_PX: i32 = 58;
pub const DETAIL_MAX_PX: i32 = data::DETAIL_SIZE;
pub const IMAGE_SCALE_ONE: i32 = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DetailGlyph {
    pub index: u8,
    pub x: i32,
    pub y: i32,
    pub scale: i32,
}

impl DetailGlyph {
    pub fn source_size(self) -> (i32, i32) {
        let source = data::DETAIL[usize::from(self.index)];
        (i32::from(source.width), i32::from(source.height))
    }

    pub fn center_x(self) -> i32 {
        self.x + self.size().0 / 2
    }

    pub fn reposition(self, font_px: i32, center_x: i32, offset_y: i32) -> Self {
        let mut glyph = Self::at_size(self.index, 0, font_px);
        glyph.x = center_x - glyph.size().0 / 2;
        glyph.y += offset_y;
        glyph
    }

    fn at_size(index: u8, pen: i32, font_px: i32) -> Self {
        let sprite = data::DETAIL[usize::from(index)];
        Self {
            index,
            x: scaled(pen + i32::from(sprite.left), font_px),
            y: scaled(i32::from(sprite.top) - data::DETAIL_INK_CENTER, font_px),
            scale: font_px * IMAGE_SCALE_ONE / DETAIL_MAX_PX,
        }
    }

    pub fn size(self) -> (i32, i32) {
        let (width, height) = self.source_size();
        (extent(width, self.scale), extent(height, self.scale))
    }
}

#[derive(Clone, Copy)]
pub struct DetailLayout {
    pub glyphs: [Option<DetailGlyph>; DETAIL_SLOTS],
    pub font_px: i32,
}

impl DetailLayout {
    fn indices(text: &[u8]) -> Option<[Option<u8>; DETAIL_SLOTS]> {
        let first_slot = DETAIL_SLOTS.checked_sub(text.len())?;
        if text.is_empty() {
            return None;
        }
        let mut indices = [None; DETAIL_SLOTS];
        for (slot, &byte) in indices[first_slot..].iter_mut().zip(text) {
            *slot = Some(match byte {
                b'0'..=b'9' => byte - b'0',
                b':' => 10,
                _ => return None,
            });
        }
        Some(indices)
    }

    /// 按字号生成居中的数字布局。
    pub fn sized(text: &[u8], font_px: i32) -> Option<Self> {
        Some(Self::at_size(Self::indices(text)?, font_px))
    }

    /// 使用等宽数字槽，固定字号下未变化的字形保持原位。
    pub fn tabular(text: &[u8], font_px: i32) -> Option<(Self, i32)> {
        let indices = Self::indices(text)?;
        let mut cursor = 0;
        let glyphs = indices.map(|index| {
            let index = index?;
            let sprite = data::DETAIL[usize::from(index)];
            let advance = if index == 10 {
                i32::from(sprite.advance)
            } else {
                47
            };
            let mut glyph = DetailGlyph::at_size(index, 0, font_px);
            glyph.x =
                (scaled(cursor, font_px) + scaled(cursor + advance, font_px) - glyph.size().0) / 2;
            cursor += advance;
            Some(glyph)
        });
        Some((Self { glyphs, font_px }, scaled(cursor, font_px)))
    }

    /// 坐标相对数字区中心，圆心的纵向偏移由前端指定。
    pub fn fit(text: &[u8], radius: i32, circle_y: i32) -> Option<Self> {
        let indices = Self::indices(text)?;
        let largest = Self::at_size(indices, DETAIL_MAX_PX);
        if largest.inside(radius, circle_y) {
            return Some(largest);
        }

        let mut low = DETAIL_MIN_PX;
        let mut high = DETAIL_MAX_PX - 1;
        while low < high {
            let mid = (low + high + 1) / 2;
            if Self::at_size(indices, mid).inside(radius, circle_y) {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        let layout = Self::at_size(indices, low);
        layout.inside(radius, circle_y).then_some(layout)
    }

    fn at_size(indices: [Option<u8>; DETAIL_SLOTS], font_px: i32) -> Self {
        let mut cursor = 0;
        let mut left = i32::MAX;
        let mut right = i32::MIN;
        let mut glyphs = indices.map(|index| {
            let index = index?;
            let glyph = DetailGlyph::at_size(index, cursor, font_px);
            cursor += i32::from(data::DETAIL[usize::from(index)].advance);
            let (width, _) = glyph.size();
            left = left.min(glyph.x);
            right = right.max(glyph.x + width);
            Some(glyph)
        });
        let center = (left + right) / 2;
        for glyph in glyphs.iter_mut().flatten() {
            glyph.x -= center;
        }
        Self { glyphs, font_px }
    }

    fn inside(self, radius: i32, circle_y: i32) -> bool {
        self.glyphs.iter().flatten().all(|glyph| {
            let (width, height) = glyph.size();
            let x = glyph.x.abs().max((glyph.x + width).abs());
            let top = glyph.y - circle_y;
            let y = top.abs().max((top + height).abs());
            x * x + y * y <= radius * radius
        })
    }
}

fn scaled(value: i32, font_px: i32) -> i32 {
    (value * font_px + DETAIL_MAX_PX / 2).div_euclid(DETAIL_MAX_PX)
}

fn extent(value: i32, scale: i32) -> i32 {
    (value * scale + IMAGE_SCALE_ONE - 1) / IMAGE_SCALE_ONE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_coverage_and_card_width() {
        for c in (' '..='~').chain(['\u{a0}', '°', '·', '–', '−']) {
            assert!(REGULAR.glyph(c).is_some(), "{c}");
        }
        assert!(CLOCK.glyph('A').is_none());
        assert!(OPTION.glyph('−').is_some());
        assert!((b'0'..=b'9').all(|digit| CARD.glyph(char::from(digit)).unwrap().advance == 19));
        assert_eq!(CARD.width("0:11:11"), Some(113));
        assert_eq!(CARD.width("0:11:10"), Some(113));
        assert_eq!(CARD.width("9:59:59"), Some(113));
    }

    #[test]
    fn device_info_digits_are_tabular_and_timestamp_fits() {
        let zero = REGULAR.glyph('0').unwrap().advance;
        assert!(('1'..='9').all(|digit| REGULAR.glyph(digit).unwrap().advance == zero));
        assert!(REGULAR.width("2026-11-07T11:09:52Z").unwrap() <= 296);
    }

    #[test]
    fn detail_size_stays_between_58_and_72() {
        let time = b"9:59:59";
        assert_eq!(DetailLayout::fit(time, 148, 10).unwrap().font_px, 72);

        let radius = (1..148)
            .find(|&radius| DetailLayout::fit(time, radius, 10).is_some())
            .unwrap();
        assert_eq!(DetailLayout::fit(time, radius, 10).unwrap().font_px, 58);
        assert!(DetailLayout::fit(time, radius - 1, 10).is_none());
    }

    #[test]
    fn all_detail_times_fit_and_leave_room_for_total() {
        for second in 0..36_000 {
            let hours = second / 3600;
            let minutes = (second / 60) % 60;
            let seconds = second % 60;
            let text = if hours == 0 {
                format!("{minutes}:{seconds:02}")
            } else {
                format!("{hours}:{minutes:02}:{seconds:02}")
            };
            let layout = DetailLayout::fit(text.as_bytes(), 148, 10).unwrap();
            assert!(layout.inside(148, 10), "{text}");
            assert!((58..=72).contains(&layout.font_px), "{text}");
            for glyph in layout.glyphs.iter().flatten() {
                assert!(234 + glyph.y + glyph.size().1 <= 279 - 6, "{text}");
            }
        }
        assert!(DetailLayout::fit(b"1.00", 148, 10).is_none());
    }
}
