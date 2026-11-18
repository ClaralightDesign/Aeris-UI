//! 由底轨与填充层组成的横向进度条。

use canopus_target_private::*;

use crate::{geometry::Rect, progress_ring::STEPS};

use super::lv::{self, Obj};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressBarOptions {
    pub show_background: bool,
    pub background_opacity: u8,
}

impl Default for ProgressBarOptions {
    fn default() -> Self {
        Self {
            show_background: true,
            background_opacity: 31,
        }
    }
}

pub struct ProgressBar {
    track: Obj,
    fill: Obj,
    width: i32,
    height: i32,
    fill_width: i32,
    background_opacity: u8,
}

impl ProgressBar {
    /// 创建初始为空的进度条，底轨与填充色使用 `0xRRGGBB`。
    ///
    /// # 安全要求
    /// 在 UI 线程调用，且父对象须在使用进度条期间保持有效。
    #[deprecated(
        note = "Use ProgressBar::new_with_options. Set background_opacity to 255 for the legacy track."
    )]
    pub unsafe fn new(parent: Obj, area: Rect, track_color: u32, fill_color: u32) -> Option<Self> {
        let options = ProgressBarOptions {
            background_opacity: 255,
            ..ProgressBarOptions::default()
        };
        unsafe { Self::new_with_options(parent, area, track_color, fill_color, options) }
    }

    /// Creates an empty bar with independently configurable track visibility.
    ///
    /// # Safety
    /// Call on the UI thread. The parent must remain valid while using the bar.
    pub unsafe fn new_with_options(
        parent: Obj,
        area: Rect,
        track_color: u32,
        fill_color: u32,
        options: ProgressBarOptions,
    ) -> Option<Self> {
        if parent.is_null() || area.w <= 0 || area.h <= 0 {
            return None;
        }
        let radius = (area.h / 2) as u32;
        unsafe {
            let track = lv::background(parent, area.w, area.h, track_color, radius);
            if track.is_null() {
                return None;
            }
            let fill = lv::background(track, area.w, area.h, fill_color, radius);
            if fill.is_null() {
                lvx_object_delete(track);
                return None;
            }
            lv::inspect_name(track, c"AerisUI.ProgressBar");
            lvx_object_align(track, 1, area.x, area.y);
            lvx_object_clear_flag(track, lv::CLICKABLE | lv::SCROLLABLE);
            lvx_object_clear_flag(fill, lv::CLICKABLE | lv::SCROLLABLE);
            lvx_set_hidden(fill, 1);
            let bar = Self {
                track,
                fill,
                width: area.w,
                height: area.h,
                fill_width: 0,
                background_opacity: options.background_opacity,
            };
            bar.set_show_background(options.show_background);
            Some(bar)
        }
    }

    /// 设置 `0..=STEPS` 的进度，超出上限时显示为满格。
    pub fn set(&mut self, steps: u32) {
        let width = (i64::from(self.width) * i64::from(steps.min(STEPS)) / i64::from(STEPS)) as i32;
        if self.fill_width == width {
            return;
        }
        unsafe {
            if width == 0 {
                lvx_set_hidden(self.fill, 1);
            } else {
                lvx_object_set_size(self.fill, width, self.height);
                if self.fill_width == 0 {
                    lvx_set_hidden(self.fill, 0);
                }
            }
        }
        self.fill_width = width;
    }

    pub fn set_show_background(&self, show: bool) {
        // The fill is a child of the track, so change the background's style
        // without hiding the track object itself.
        lv::style(
            self.track,
            lv::BG_OPA,
            if show {
                u32::from(self.background_opacity)
            } else {
                0
            },
        );
    }

    /// 更新底轨与填充层的 RGB 色值。
    pub fn set_colors(&self, track_color: u32, fill_color: u32) {
        lv::style(self.track, lv::BG_COLOR, track_color);
        lv::style(self.fill, lv::BG_COLOR, fill_color);
    }
}
