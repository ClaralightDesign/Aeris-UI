//! Aeris 默认色值及设备主题覆盖。

use core::sync::atomic::{AtomicU32, Ordering};

/// 应用按钮外观。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ButtonTheme {
    Glass,
    Normal,
}

static BUTTON_THEME: AtomicU32 = AtomicU32::new(ButtonTheme::Glass as u32);

impl Default for ButtonTheme {
    fn default() -> Self {
        match BUTTON_THEME.load(Ordering::Relaxed) {
            value if value == Self::Normal as u32 => Self::Normal,
            _ => Self::Glass,
        }
    }
}

/// 在创建组件前应用按钮外观和语义色。缺省色值沿用组件原色。
pub fn apply(button_theme: ButtonTheme, colors: [Option<Rgb>; Color::ALL.len()]) {
    BUTTON_THEME.store(button_theme as u32, Ordering::Relaxed);
    set_colors(colors);
}

/// 供 LVGL 绘制使用的基础 RGB 色值。
pub struct Theme {
    /// 设计稿画布底色。
    pub canvas: u32,
    /// 标题栏及设备页面底色。
    pub backdrop: u32,
    /// 主文字色。
    pub text_primary: u32,
    /// 标题和激活状态的强调色。
    pub accent: u32,
    /// 实底添加按钮的底色。
    pub accent_fill: u32,
    /// 实底添加按钮的图标色。
    pub accent_icon: u32,
    /// 深底设置、返回及关闭按钮的图标色。
    pub glass_icon: u32,
}

pub const QLING: Theme = Theme {
    canvas: 0x1A1A1A,
    backdrop: 0x000000,
    text_primary: 0xFFFFFF,
    accent: 0xFF8400,
    accent_fill: 0xFF9D00,
    accent_icon: 0xFFEAD7,
    glass_icon: 0xDDDDDD,
};

const GALLERY_ACCENT: u32 = 0x0088FF;
const GALLERY_ACCENT_FOREGROUND: u32 = 0xD7EEFF;

/// 页码指示点的不透明度。
pub const DOT_INACTIVE_OPACITY: u8 = 102;
pub const DOT_ACTIVE_OPACITY: u8 = 250;

/// 深底按钮的高光和描边不透明度。
pub const GLASS_HIGHLIGHT_OPACITY: u8 = 46;
pub const GLASS_BORDER_OPACITY: u8 = 41;

impl Theme {
    pub const fn accent_with_opacity(self, opacity: u8) -> u32 {
        // LVGL 单独设置不透明度。
        let _ = opacity;
        self.accent
    }
}

/// 设备主题允许覆盖的语义色。
#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum Color {
    Background,
    PrimaryText,
    SecondaryText,
    Accent,
}

impl Color {
    pub const ALL: [Self; 4] = [
        Self::Background,
        Self::PrimaryText,
        Self::SecondaryText,
        Self::Accent,
    ];

    pub const fn default_rgb(self) -> u32 {
        match self {
            Self::Background => QLING.backdrop,
            Self::PrimaryText => QLING.text_primary,
            Self::SecondaryText => 0xAAAAAA,
            Self::Accent if cfg!(feature = "gallery") => GALLERY_ACCENT,
            Self::Accent => QLING.accent,
        }
    }

    pub fn resolve(self) -> u32 {
        self.resolve_or(self.default_rgb())
    }

    pub fn resolve_or(self, fallback: u32) -> u32 {
        self.override_rgb().unwrap_or(fallback)
    }

    fn override_rgb(self) -> Option<u32> {
        let rgb = ACTIVE[self as usize].load(Ordering::Relaxed);
        (rgb != UNSET).then_some(rgb)
    }
}

pub struct AccentButton {
    pub fill: u32,
    pub foreground: u32,
}

impl AccentButton {
    /// 根据原图的明暗对 BGRA 底图染色，保留透明通道。
    pub fn tint_pixels(&self, pixels: &mut [[u8; 4]], stock_fill: [u8; 3]) {
        let reference = luma(stock_fill);
        let [_, red, green, blue] = self.fill.to_be_bytes();
        let fill = [red, green, blue];
        let lightness = luma(fill);
        // 纯白和纯黑也为高光、暗部留出层次。
        let anchor = lightness.clamp(24 * 256, 231 * 256);
        let [red, green, blue] = fill.map(|channel| shade(channel, lightness, anchor));
        for [b, g, r, alpha] in pixels {
            if *alpha == 0 {
                continue;
            }
            let tone = luma([*r, *g, *b]);
            *b = shade(blue, reference, tone);
            *g = shade(green, reference, tone);
            *r = shade(red, reference, tone);
        }
    }
}

fn luma([red, green, blue]: [u8; 3]) -> u32 {
    54 * u32::from(red) + 183 * u32::from(green) + 19 * u32::from(blue)
}

fn shade(channel: u8, reference: u32, tone: u32) -> u8 {
    const MAX: u32 = 255 * 256;
    let channel = u32::from(channel);
    if tone < reference {
        ((channel * tone + reference / 2) / reference) as u8
    } else if tone > reference {
        let range = MAX - reference;
        (channel + ((255 - channel) * (tone - reference) + range / 2) / range) as u8
    } else {
        channel as u8
    }
}

pub fn accent_button() -> Option<AccentButton> {
    Some(button_colors(Color::Accent.override_rgb()?))
}

pub fn primary_icon_button() -> Option<AccentButton> {
    #[cfg(target_arch = "wasm32")]
    return Some(button_colors(Color::Accent.resolve()));

    #[cfg(not(target_arch = "wasm32"))]
    accent_button()
}

pub fn primary_button_foreground() -> u32 {
    accent_button().map_or(
        if cfg!(feature = "gallery") {
            GALLERY_ACCENT_FOREGROUND
        } else {
            QLING.accent_icon
        },
        |button| button.foreground,
    )
}

fn button_colors(fill: u32) -> AccentButton {
    let [_, red, green, blue] = fill.to_be_bytes();
    let luminance = 0.2126 * linear_channel(red)
        + 0.7152 * linear_channel(green)
        + 0.0722 * linear_channel(blue);
    let foreground = match fill {
        GALLERY_ACCENT => GALLERY_ACCENT_FOREGROUND,
        fill if fill == QLING.accent || fill == QLING.accent_fill => QLING.accent_icon,
        _ if luminance > 0.65 => 0x000000,
        _ => {
            // 在白色中混入少量强调色，保留同色系的浅色前景。
            let [red, green, blue] = [red, green, blue]
                .map(|channel| 255 - ((255 - u32::from(channel)) * 40 + 127) / 255);
            (red << 16) | (green << 8) | blue
        }
    };
    AccentButton { fill, foreground }
}

// 用三次多项式近似 sRGB 线性化，区分需要深色前景的高明度底色。
fn linear_channel(channel: u8) -> f32 {
    let value = f32::from(channel) / 255.0;
    value * (value * (0.305306 * value + 0.682171) + 0.012523)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(u32);

impl Rgb {
    pub fn parse(hex: &[u8]) -> Option<Self> {
        let [b'#', digits @ ..] = hex else {
            return None;
        };
        if digits.len() != 6 {
            return None;
        }
        let mut rgb = 0;
        for &digit in digits {
            let value = match digit {
                b'0'..=b'9' => digit - b'0',
                b'A'..=b'F' => digit - b'A' + 10,
                b'a'..=b'f' => digit - b'a' + 10,
                _ => return None,
            };
            rgb = (rgb << 4) | u32::from(value);
        }
        Some(Self(rgb))
    }
}

const UNSET: u32 = 0x1000000;
static ACTIVE: [AtomicU32; Color::ALL.len()] = [const { AtomicU32::new(UNSET) }; Color::ALL.len()];

/// 在 UI 线程重建页面之前应用整套覆盖值，缺省项沿用组件原色。
pub fn set_colors(colors: [Option<Rgb>; Color::ALL.len()]) {
    for (slot, color) in ACTIVE.iter().zip(colors) {
        slot.store(color.map_or(UNSET, |rgb| rgb.0), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_partial_colors_and_restores_original_shades() {
        set_colors([None, Some(Rgb(0)), Some(Rgb(0x123456)), None]);
        assert_eq!(Color::Background.resolve(), QLING.backdrop);
        assert_eq!(Color::PrimaryText.resolve(), 0);
        assert_eq!(Color::SecondaryText.resolve_or(0xDDDDDD), 0x123456);
        let stock_accent = if cfg!(feature = "gallery") {
            0x0088FF
        } else {
            0xFF8400
        };
        assert_eq!(Color::Accent.resolve(), stock_accent);
        assert!(accent_button().is_none());

        for (fill, foreground) in [
            (0x0088FF, 0xD7EEFF),
            (0xFF8400, 0xFFEAD7),
            (0x1685EF, 0xDAECFC),
            (0x12258C, 0xDADDED),
            (0x5EE7FE, 0x000000),
        ] {
            set_colors([None, None, None, Some(Rgb(fill))]);
            let button = accent_button().unwrap();
            assert_eq!(button.fill, fill);
            assert_eq!(button.foreground, foreground);
            assert_eq!(primary_button_foreground(), foreground);
        }

        set_colors([None; Color::ALL.len()]);
        assert!(accent_button().is_none());
        assert_eq!(Color::Accent.resolve(), stock_accent);
        assert_eq!(
            primary_button_foreground(),
            if cfg!(feature = "gallery") {
                0xD7EEFF
            } else {
                0xFFEAD7
            }
        );
        assert_eq!(Color::PrimaryText.resolve(), QLING.text_primary);
        assert_eq!(Color::SecondaryText.resolve(), 0xAAAAAA);
        assert_eq!(Color::SecondaryText.resolve_or(0xDDDDDD), 0xDDDDDD);
    }

    #[test]
    fn button_tint_keeps_highlights_shadows_and_alpha() {
        let stock = [212, 111, 2];
        let source = [
            [2, 111, 212, 255],
            [32, 141, 242, 192],
            [0, 81, 182, 128],
            [1, 2, 3, 0],
        ];
        for fill in [0x5EE7FE, 0x12258C, 0x000000, 0xFFFFFF] {
            let button = AccentButton {
                fill,
                foreground: 0,
            };
            let mut pixels = source;
            button.tint_pixels(&mut pixels, stock);
            let [base, highlight, shadow, transparent] = pixels;
            let tone = |[blue, green, red, _]: [u8; 4]| luma([red, green, blue]);
            assert!(tone(highlight) > tone(base) && tone(base) > tone(shadow));
            assert_eq!(pixels.map(|pixel| pixel[3]), source.map(|pixel| pixel[3]));
            assert_eq!(transparent, source[3]);
        }
        let mut base = [source[0]];
        AccentButton {
            fill: 0x5EE7FE,
            foreground: 0,
        }
        .tint_pixels(&mut base, stock);
        assert_eq!(base[0], [0xFE, 0xE7, 0x5E, 255]);
    }

    #[test]
    fn parses_only_six_digit_rgb() {
        assert_eq!(Rgb::parse(b"#0AbF19"), Some(Rgb(0x0ABF19)));
        assert_eq!(Rgb::parse(b"#000000"), Some(Rgb(0)));
        assert_eq!(Rgb::parse(b"#FFFFFF"), Some(Rgb(0xFFFFFF)));
        for value in [
            b"#123".as_slice(),
            b"123456",
            b"#12345678",
            b"#12345g",
            b"#12345 ",
        ] {
            assert_eq!(Rgb::parse(value), None);
        }
    }
}
