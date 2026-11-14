//! 等宽及自适应圆形排版的滚动数字。

use crate::{
    font::{DetailGlyph, DetailLayout, DETAIL_MAX_PX, DETAIL_SLOTS},
    geometry::Rect,
    motion::ease_out_cubic,
};
use canopus_target_private::*;

use super::{
    assets, font,
    lv::{self, Obj},
    page::{PAGE_TOP, WIDTH},
};

const RADIUS: i32 = 148;
const DIAMETER: i32 = RADIUS * 2;
const CIRCLE_Y: i32 = PAGE_TOP + 162;
const CENTER_OFFSET: i32 = 10;
pub const TOTAL_Y: i32 = PAGE_TOP + 197;
const TOTAL_GAP: i32 = 6;
const VIEWPORT_HEIGHT: i32 = TOTAL_Y - TOTAL_GAP - (CIRCLE_Y - RADIUS);
const ROLL_MS: u32 = 240;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Paint {
    glyph: DetailGlyph,
    opacity: u8,
}

#[derive(Clone, Copy)]
enum Layout {
    Fixed {
        width: i32,
        height: i32,
        font_px: i32,
        centered: bool,
    },
    Detail,
}

impl Layout {
    fn layout(self, text: &[u8]) -> Option<DetailLayout> {
        match self {
            Self::Fixed {
                width,
                height,
                font_px,
                centered,
            } => {
                if !(16..=DETAIL_MAX_PX).contains(&font_px) || height < font_px {
                    return None;
                }
                let (mut layout, advance) = DetailLayout::tabular(text, font_px)?;
                if advance > width {
                    return None;
                }
                let offset = if centered { (width - advance) / 2 } else { 0 };
                for glyph in layout.glyphs.iter_mut().flatten() {
                    glyph.x += offset;
                }
                Some(layout)
            }
            Self::Detail => DetailLayout::fit(text, RADIUS, CENTER_OFFSET),
        }
    }
}

pub struct RollingNumber {
    clip: Obj,
    layout: Layout,
    origin: [i32; 2],
    images: [[Obj; DETAIL_SLOTS]; 2],
    previous: [[Option<Paint>; DETAIL_SLOTS]; 2],
    from: DetailLayout,
    to: DetailLayout,
    started: Option<u32>,
    pending: Option<DetailLayout>,
}

impl RollingNumber {
    /// 在矩形视口内居中显示指定字号的数字。
    pub unsafe fn new(parent: Obj, area: Rect, text: &[u8], font_px: i32) -> Option<Self> {
        unsafe { Self::fixed(parent, area, text, font_px, true) }
    }

    pub(crate) unsafe fn fixed(
        parent: Obj,
        area: Rect,
        text: &[u8],
        font_px: i32,
        centered: bool,
    ) -> Option<Self> {
        let layout = Layout::Fixed {
            width: area.w,
            height: area.h,
            font_px,
            centered,
        };
        let initial = layout.layout(text)?;
        if !assets::fonts_ready() {
            return None;
        }
        let clip = unsafe { font::clip(parent, area.w, area.h, 0) }?;
        unsafe { lvx_object_align(clip, 1, area.x, area.y) };
        unsafe { Self::mount(clip, clip, layout, initial, [0, area.h / 2]) }
    }

    /// 按详情页的圆形空间自动匹配字号。
    pub unsafe fn detail(parent: Obj, text: &[u8]) -> Option<Self> {
        if !assets::fonts_ready() {
            return None;
        }
        let layout = Layout::Detail;
        let initial = layout.layout(text)?;
        let clip = unsafe { font::clip(parent, DIAMETER, DIAMETER, RADIUS as u32) }?;
        unsafe { lvx_object_align(clip, 1, WIDTH / 2 - RADIUS, CIRCLE_Y - RADIUS) };
        lv::style(clip, LV_STYLE_CLIP_CORNER, 1);
        let Some(viewport) = (unsafe { font::clip(clip, DIAMETER, VIEWPORT_HEIGHT, 0) }) else {
            unsafe { lvx_object_delete(clip) };
            return None;
        };
        unsafe {
            Self::mount(
                clip,
                viewport,
                layout,
                initial,
                [RADIUS, RADIUS - CENTER_OFFSET],
            )
        }
    }

    unsafe fn mount(
        clip: Obj,
        viewport: Obj,
        layout: Layout,
        initial: DetailLayout,
        origin: [i32; 2],
    ) -> Option<Self> {
        let mut images = [[core::ptr::null_mut(); DETAIL_SLOTS]; 2];
        for image in images.iter_mut().flatten() {
            *image = unsafe { lvx_image_create(viewport) };
            if image.is_null() {
                unsafe { lvx_object_delete(clip) };
                return None;
            }
            unsafe {
                lvx_object_clear_flag(*image, lv::CLICKABLE | lv::SCROLLABLE);
                lvx_set_hidden(*image, 1);
            }
        }

        let mut time = Self {
            clip,
            layout,
            origin,
            images,
            previous: [[None; DETAIL_SLOTS]; 2],
            from: initial,
            to: initial,
            started: None,
            pending: None,
        };
        unsafe { lv::inspect_name(clip, c"AerisUI.RollingNumber") };
        time.paint(1000);
        Some(time)
    }

    /// 修改固定视口的字号；字号超出范围或内容放不下时保留原样。
    pub fn set_font_px(&mut self, font_px: i32) -> bool {
        let Layout::Fixed {
            width,
            height,
            centered,
            ..
        } = self.layout
        else {
            return false;
        };
        let current = self.pending.unwrap_or(self.to);
        let mut text = [0; DETAIL_SLOTS];
        let mut len = 0;
        for glyph in current.glyphs.iter().flatten() {
            text[len] = if glyph.index == 10 {
                b':'
            } else {
                b'0' + glyph.index
            };
            len += 1;
        }
        let layout = Layout::Fixed {
            width,
            height,
            font_px,
            centered,
        };
        let Some(next) = layout.layout(&text[..len]) else {
            return false;
        };
        self.layout = layout;
        self.from = next;
        self.to = next;
        self.pending = None;
        self.started = None;
        self.paint(1000);
        true
    }

    pub fn set(&mut self, text: &[u8], now: u32, animate: bool) -> Option<bool> {
        let next = self.layout.layout(text)?;
        if animate && self.started.is_some() {
            self.pending = Some(next);
        } else {
            self.pending = None;
            self.from = self.to;
            self.to = next;
            self.started = animate.then_some(now);
        }
        if animate {
            Some(self.draw(now))
        } else {
            self.paint(1000);
            Some(false)
        }
    }

    pub fn finish(&mut self) {
        if self.started.is_none() {
            return;
        }
        if let Some(next) = self.pending.take() {
            self.to = next;
        }
        self.started = None;
        self.paint(1000);
    }

    pub fn delete(self) {
        unsafe { lvx_object_delete(self.clip) };
    }

    pub fn draw(&mut self, now: u32) -> bool {
        let Some(start) = self.started else {
            return false;
        };
        let mut elapsed = now.wrapping_sub(start);
        if elapsed >= ROLL_MS {
            if let Some(next) = self.pending.take() {
                self.from = self.to;
                self.to = next;
                self.started = Some(now);
                elapsed = 0;
            } else {
                self.started = None;
            }
        }
        let progress = ease_out_cubic((elapsed.min(ROLL_MS) * 1000 / ROLL_MS) as i32);
        self.paint(progress);
        self.started.is_some()
    }

    fn paint(&mut self, progress: i32) {
        let font_px = lerp(self.from.font_px, self.to.font_px, progress);
        let value = |layout: DetailLayout| {
            layout
                .glyphs
                .iter()
                .flatten()
                .filter(|glyph| glyph.index < 10)
                .fold(0, |number, glyph| number * 10 + i32::from(glyph.index))
        };
        let direction = if value(self.to) > value(self.from) {
            -1
        } else {
            1
        };
        let travel = direction * 14 * font_px / 30;
        for slot in 0..DETAIL_SLOTS {
            let old = self.from.glyphs[slot];
            let next = self.to.glyphs[slot];
            let (outgoing, incoming) = if progress == 1000 {
                (
                    None,
                    next.map(|glyph| Paint {
                        glyph,
                        opacity: 255,
                    }),
                )
            } else if let Some(anchor) = next.or(old) {
                let start = old.unwrap_or(anchor).center_x();
                let end = next.unwrap_or(anchor).center_x();
                let x = lerp(start, end, progress);
                if old.map(|glyph| glyph.index) == next.map(|glyph| glyph.index) {
                    (
                        None,
                        Some(Paint {
                            glyph: anchor.reposition(font_px, x, 0),
                            opacity: 255,
                        }),
                    )
                } else {
                    let outgoing = old.map(|glyph| Paint {
                        glyph: glyph.reposition(font_px, x, travel * progress / 1000),
                        opacity: lerp(255, 0, progress) as u8,
                    });
                    let incoming = next.map(|glyph| Paint {
                        glyph: glyph.reposition(font_px, x, travel * (progress - 1000) / 1000),
                        opacity: lerp(0, 255, progress) as u8,
                    });
                    (outgoing, incoming)
                }
            } else {
                (None, None)
            };
            for (layer, paint) in [outgoing, incoming].into_iter().enumerate() {
                patch(
                    self.origin,
                    self.images[layer][slot],
                    &mut self.previous[layer][slot],
                    paint,
                );
            }
        }
    }
}

fn lerp(from: i32, to: i32, progress: i32) -> i32 {
    from + (to - from) * progress / 1000
}

fn patch(
    [origin_x, origin_y]: [i32; 2],
    image: Obj,
    previous: &mut Option<Paint>,
    next: Option<Paint>,
) {
    let next = next.filter(|paint| paint.opacity != 0);
    if *previous == next {
        return;
    }

    unsafe {
        if let Some(paint) = next {
            let old_glyph = previous.map(|old| old.glyph);
            let glyph = paint.glyph;
            let changed_source = old_glyph.map(|old| old.index) != Some(glyph.index);
            if changed_source {
                lv::set_image_source(image, font::detail_source(glyph));
            }
            if changed_source || old_glyph.map(|old| old.scale) != Some(glyph.scale) {
                lvx_image_set_scale(image, glyph.scale, glyph.scale);
            }
            if old_glyph != Some(glyph) {
                // 缩放围绕图片中心，先将计算出的墨迹左上角换算为原图坐标。
                let (source_width, source_height) = glyph.source_size();
                let (width, height) = glyph.size();
                lvx_object_align(
                    image,
                    1,
                    origin_x + glyph.x + (width - source_width).div_euclid(2),
                    origin_y + glyph.y + (height - source_height).div_euclid(2),
                );
            }
            if previous.map(|old| old.opacity) != Some(paint.opacity) {
                lv::style(image, lv::OPA, u32::from(paint.opacity));
            }
            if previous.is_none() {
                lvx_set_hidden(image, 0);
            }
        } else {
            lvx_set_hidden(image, 1);
        }
    }
    *previous = next;
}
