//! 使用预渲染圆弧素材，分别绘制弧身和圆头端点。

use crate::{
    progress_ring::{quadrant_lengths, STEPS},
    theme::Color,
};
use canopus_target_private::*;

use super::{
    assets,
    lv::{self, Obj},
};

// 记录布局与素材生成脚本一致。
const FRAME_BYTES: usize = 11;
const ATLAS_WIDTH: i32 = 256;
const DESIGN_ACCENT: u32 = 0x0091FF;
const FRAME_TABLE_BYTES: usize = FRAME_BYTES * (STEPS as usize + 1);

struct RingArt {
    half: i32,
    start_x: i32,
    start_y: i32,
    clockwise: bool,
    tiles: [&'static [u8]; 4],
    start: &'static [u8],
    atlases: &'static [&'static [u8]],
    frames: &'static [u8; FRAME_TABLE_BYTES],
}

static CARD: RingArt = RingArt {
    half: 35,
    start_x: 35,
    start_y: 1,
    clockwise: false,
    tiles: assets::CARD_RING_TILES,
    start: assets::CARD_RING_START,
    atlases: &assets::CARD_RING_ENDS,
    frames: include_bytes!("../../assets/timer-ring-card.frames"),
};

static DETAIL: RingArt = RingArt {
    half: 162,
    start_x: 162,
    start_y: 0,
    clockwise: false,
    tiles: assets::DETAIL_RING_TILES,
    start: assets::DETAIL_RING_START,
    atlases: &assets::DETAIL_RING_ENDS,
    frames: include_bytes!("../../assets/timer-ring-detail.frames"),
};

pub const INDICATOR_SIZE: i32 = 62;
const INDICATOR: RingArt = RingArt {
    half: INDICATOR_SIZE / 2,
    start_x: INDICATOR_SIZE / 2 - 3,
    start_y: 0,
    clockwise: true,
    tiles: assets::PROGRESS_RING_FILL,
    start: assets::PROGRESS_RING_FILL_START,
    atlases: &assets::PROGRESS_RING_FILL_ENDS,
    frames: include_bytes!("../../assets/progress-ring-fill.frames"),
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgressRingStyle {
    #[default]
    Determinate,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressRingOptions {
    pub style: ProgressRingStyle,
    pub show_background: bool,
}

impl Default for ProgressRingOptions {
    fn default() -> Self {
        Self {
            style: ProgressRingStyle::Determinate,
            show_background: true,
        }
    }
}

enum Fill {
    Determinate(Ring),
    Indeterminate(Obj),
}

/// Standalone 62 px indicator. The backdrop and progress arc are separate layers.
pub struct CircularProgressIndicator {
    track: Obj,
    fill: Fill,
}

impl CircularProgressIndicator {
    /// # Safety
    /// Call on the UI thread. The parent must remain valid while using the indicator.
    pub unsafe fn new(parent: Obj, x: i32, y: i32, options: ProgressRingOptions) -> Option<Self> {
        if parent.is_null() {
            return None;
        }
        let root = unsafe { clip(parent, INDICATOR_SIZE, INDICATOR_SIZE) };
        if root.is_null() {
            return None;
        }
        unsafe {
            lvx_object_align(root, 1, x, y);
            lv::inspect_name(root, c"AerisUI.CircularProgressIndicator");
            let track = create_image(root, assets::PROGRESS_RING_TRACK, 0, 0);
            if track.is_null() {
                lvx_object_delete(root);
                return None;
            }
            let fill = match options.style {
                ProgressRingStyle::Determinate => {
                    Ring::new(root, 0, 0, &INDICATOR).map(Fill::Determinate)
                }
                ProgressRingStyle::Indeterminate => {
                    let image = create_image(root, assets::PROGRESS_RING_INDETERMINATE, 0, 0);
                    (!image.is_null()).then_some(Fill::Indeterminate(image))
                }
            };
            let Some(fill) = fill else {
                lvx_object_delete(root);
                return None;
            };
            let indicator = Self { track, fill };
            indicator.set_show_background(options.show_background);
            lv::recolor(track, Color::PrimaryText.resolve());
            indicator.set_color(Color::Accent.resolve_or(DESIGN_ACCENT));
            Some(indicator)
        }
    }

    /// Updates the determinate arc, clamped to 0–256 steps.
    pub fn set(&mut self, steps: u32) {
        if let Fill::Determinate(ring) = &mut self.fill {
            ring.set(steps);
        }
    }

    pub fn set_show_background(&self, show: bool) {
        unsafe { lvx_set_hidden(self.track, u32::from(!show)) };
    }

    /// Sets the fill color. The 12% track keeps the theme's foreground color.
    pub fn set_color(&self, color: u32) {
        match &self.fill {
            Fill::Determinate(ring) => ring.set_color(color),
            Fill::Indeterminate(image) => lv::recolor(*image, color),
        }
    }
}

struct Frame {
    length: i32,
    show_start: bool,
    atlas: usize,
    source_x: i32,
    source_y: i32,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
}

impl RingArt {
    fn frame(&self, steps: u32) -> Frame {
        let offset = steps as usize * FRAME_BYTES;
        let bytes = &self.frames[offset..offset + FRAME_BYTES];
        Frame {
            length: i32::from(bytes[0]),
            show_start: bytes[1] != 0,
            atlas: usize::from(bytes[2]),
            source_x: i32::from(bytes[3]),
            source_y: i32::from(bytes[4]),
            width: i32::from(bytes[5]),
            height: i32::from(bytes[6]),
            x: i32::from(u16::from_le_bytes([bytes[7], bytes[8]])),
            y: i32::from(u16::from_le_bytes([bytes[9], bytes[10]])),
        }
    }
}

struct Segment {
    clip: Obj,
    image: Obj,
    length: i32,
}

pub struct Ring {
    art: &'static RingArt,
    segments: [Segment; 4],
    start: Obj,
    end_clip: Obj,
    end_image: Obj,
    atlas: usize,
    steps: u32,
}

impl Ring {
    pub unsafe fn card(parent: Obj) -> Option<Self> {
        unsafe { Self::new(parent, 0, 0, &CARD) }
    }

    pub unsafe fn detail(parent: Obj, x: i32, y: i32) -> Option<Self> {
        unsafe { Self::new(parent, x, y, &DETAIL) }
    }

    unsafe fn new(parent: Obj, x: i32, y: i32, art: &'static RingArt) -> Option<Self> {
        let half = art.half;
        let layer = unsafe { clip(parent, half * 2, half * 2) };
        if layer.is_null() {
            return None;
        }
        unsafe { lv::inspect_name(layer, c"AerisUI.ProgressRing") };
        unsafe { lvx_object_align(layer, 1, x, y) };
        let segments = art.tiles.map(|asset| {
            let clip = unsafe { clip(layer, half, half) };
            let image = unsafe { create_image(clip, asset, 0, 0) };
            if !clip.is_null() {
                unsafe { lvx_set_hidden(clip, 1) };
            }
            Segment {
                clip,
                image,
                length: 0,
            }
        });
        let start = unsafe { create_image(layer, art.start, art.start_x, art.start_y) };
        let end_clip = unsafe { clip(layer, 1, 1) };
        let end_image = unsafe { create_image(end_clip, art.atlases[0], 0, 0) };
        if segments.iter().any(|segment| segment.image.is_null())
            || start.is_null()
            || end_image.is_null()
        {
            unsafe { lvx_object_delete(layer) };
            return None;
        }
        unsafe {
            lvx_set_hidden(start, 1);
            lvx_set_hidden(end_clip, 1);
        }
        Some(Self {
            art,
            segments,
            start,
            end_clip,
            end_image,
            atlas: 0,
            steps: 0,
        })
    }

    /// 设置弧身和圆头的 RGB 色值，格式为 `0xRRGGBB`。
    pub fn set_color(&self, color: u32) {
        for segment in &self.segments {
            lv::recolor(segment.image, color);
        }
        lv::recolor(self.start, color);
        lv::recolor(self.end_image, color);
    }

    pub fn set(&mut self, steps: u32) {
        let steps = steps.min(STEPS);
        if self.steps == steps {
            return;
        }
        let previous = self.art.frame(self.steps);
        let frame = self.art.frame(steps);
        let h = self.art.half;
        for (index, (segment, length)) in self
            .segments
            .iter_mut()
            .zip(quadrant_lengths(steps, h, frame.length))
            .enumerate()
        {
            if length == segment.length {
                continue;
            }
            unsafe {
                if length == 0 {
                    lvx_set_hidden(segment.clip, 1);
                } else {
                    let (x, y, width, height, image_x, image_y) = match index {
                        0 => (h - length, 0, length, h, length - h, 0),
                        1 => (0, h, h, length, 0, 0),
                        2 => (h, h, length, h, 0, 0),
                        _ => (h, h - length, h, length, 0, length - h),
                    };
                    let (x, image_x) = if self.art.clockwise {
                        (2 * h - x - width, width - image_x - h)
                    } else {
                        (x, image_x)
                    };
                    lvx_object_align(segment.clip, 1, x, y);
                    lvx_object_set_size(segment.clip, width, height);
                    lvx_object_align(segment.image, 1, image_x, image_y);
                    if segment.length == 0 {
                        lvx_set_hidden(segment.clip, 0);
                    }
                }
            }
            segment.length = length;
        }

        unsafe {
            if frame.show_start != previous.show_start {
                lvx_set_hidden(self.start, u32::from(!frame.show_start));
            }
            if frame.width != 0 {
                if frame.atlas != self.atlas {
                    lv::set_image_source(self.end_image, self.art.atlases[frame.atlas]);
                    self.atlas = frame.atlas;
                }
                let (x, source_x) = if self.art.clockwise {
                    (
                        2 * h - frame.x - frame.width,
                        ATLAS_WIDTH - frame.source_x - frame.width,
                    )
                } else {
                    (frame.x, frame.source_x)
                };
                lvx_object_align(self.end_clip, 1, x, frame.y);
                lvx_object_set_size(self.end_clip, frame.width, frame.height);
                lvx_object_align(self.end_image, 1, -source_x, -frame.source_y);
                if previous.width == 0 {
                    lvx_set_hidden(self.end_clip, 0);
                }
            } else if previous.width != 0 {
                lvx_set_hidden(self.end_clip, 1);
            }
        }
        self.steps = steps;
    }
}

unsafe fn clip(parent: Obj, width: i32, height: i32) -> Obj {
    let object = unsafe { lv::background(parent, width, height, 0, 0) };
    if !object.is_null() {
        unsafe { lvx_object_clear_flag(object, lv::CLICKABLE | lv::SCROLLABLE) };
        lv::style(object, lv::BG_OPA, 0);
    }
    object
}

unsafe fn create_image(parent: Obj, path: &[u8], x: i32, y: i32) -> Obj {
    if parent.is_null() {
        return parent;
    }
    let object = unsafe { lv::image(parent, path, x, y) };
    if !object.is_null() {
        unsafe { lvx_object_clear_flag(object, lv::CLICKABLE | lv::SCROLLABLE) };
    }
    object
}
