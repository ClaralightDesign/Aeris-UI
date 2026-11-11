//! Bitmap assets exported from the QLingClock Figma file.
//!
//! These are the source of truth for the header backdrop and the action button
//! faces. The LVGL renderer must blit these images and must not redraw the
//! glass gradients in code. Buttons are 68×68 transparent PNGs with the icon
//! already baked in and no shadow; the button shadows live in the left/right
//! 168×94 half-strips.

/// All Figma exports are authored at 2× and downsampled to logical pixels by
/// the renderer. The watch panel itself never scales, so 2× is the maximum
/// useful source resolution.
pub const ASSET_SCALE: i32 = 2;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BitmapAsset {
    /// Logical name inside the module's private asset directory.
    pub name: &'static str,
    /// Source PNG exported from Figma and embedded in the module.
    pub file: &'static str,
    /// Logical size on the 336×94 header.
    pub width: i32,
    pub height: i32,
}

/// 168×94 transparent half-strip with the pure gradient and no button shadow.
pub const HEADER_BACKGROUND: BitmapAsset = BitmapAsset {
    name: "purebg",
    file: "purebg.png",
    width: 168,
    height: 94,
};

/// 168×94 transparent left half-strip: gradient + left button shadow.
pub const HEADER_SHADOW_LEFT: BitmapAsset = BitmapAsset {
    name: "leftshadow",
    file: "leftshadow.png",
    width: 168,
    height: 94,
};

/// 168×94 transparent right half-strip: gradient + right button shadow.
pub const HEADER_SHADOW_RIGHT: BitmapAsset = BitmapAsset {
    name: "rightshadow",
    file: "rightshadow.png",
    width: 168,
    height: 94,
};

pub const BUTTON_SETTINGS: BitmapAsset = BitmapAsset {
    name: "btn-settings",
    file: "header-button-settings.png",
    width: 68,
    height: 68,
};

pub const BUTTON_ADD: BitmapAsset = BitmapAsset {
    name: "btn-add",
    file: "header-button-add-orange.png",
    width: 68,
    height: 68,
};

pub const BUTTON_BACK: BitmapAsset = BitmapAsset {
    name: "btn-back",
    file: "header-button-back.png",
    width: 68,
    height: 68,
};

pub const BUTTON_CLOSE: BitmapAsset = BitmapAsset {
    name: "btn-close",
    file: "header-button-x.png",
    width: 68,
    height: 68,
};


/// 38×38 transparent white-alpha corner (76×76 exported at 2×).
pub const NINE_SLICE_LEFT_TOP: BitmapAsset = BitmapAsset {
    name: "lefttop",
    file: "lefttop.png",
    width: 38,
    height: 38,
};

pub const NINE_SLICE_RIGHT_TOP: BitmapAsset = BitmapAsset {
    name: "righttop",
    file: "righttop.png",
    width: 38,
    height: 38,
};

pub const NINE_SLICE_LEFT_BOTTOM: BitmapAsset = BitmapAsset {
    name: "leftbottom",
    file: "leftbottom.png",
    width: 38,
    height: 38,
};

pub const NINE_SLICE_RIGHT_BOTTOM: BitmapAsset = BitmapAsset {
    name: "rightbottom",
    file: "rightbottom.png",
    width: 38,
    height: 38,
};

/// 38×38 uniform white-alpha centre tile used for all four edges and the middle.
pub const NINE_SLICE_CENTER: BitmapAsset = BitmapAsset {
    name: "center",
    file: "center.png",
    width: 38,
    height: 38,
};

pub const NINE_SLICE_ASSETS: [BitmapAsset; 5] = [
    NINE_SLICE_LEFT_TOP,
    NINE_SLICE_RIGHT_TOP,
    NINE_SLICE_LEFT_BOTTOM,
    NINE_SLICE_RIGHT_BOTTOM,
    NINE_SLICE_CENTER,
];


const fn capsule(name: &'static str, file: &'static str, height: i32) -> BitmapAsset {
    BitmapAsset {
        name,
        file,
        width: 80,
        height,
    }
}

pub const CAPSULE_LEFT: BitmapAsset = capsule("capsule-left", "capsule-left.png", 70);
pub const CAPSULE_CENTER: BitmapAsset = capsule("capsule-center", "capsule-center.png", 70);
pub const CAPSULE_RIGHT: BitmapAsset = capsule("capsule-right", "capsule-right.png", 70);

pub const CAPSULE_LARGE_LEFT: BitmapAsset =
    capsule("capsule-large-left", "capsule-large-left.png", 82);
pub const CAPSULE_LARGE_CENTER: BitmapAsset =
    capsule("capsule-large-center", "capsule-large-center.png", 82);
pub const CAPSULE_LARGE_RIGHT: BitmapAsset =
    capsule("capsule-large-right", "capsule-large-right.png", 82);

pub const CAPSULE_LARGE_PRIMARY_LEFT: BitmapAsset =
    capsule("capsule-large-primary-left", "capsule-large-primary-left.png", 82);
pub const CAPSULE_LARGE_PRIMARY_CENTER: BitmapAsset =
    capsule("capsule-large-primary-center", "capsule-large-primary-center.png", 82);
pub const CAPSULE_LARGE_PRIMARY_RIGHT: BitmapAsset =
    capsule("capsule-large-primary-right", "capsule-large-primary-right.png", 82);

pub const CAPSULE_ASSETS: [BitmapAsset; 9] = [
    CAPSULE_LEFT,
    CAPSULE_CENTER,
    CAPSULE_RIGHT,
    CAPSULE_LARGE_LEFT,
    CAPSULE_LARGE_CENTER,
    CAPSULE_LARGE_RIGHT,
    CAPSULE_LARGE_PRIMARY_LEFT,
    CAPSULE_LARGE_PRIMARY_CENTER,
    CAPSULE_LARGE_PRIMARY_RIGHT,
];

pub const ALL: [BitmapAsset; 21] = [
    HEADER_BACKGROUND,
    HEADER_SHADOW_LEFT,
    HEADER_SHADOW_RIGHT,
    BUTTON_SETTINGS,
    BUTTON_ADD,
    BUTTON_BACK,
    BUTTON_CLOSE,
    NINE_SLICE_LEFT_TOP,
    NINE_SLICE_RIGHT_TOP,
    NINE_SLICE_LEFT_BOTTOM,
    NINE_SLICE_RIGHT_BOTTOM,
    NINE_SLICE_CENTER,
    CAPSULE_LEFT,
    CAPSULE_CENTER,
    CAPSULE_RIGHT,
    CAPSULE_LARGE_LEFT,
    CAPSULE_LARGE_CENTER,
    CAPSULE_LARGE_RIGHT,
    CAPSULE_LARGE_PRIMARY_LEFT,
    CAPSULE_LARGE_PRIMARY_CENTER,
    CAPSULE_LARGE_PRIMARY_RIGHT,
];
