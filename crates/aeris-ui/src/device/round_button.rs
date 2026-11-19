//! Icon actions with circular or capsule-edge faces.

use canopus_target_private::*;

use crate::{
    geometry::Rect,
    theme::{self, Color},
};

use super::{
    anim, assets, font,
    lv::{self, Obj},
    page::ButtonTheme,
};

pub(super) const STANDARD_FLAT_NEUTRAL: &[u8] = asset_path!("round-standard-normal-neutral.bin");
pub(super) const STANDARD_FLAT_ACCENT: &[u8] = asset_path!("round-standard-normal-accent.bin");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Center,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Neutral,
    Primary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Settings,
    Add,
    Back,
    Close,
    Check,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub position: Position,
    pub status: Status,
    pub theme: ButtonTheme,
}

impl Options {
    /// The edge buttons use the width of the capsule's curved end.
    pub fn bounds(self, slot: Rect) -> Option<Rect> {
        let (width, height) = match self.position {
            Position::Center if slot.w >= 76 && slot.h >= 76 => (76, 76),
            Position::Center => (68, 68),
            _ => (150, 78),
        };
        if slot.w < width || slot.h < height {
            return None;
        }
        let y = match self.position {
            Position::Center => slot.center_y() - height / 2,
            Position::Top => slot.y,
            Position::Bottom => slot.bottom() - height,
        };
        Some(Rect::new(slot.center_x() - width / 2, y, width, height))
    }
}

impl Icon {
    fn asset(self, face: Rect) -> (&'static [u8], i32, &'static core::ffi::CStr) {
        let size = face.w;
        match self {
            Self::Settings if size == 150 => (asset_path!("round-settings-edge.bin"), 50, c"Settings"),
            Self::Settings if size == 76 => (asset_path!("round-settings-wide.bin"), 45, c"Settings"),
            Self::Settings => (assets::ICON_SETTINGS, 40, c"Settings"),
            Self::Back if size == 150 => (asset_path!("round-back-edge.bin"), 44, c"Back"),
            Self::Back if size == 76 => (asset_path!("round-back-wide.bin"), 45, c"Back"),
            Self::Back => (assets::ICON_BACK, 40, c"Back"),
            Self::Close if size == 76 => (asset_path!("round-close-wide.bin"), 45, c"Close"),
            Self::Close => (assets::ICON_X, 40, c"Close"),
            Self::Add if size == 76 => (asset_path!("round-add-wide.bin"), 45, c"Add"),
            Self::Add => (assets::ICON_ADD, 40, c"Add"),
            Self::Check => (asset_path!("round-check-edge.bin"), 44, c"Confirm"),
        }
    }
}

/// Builds an icon button in a layout slot. An absent callback produces a
/// decorative icon with no hit target.
///
/// # Safety
/// Call on the UI thread with a live parent.
pub unsafe fn create(
    parent: Obj,
    slot: Rect,
    icon: Icon,
    options: Options,
    on_click: Option<LvxEventCallback>,
    data: usize,
) -> Option<Obj> {
    use ButtonTheme::{Glass, Normal};
    use Position::{Bottom, Center, Top};
    use Status::{Neutral, Primary};

    let bounds = options.bounds(slot)?;
    let wide = bounds.w == 76;
    let face = match (options.position, options.theme, options.status, wide) {
        (Center, Glass, Neutral, false) => asset_path!("round-standard-glass-neutral.bin"),
        (Center, Glass, Primary, false) => asset_path!("round-standard-glass-accent.bin"),
        (Center, Normal, Neutral, false) => STANDARD_FLAT_NEUTRAL,
        (Center, Normal, Primary, false) => STANDARD_FLAT_ACCENT,
        (Center, Glass, Neutral, true) => asset_path!("round-wide-glass-neutral.bin"),
        (Center, Glass, Primary, true) => asset_path!("round-wide-glass-accent.bin"),
        (Center, Normal, Neutral, true) => asset_path!("round-wide-normal-neutral.bin"),
        (Center, Normal, Primary, true) => asset_path!("round-wide-normal-accent.bin"),
        (Top, Glass, Neutral, _) => asset_path!("round-top-glass-neutral.bin"),
        (Top, Glass, Primary, _) => asset_path!("round-top-glass-accent.bin"),
        (Top, Normal, Neutral, _) => asset_path!("round-top-normal-neutral.bin"),
        (Top, Normal, Primary, _) => asset_path!("round-top-normal-accent.bin"),
        (Bottom, Glass, Neutral, _) => asset_path!("round-bottom-glass-neutral.bin"),
        (Bottom, Glass, Primary, _) => asset_path!("round-bottom-glass-accent.bin"),
        (Bottom, Normal, Neutral, _) => asset_path!("round-bottom-normal-neutral.bin"),
        (Bottom, Normal, Primary, _) => asset_path!("round-bottom-normal-accent.bin"),
    };
    let (glyph, size, name) = icon.asset(bounds);

    unsafe {
        let root = font::clip(parent, bounds.w, bounds.h, 0)?;
        lvx_object_align(root, 1, bounds.x, bounds.y);
        lv::inspect_name(root, name);
        if options.position == Center {
            lv::inspect_circle(root);
        }
        if options.position == Center && options.theme == Glass && options.status == Neutral {
            lv::style(root, lv::RADIUS, (bounds.w / 2) as u32);
            lv::style(root, lv::BG_COLOR, Color::Background.resolve());
            lv::style(root, lv::BG_OPA, 255);
        }
        let face = lv::image(root, face, 0, 0);
        let glyph = lv::image(root, glyph, (bounds.w - size) / 2, (bounds.h - size) / 2);
        if face.is_null() || glyph.is_null() {
            lvx_object_delete(root);
            return None;
        }
        for child in [face, glyph] {
            lvx_object_clear_flag(child, lv::CLICKABLE | lv::SCROLLABLE);
        }
        if options.position == Center && options.theme == Glass && options.status == Neutral {
            lv::style(face, lv::OPA, 100);
        }
        lv::recolor(
            glyph,
            if options.status == Primary {
                Color::PrimaryText.resolve_or(
                    theme::accent_button().map_or(0xD7E8FF, |button| button.foreground),
                )
            } else {
                Color::PrimaryText.resolve_or(0xDDDDDD)
            },
        );
        if let Some(on_click) = on_click {
            lvx_object_add_flag(root, lv::CLICKABLE);
            anim::attach(root, bounds.w / 2, bounds.h / 2, anim::Feedback::Scale);
            lv::bind(root, on_click, data);
        }
        Some(root)
    }
}
