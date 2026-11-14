//! 按需将压缩素材还原为固件可读的 LVGL 位图。

#[cfg(not(target_arch = "wasm32"))]
use canopus_abi::{CanopusContext, FS_CREATE, FS_TRUNC, FS_WRITE};
#[cfg(not(target_arch = "wasm32"))]
use canopus_runtime::{fs, mem};

#[cfg(not(target_arch = "wasm32"))]
use super::{context, packed_assets};

pub const HEADER_LEFT: &[u8] = asset_path!("header-left.bin");
pub const HEADER_RIGHT: &[u8] = asset_path!("header-right.bin");
pub const HEADER_BACKGROUND: &[u8] = asset_path!("header-background.bin");
pub const FOOTER_BACKGROUND: &[u8] = asset_path!("footer-background.bin");

pub const BUTTON_BLACK: &[u8] = asset_path!("header-button-black.bin");
pub const BUTTON_ABOUT: &[u8] = asset_path!("header-button-about.bin");
pub const BUTTON_PRIMARY: &[u8] = asset_path!("header-button-primary.bin");
pub const ICON_SETTINGS: &[u8] = asset_path!("header-icon-settings.bin");
pub const ICON_ADD: &[u8] = asset_path!("header-icon-add.bin");
pub const ICON_ADD_ORANGE: &[u8] = asset_path!("header-icon-add-orange.bin");
pub const ICON_BACK: &[u8] = asset_path!("header-icon-back.bin");
pub const ICON_HIDE: &[u8] = asset_path!("header-icon-hide.bin");
pub const ICON_X: &[u8] = asset_path!("header-icon-x.bin");
pub const ICON_PLAY: &[u8] = asset_path!("header-icon-play.bin");
pub const ICON_PAUSE: &[u8] = asset_path!("header-icon-pause.bin");

pub const CARD_FACE: &[u8] = asset_path!("card-face.bin");
pub const CHECKBOX_MINUS: &[u8] = asset_path!("checkbox-minus.bin");
pub const CHECKBOX_CHECK: &[u8] = asset_path!("checkbox-check.bin");
pub(super) const CONFIRMATION_SUCCESS: &[u8] = asset_path!("confirmation-success.bin");
pub const SLIDER_MINUS: &[u8] = asset_path!("slider-minus.bin");
pub const SLIDER_PLUS: &[u8] = asset_path!("slider-plus.bin");

pub const CAPSULE_LEFT: &[u8] = asset_path!("capsule-left.bin");
pub const CAPSULE_CENTER_WIDE: &[u8] = asset_path!("capsule-center-wide.bin");
pub const CAPSULE_RIGHT: &[u8] = asset_path!("capsule-right.bin");
pub const CAPSULE_LARGE_LEFT: &[u8] = asset_path!("capsule-large-neutral-v2-left.bin");
pub const CAPSULE_LARGE_CENTER_WIDE: &[u8] =
    asset_path!("capsule-large-neutral-v2-center-wide.bin");
pub const CAPSULE_LARGE_RIGHT: &[u8] = asset_path!("capsule-large-neutral-v2-right.bin");
pub const CAPSULE_PRIMARY_LEFT: &[u8] = if cfg!(feature = "gallery") {
    asset_path!("gallery-capsule-primary-left.bin")
} else {
    asset_path!("capsule-primary-left.bin")
};
pub const CAPSULE_PRIMARY_CENTER_WIDE: &[u8] = if cfg!(feature = "gallery") {
    asset_path!("gallery-capsule-primary-center-wide.bin")
} else {
    asset_path!("capsule-primary-center-wide.bin")
};
pub const CAPSULE_PRIMARY_RIGHT: &[u8] = if cfg!(feature = "gallery") {
    asset_path!("gallery-capsule-primary-right.bin")
} else {
    asset_path!("capsule-primary-right.bin")
};

pub const CARD_TIMER_START: &[u8] = asset_path!("card-timer-start.bin");
pub const CARD_TIMER_PLAY: &[u8] = asset_path!("card-timer-play.bin");
pub const CARD_TIMER_PAUSE: &[u8] = asset_path!("card-timer-pause.bin");
pub const CARD_CHEVRON: &[u8] = asset_path!("card-chevron.bin");
pub const EDITOR_STAGE_PREV: &[u8] = asset_path!("editor-stage-prev.bin");
pub const EDITOR_STAGE_NEXT: &[u8] = asset_path!("editor-stage-next.bin");
pub const EDITOR_WHEEL_SHADE: &[u8] = asset_path!("editor-wheel-shade.bin");
pub const CARD_SWITCH_ON_BG: &[u8] = asset_path!("card-switch-on-bg.bin");
pub const CARD_SWITCH_OFF_BG: &[u8] = asset_path!("card-switch-off-bg.bin");
pub const CARD_SWITCH_THUMB: &[u8] = asset_path!("card-switch-thumb.bin");
#[cfg(feature = "gallery")]
pub const CARD_STATUS_WARNING: &[u8] = asset_path!("card-status-warning.bin");
#[cfg(feature = "gallery")]
pub const CARD_STATUS_ERROR: &[u8] = asset_path!("card-status-error.bin");
pub const CARD_STATUS_SUCCESS: &[u8] = asset_path!("card-status-success.bin");

pub const TIMER_DETAIL_TRACK: &[u8] = asset_path!("timer-detail-track.bin");
pub const TIMER_CAPSULE_SHORT: &[u8] = asset_path!("timer-capsule-shortbg.bin");

pub const CARD_RING_TILES: [&[u8]; 4] = [
    asset_path!("timer-ring-card-0.bin"),
    asset_path!("timer-ring-card-1.bin"),
    asset_path!("timer-ring-card-2.bin"),
    asset_path!("timer-ring-card-3.bin"),
];
pub const CARD_RING_START: &[u8] = asset_path!("timer-ring-card-start.bin");
pub const CARD_RING_ENDS: [&[u8]; 1] = [asset_path!("timer-ring-card-ends-0.bin")];
pub const DETAIL_RING_TILES: [&[u8]; 4] = [
    asset_path!("timer-ring-detail-0.bin"),
    asset_path!("timer-ring-detail-1.bin"),
    asset_path!("timer-ring-detail-2.bin"),
    asset_path!("timer-ring-detail-3.bin"),
];
pub const DETAIL_RING_START: &[u8] = asset_path!("timer-ring-detail-start.bin");
pub const DETAIL_RING_ENDS: [&[u8]; 2] = [
    asset_path!("timer-ring-detail-ends-0.bin"),
    asset_path!("timer-ring-detail-ends-1.bin"),
];

pub const STOP_TRACK: &[u8] = asset_path!("stop-track-88.bin");
pub const STOP_FORE: &[u8] = asset_path!("stop-slider-fore.bin");
pub const EXIT_FORE: &[u8] = asset_path!("exit-slider-fore.bin");
#[cfg(feature = "gallery")]
pub const STOP_CAPTION_ICON: &[u8] = asset_path!("stop-caption-icon.bin");
#[cfg(feature = "gallery")]
pub const EXIT_CAPTION_ICON: &[u8] = asset_path!("exit-caption-icon.bin");

macro_rules! asset {
    ($name:literal, $file:literal) => {
        (
            $name,
            include_bytes!(concat!("../../assets/", $file, ".lz4")),
        )
    };
}

pub const ALL: &[(&str, &[u8])] = &[
    asset!("confirmation-success.bin", "confirmation-success.bin"),
    asset!("footer-background.bin", "footer-background.bin"),
    asset!("timer-ring-card-0.bin", "timer-ring-card-0.bin"),
    asset!("timer-ring-card-1.bin", "timer-ring-card-1.bin"),
    asset!("timer-ring-card-2.bin", "timer-ring-card-2.bin"),
    asset!("timer-ring-card-3.bin", "timer-ring-card-3.bin"),
    asset!("timer-ring-card-start.bin", "timer-ring-card-start.bin"),
    asset!("timer-ring-card-ends-0.bin", "timer-ring-card-ends-0.bin"),
    asset!("timer-ring-detail-0.bin", "timer-ring-detail-0.bin"),
    asset!("timer-ring-detail-1.bin", "timer-ring-detail-1.bin"),
    asset!("timer-ring-detail-2.bin", "timer-ring-detail-2.bin"),
    asset!("timer-ring-detail-3.bin", "timer-ring-detail-3.bin"),
    asset!("timer-ring-detail-start.bin", "timer-ring-detail-start.bin"),
    asset!(
        "timer-ring-detail-ends-0.bin",
        "timer-ring-detail-ends-0.bin"
    ),
    asset!(
        "timer-ring-detail-ends-1.bin",
        "timer-ring-detail-ends-1.bin"
    ),
    asset!("timer-detail-track.bin", "timer-detail-track.bin"),
    asset!("timer-capsule-shortbg.bin", "timer-capsule-shortbg.bin"),
    asset!("card-timer-start.bin", "card-timer-start.bin"),
    asset!("card-timer-play.bin", "card-timer-play.bin"),
    asset!("card-timer-pause.bin", "card-timer-pause.bin"),
    asset!("card-chevron.bin", "card-chevron.bin"),
    asset!("editor-stage-prev.bin", "editor-stage-prev.bin"),
    asset!("editor-stage-next.bin", "editor-stage-next.bin"),
    asset!("editor-wheel-shade.bin", "editor-wheel-shade.bin"),
    asset!("card-switch-on-bg.bin", "card-switch-on-bg.bin"),
    asset!("card-switch-off-bg.bin", "card-switch-off-bg.bin"),
    asset!("card-switch-thumb.bin", "card-switch-thumb.bin"),
    #[cfg(feature = "gallery")]
    asset!("card-status-warning.bin", "card-status-warning.bin"),
    #[cfg(feature = "gallery")]
    asset!("card-status-error.bin", "card-status-error.bin"),
    asset!("card-status-success.bin", "card-status-success.bin"),
    asset!("stop-track-88.bin", "stop-track.bin"),
    asset!("stop-slider-fore.bin", "stop-slider-fore.bin"),
    asset!("exit-slider-fore.bin", "exit-slider-fore.bin"),
    #[cfg(feature = "gallery")]
    asset!("stop-caption-icon.bin", "stop-caption-icon.bin"),
    #[cfg(feature = "gallery")]
    asset!("exit-caption-icon.bin", "exit-caption-icon.bin"),
    asset!("header-button-about.bin", "header-button-about.bin"),
    asset!("header-left.bin", "header-left.bin"),
    asset!("header-right.bin", "header-right.bin"),
    asset!("header-background.bin", "header-background.bin"),
    asset!("card-face.bin", "card-face.bin"),
    asset!("checkbox-minus.bin", "checkbox-minus.bin"),
    asset!("checkbox-check.bin", "checkbox-check.bin"),
    asset!("slider-minus.bin", "slider-minus.bin"),
    asset!("slider-plus.bin", "slider-plus.bin"),
    asset!("capsule-left.bin", "capsule-left.bin"),
    asset!("capsule-center-wide.bin", "capsule-center-wide.bin"),
    asset!("capsule-right.bin", "capsule-right.bin"),
    asset!(
        "capsule-large-neutral-v2-left.bin",
        "capsule-large-left.bin"
    ),
    asset!(
        "capsule-large-neutral-v2-center-wide.bin",
        "capsule-large-center-wide.bin"
    ),
    asset!(
        "capsule-large-neutral-v2-right.bin",
        "capsule-large-right.bin"
    ),
    #[cfg(not(feature = "gallery"))]
    asset!("capsule-primary-left.bin", "capsule-primary-left.bin"),
    #[cfg(not(feature = "gallery"))]
    asset!(
        "capsule-primary-center-wide.bin",
        "capsule-primary-center-wide.bin"
    ),
    #[cfg(not(feature = "gallery"))]
    asset!("capsule-primary-right.bin", "capsule-primary-right.bin"),
    #[cfg(feature = "gallery")]
    asset!(
        "gallery-capsule-primary-left.bin",
        "gallery-capsule-primary-left.bin"
    ),
    #[cfg(feature = "gallery")]
    asset!(
        "gallery-capsule-primary-center-wide.bin",
        "gallery-capsule-primary-center-wide.bin"
    ),
    #[cfg(feature = "gallery")]
    asset!(
        "gallery-capsule-primary-right.bin",
        "gallery-capsule-primary-right.bin"
    ),
    asset!("header-button-black.bin", "header-button-black.bin"),
    asset!("header-button-primary.bin", "header-button-primary.bin"),
    asset!("header-icon-settings.bin", "header-icon-settings.bin"),
    asset!("header-icon-add.bin", "header-icon-add.bin"),
    asset!("header-icon-add-orange.bin", "header-icon-add-orange.bin"),
    asset!("header-icon-back.bin", "header-icon-back.bin"),
    asset!("header-icon-hide.bin", "header-icon-hide.bin"),
    asset!("header-icon-x.bin", "header-icon-x.bin"),
    asset!("header-icon-play.bin", "header-icon-play.bin"),
    asset!("header-icon-pause.bin", "header-icon-pause.bin"),
];

#[cfg(not(target_arch = "wasm32"))]
fn needs_write(ctx: *const CanopusContext, name: &str, data: &[u8]) -> bool {
    let size = packed_assets::decoded_len(data);
    size.is_none() || fs::stat(ctx, name).ok().map(|stat| stat.size as usize) != size
}

#[cfg(not(target_arch = "wasm32"))]
fn unpack(ctx: *const CanopusContext, name: &str, data: &[u8], scratch: &mut [u8]) -> bool {
    // 页面和启动器均在 UI 线程发布，可复用同一个临时文件。
    const STAGING: &str = "asset.unpack";
    let Ok(mut file) = fs::File::open(ctx, STAGING, FS_WRITE | FS_CREATE | FS_TRUNC) else {
        return false;
    };
    let written = packed_assets::decode(data, scratch, |mut chunk| {
        while !chunk.is_empty() {
            let count = file.write(chunk).ok()?;
            if count == 0 {
                return None;
            }
            chunk = chunk.get(count..)?;
        }
        Some(())
    });
    let synced = written.is_some() && file.sync().is_ok();
    drop(file);
    if synced && fs::rename(ctx, STAGING, name).is_ok() {
        return true;
    }
    let _ = fs::remove(ctx, STAGING);
    false
}

#[cfg(not(target_arch = "wasm32"))]
/// 将应用提供的压缩位图解包到当前模块的数据目录。
pub fn ensure_files(entries: &[(&str, &[u8])]) -> bool {
    let ctx = context();
    if ctx.is_null() {
        return false;
    }
    let mut missing = entries
        .iter()
        .copied()
        .filter(|&(name, data)| needs_write(ctx, name, data))
        .peekable();
    if missing.peek().is_none() {
        return true;
    }

    let capacity = packed_assets::BLOCK_SIZE;
    let buffer = mem::alloc(ctx, capacity);
    if buffer.is_null() {
        return false;
    }
    let success = unsafe {
        buffer.write_bytes(0, capacity);
        let scratch = core::slice::from_raw_parts_mut(buffer, capacity);
        missing.all(|(name, data)| unpack(ctx, name, data, scratch))
    };
    mem::free(ctx, buffer);
    success
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure() -> bool {
    ensure_files(ALL)
}

#[cfg(not(target_arch = "wasm32"))]
static FONTS_READY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_fonts() {
    let ready = ensure_files(super::font_assets::ALL);
    FONTS_READY.store(ready, core::sync::atomic::Ordering::Relaxed);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fonts_ready() -> bool {
    FONTS_READY.load(core::sync::atomic::Ordering::Relaxed)
}

#[cfg(target_os = "emscripten")]
pub fn ensure_files(entries: &[(&str, &[u8])]) -> bool {
    let mut scratch = [0; super::packed_assets::BLOCK_SIZE];
    entries.iter().all(|&(name, data)| unsafe {
        crate::devbox::assets::register(name, data, &mut scratch)
    })
}

#[cfg(target_os = "emscripten")]
pub fn ensure() -> bool {
    ensure_files(ALL)
}

#[cfg(target_os = "emscripten")]
pub fn ensure_fonts() {
    crate::devbox::assets::load_fonts();
    let _ = ensure_files(super::font_assets::ALL);
}

#[cfg(target_os = "emscripten")]
pub fn fonts_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{packed_assets, ALL};
    use crate::theme_slots::{Alpha, App, SLOTS};

    #[test]
    fn theme_slots_match_stock_bitmaps() {
        let app = if cfg!(feature = "gallery") {
            App::Gallery
        } else {
            App::Timer
        };
        for (index, slot) in SLOTS.iter().enumerate() {
            assert!(SLOTS[..index]
                .iter()
                .all(|other| other.id != slot.id && other.file != slot.file));
            if !slot.supports(app) {
                continue;
            }
            let &(_, packed) = ALL
                .iter()
                .find(|(name, _)| *name == slot.file)
                .expect(slot.file);
            let mut bitmap = Vec::new();
            packed_assets::decode(packed, &mut [0; packed_assets::BLOCK_SIZE], |chunk| {
                bitmap.extend_from_slice(chunk);
                Some(())
            })
            .expect(slot.file);
            let stride = slot.width * 4;
            assert_eq!(
                bitmap.len(),
                12 + usize::from(stride) * usize::from(slot.height),
                "{}",
                slot.id
            );
            assert_eq!(&bitmap[..4], &[0x19, 0x10, 0, 0], "{}", slot.id);
            assert_eq!(&bitmap[4..6], &slot.width.to_le_bytes(), "{}", slot.id);
            assert_eq!(&bitmap[6..8], &slot.height.to_le_bytes(), "{}", slot.id);
            assert_eq!(&bitmap[8..10], &stride.to_le_bytes(), "{}", slot.id);
            if slot.alpha == Alpha::TransparentBorder {
                for (y, row) in bitmap[12..].chunks_exact(usize::from(stride)).enumerate() {
                    for (x, pixel) in row.chunks_exact(4).enumerate() {
                        if x == 0
                            || y == 0
                            || x + 1 == usize::from(slot.width)
                            || y + 1 == usize::from(slot.height)
                        {
                            assert_eq!(pixel[3], 0, "{} ({x}, {y})", slot.id);
                        }
                    }
                }
            }
        }
    }
}
