//! 计时器的静态字形图集渲染。

use crate::font::{self, Atlas, Glyph};
use canopus_target_private::*;

use super::{
    assets, font_assets,
    lv::{self, Font, Obj},
};

#[derive(Clone, Copy)]
pub struct BitmapFont {
    pub atlas: Atlas,
    pub path: &'static [u8],
}

pub const CLOCK: BitmapFont = BitmapFont {
    atlas: font::CLOCK,
    path: font_assets::CLOCK,
};
pub const CLOCK_DARK: BitmapFont = BitmapFont {
    atlas: font::CLOCK,
    path: font_assets::CLOCK_DARK,
};

pub const CARD: BitmapFont = BitmapFont {
    atlas: font::CARD,
    path: font_assets::CARD,
};

pub const REGULAR: BitmapFont = BitmapFont {
    atlas: font::REGULAR,
    path: font_assets::REGULAR,
};

pub const OPTION: BitmapFont = BitmapFont {
    atlas: font::OPTION,
    path: font_assets::OPTION,
};

pub fn detail_source(glyph: crate::font::DetailGlyph) -> &'static [u8] {
    font_assets::DETAIL[usize::from(glyph.index)]
}

pub unsafe fn clip(parent: Obj, width: i32, height: i32, radius: u32) -> Option<Obj> {
    if parent.is_null() {
        return None;
    }
    let object = unsafe { lv::background(parent, width, height, 0, radius) };
    if object.is_null() {
        return None;
    }
    unsafe {
        lvx_object_clear_flag(
            object,
            lv::CLICKABLE
                | lv::SCROLLABLE
                | lv::SCROLL_CHAIN
                | lv::GESTURE_BUBBLE
                | lv::OVERFLOW_VISIBLE,
        );
    }
    lv::style(object, lv::BG_OPA, 0);
    Some(object)
}

#[derive(Clone, Copy)]
pub struct Tile {
    pub root: Obj,
    image: Obj,
}

impl Tile {
    pub unsafe fn new(parent: Obj, path: &[u8]) -> Option<Self> {
        let root = unsafe { clip(parent, 1, 1, 0) }?;
        let image = unsafe { lv::image(root, path, 0, 0) };
        if image.is_null() {
            unsafe { lvx_object_delete(root) };
            return None;
        }
        unsafe { lvx_object_clear_flag(image, lv::CLICKABLE | lv::SCROLLABLE) };
        Some(Self { root, image })
    }

    pub unsafe fn select(self, glyph: Glyph) {
        unsafe {
            lvx_object_set_size(self.root, i32::from(glyph.width), i32::from(glyph.height));
            lvx_object_align(self.image, 1, -i32::from(glyph.x), -i32::from(glyph.y));
        }
    }

    pub unsafe fn place(self, glyph: Glyph, pen: i32, baseline: i32) {
        unsafe {
            lvx_object_align(
                self.root,
                1,
                pen + i32::from(glyph.left),
                baseline + i32::from(glyph.top),
            );
        }
    }
}

pub unsafe fn fixed(
    parent: Obj,
    bitmap: BitmapFont,
    text: &[u8],
    area: [i32; 4],
    align: u32,
) -> Option<Obj> {
    if !assets::fonts_ready() {
        return None;
    }
    let [x, y, width, height] = area;
    let text = core::str::from_utf8(text.strip_suffix(b"\0").unwrap_or(text)).ok()?;
    let text_width = bitmap.atlas.width(text)?;
    if text_width > width {
        return None;
    }
    let root = unsafe { clip(parent, width, height, 0) }?;
    unsafe { lvx_object_align(root, 1, x, y) };
    let mut pen = match align {
        2 => (width - text_width) / 2,
        3 => width - text_width,
        _ => 0,
    };
    let baseline = height / 2 - bitmap.atlas.ink_center;
    for ch in text.chars() {
        let Some(glyph) = bitmap.atlas.glyph(ch) else {
            unsafe { lvx_object_delete(root) };
            return None;
        };
        if glyph.width != 0 {
            let Some(tile) = (unsafe { Tile::new(root, bitmap.path) }) else {
                unsafe { lvx_object_delete(root) };
                return None;
            };
            unsafe {
                tile.select(glyph);
                tile.place(glyph, pen, baseline);
            }
        }
        pen += i32::from(glyph.advance);
    }
    Some(root)
}

pub unsafe fn clock(parent: Obj, text: &[u8], area: [i32; 4], align: u32) -> (Obj, bool) {
    if let Some(object) = unsafe { fixed(parent, CLOCK, text, area, align) } {
        unsafe { lv::inspect_name(object, c"AerisUI.Clock") };
        return (object, true);
    }
    let [x, y, width, height] = area;
    let label = unsafe { lv::label(parent, text, x, y, width, height, 0xFFFFFF, Font::Small24) };
    if align == 3 {
        lv::style(label, 94, 3);
    }
    unsafe { lv::inspect_name(label, c"AerisUI.Clock") };
    lv::text_px(label, width, height, align, Font::Small24, 26);
    (label, false)
}
