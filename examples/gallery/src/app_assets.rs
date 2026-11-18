//! 组件展厅启动器图标。

use aeris_ui::device::assets;

pub const APPICON: &[u8] = b"/data/canopus/m.aeris_gallery/launcher-icon-v2.bin\0";

pub fn ensure_icon() -> bool {
    assets::ensure_files(&[(
        "launcher-icon-v2.bin",
        include_bytes!("../assets/gallery-appicon.bin.lz4"),
    )])
}
