//! 主题素材白名单与基础主题版本。

pub const SCHEMA_VERSION: u32 = 1;
pub const BASE_THEME: &str = "aeris";
pub const BASE_THEME_VERSION: u32 = 1;
pub const FIRMWARE: &str = "3.101.043";
pub const VIEWPORT: (u16, u16) = (336, 480);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum App {
    Timer,
    Gallery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Both,
    Timer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alpha {
    Any,
    TransparentBorder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub id: &'static str,
    pub file: &'static str,
    pub width: u16,
    pub height: u16,
    pub scope: Scope,
    pub alpha: Alpha,
}

impl Slot {
    pub fn supports(self, app: App) -> bool {
        self.scope == Scope::Both || app == App::Timer
    }
}

macro_rules! slot {
    ($id:literal, $width:literal, $height:literal, $scope:ident, $alpha:ident) => {
        Slot {
            id: $id,
            file: concat!($id, ".bin"),
            width: $width,
            height: $height,
            scope: Scope::$scope,
            alpha: Alpha::$alpha,
        }
    };
}

/// 缺省素材通过同名运行时文件从应用内置资源表读取。
pub const SLOTS: &[Slot] = &[
    slot!("card-face", 328, 98, Both, Any),
    slot!("timer-capsule-shortbg", 152, 112, Both, Any),
    slot!("header-background", 168, 94, Both, Any),
    slot!("header-left", 168, 94, Both, Any),
    slot!("header-right", 168, 94, Timer, Any),
    slot!("header-button-black", 68, 68, Both, Any),
    slot!("header-button-primary", 68, 68, Timer, Any),
    slot!("header-icon-back", 40, 40, Both, TransparentBorder),
    slot!("header-icon-settings", 40, 40, Timer, TransparentBorder),
    slot!("header-icon-add", 40, 40, Timer, TransparentBorder),
    slot!("header-icon-add-orange", 40, 40, Timer, TransparentBorder),
    slot!("header-icon-hide", 40, 40, Timer, TransparentBorder),
    slot!("header-icon-x", 40, 40, Timer, TransparentBorder),
    slot!("header-icon-play", 40, 40, Timer, TransparentBorder),
    slot!("header-icon-pause", 40, 40, Timer, TransparentBorder),
];
