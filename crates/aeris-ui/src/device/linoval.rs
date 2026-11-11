//! 以固件原生标签 ABI 渲染 Linoval Sans 的静态字形。

use core::ptr;

use super::lv::{self, Font, Obj};

const TEXT_FONT: u32 = 90;
const REPERTOIRE_LEN: usize = 100;
const GLYPH_BYTES: usize = 7;
const BITMAP_OFFSET: usize = 2 + REPERTOIRE_LEN * GLYPH_BYTES;

struct FontData {
    bytes: &'static [u8],
    glyphs: &'static [u8],
}

impl FontData {
    fn index(&self, letter: u32) -> Option<usize> {
        if !self.glyphs.is_empty() {
            return self.glyphs.binary_search(&u8::try_from(letter).ok()?).ok();
        }
        match letter {
            9 => Some(0),
            32..=126 => Some(letter as usize - 32),
            0xA0 => Some(95),
            0xB0 => Some(96),
            0xB7 => Some(97),
            0x2013 => Some(98),
            0x2212 => Some(99),
            _ => None,
        }
    }

    fn bitmap_offset(&self) -> usize {
        if self.glyphs.is_empty() {
            BITMAP_OFFSET
        } else {
            2 + self.glyphs.len() * GLYPH_BYTES
        }
    }
}

#[repr(C)]
struct NativeFont {
    describe: unsafe extern "C" fn(*const Self, *mut Glyph, u32, u32) -> bool,
    bitmap: unsafe extern "C" fn(*const Glyph, *mut DrawBuffer) -> *mut DrawBuffer,
    release: Option<unsafe extern "C" fn(*const Self, *mut Glyph)>,
    line_height: i32,
    baseline: i32,
    flags: u8,
    underline_position: i8,
    underline_thickness: i8,
    data: &'static FontData,
    fallback: Option<&'static Self>,
    user_data: usize,
}

#[repr(C)]
struct Glyph {
    font: *const NativeFont,
    advance: u16,
    width: u16,
    height: u16,
    x: i16,
    y: i16,
    format: u8,
    flags: u8,
    bitmap_offset: u32,
    cache_entry: usize,
}

#[repr(C)]
struct DrawBuffer {
    format: u32,
    dimensions: u32,
    stride: u16,
    reserved: u16,
    size: u32,
    data: *mut u8,
    unaligned_data: *mut u8,
    handlers: *const DrawBufferHandlers,
}

#[repr(C)]
struct DrawBufferHandlers {
    memory_ops: [usize; 4],
    flush_cache: Option<unsafe extern "C" fn(*const DrawBuffer, *const Area)>,
}

#[repr(C)]
struct Area {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

#[cfg(target_pointer_width = "32")]
const _: () = {
    assert!(core::mem::size_of::<NativeFont>() == 36);
    assert!(core::mem::offset_of!(NativeFont, data) == 24);
    assert!(core::mem::size_of::<Glyph>() == 24);
    assert!(core::mem::offset_of!(Glyph, bitmap_offset) == 16);
    assert!(core::mem::offset_of!(DrawBuffer, data) == 16);
    assert!(core::mem::offset_of!(DrawBuffer, handlers) == 24);
    assert!(core::mem::offset_of!(DrawBufferHandlers, flush_cache) == 16);
};

// 字体和资源仅由固件读取，始终保存在模块的静态内存中。
macro_rules! font {
    ($name:literal, $glyphs:expr) => {
        NativeFont::new(&FontData {
            bytes: include_bytes!(concat!("../../assets/linoval-", $name, ".font")),
            glyphs: $glyphs,
        })
    };
}

static REGULAR_24: NativeFont = font!("regular-24", b"");
static REGULAR_25: NativeFont = font!("regular-25", b"");
static REGULAR_26: NativeFont = font!("regular-26", b"");
static MEDIUM_24: NativeFont = font!("medium-24", b"");
static MEDIUM_25: NativeFont = font!("medium-25", b"");
static MEDIUM_26: NativeFont = font!("medium-26", b"");
static MEDIUM_28: NativeFont = font!("medium-28", b"");
static MEDIUM_30: NativeFont = font!("medium-30", b"");
static MEDIUM_32: NativeFont = font!("medium-32", b"");
static DIGITS_40: NativeFont = font!("digits-40", b"0123456789:");
static ROUNDED_44: NativeFont = font!("rounded-44", b" 'TUeimps");

impl NativeFont {
    const fn new(data: &'static FontData) -> Self {
        Self {
            describe,
            bitmap,
            release: None,
            line_height: data.bytes[0] as i32,
            baseline: data.bytes[1] as i32,
            flags: 0,
            underline_position: -1,
            underline_thickness: 1,
            data,
            fallback: None,
            user_data: 0,
        }
    }
}

pub(super) fn apply(label: Obj, font: Font, px: i32) -> i32 {
    let native = match (font, px) {
        (Font::Small24, 25) => &REGULAR_25,
        (Font::Small24, 26) => &REGULAR_26,
        (Font::Small24, _) => &REGULAR_24,
        (Font::Medium24, 25) => &MEDIUM_25,
        (Font::Medium24, 26) => &MEDIUM_26,
        (Font::Medium24, _) => &MEDIUM_24,
        (Font::Medium28, 30) => &MEDIUM_30,
        (Font::Medium28, _) => &MEDIUM_28,
        (Font::Demibold32, 40) => &DIGITS_40,
        (Font::Demibold32, _) => &MEDIUM_32,
        (Font::Rounded44, _) => &ROUNDED_44,
    };
    lv::style(
        label,
        TEXT_FONT,
        native as *const NativeFont as usize as u32,
    );
    native.line_height
}

unsafe extern "C" fn describe(
    font: *const NativeFont,
    glyph: *mut Glyph,
    letter: u32,
    _next: u32,
) -> bool {
    let font_data = unsafe { (*font).data };
    let Some(index) = font_data.index(letter) else {
        return false;
    };
    unsafe {
        let data = font_data.bytes;
        let offset = 2 + index * GLYPH_BYTES;
        let metrics = &data[offset..offset + GLYPH_BYTES];
        *glyph = Glyph {
            font,
            advance: u16::from(metrics[2]) * if letter == 9 { 2 } else { 1 },
            width: u16::from(metrics[3]),
            height: u16::from(metrics[4]),
            x: i16::from(metrics[5] as i8),
            y: i16::from(metrics[6] as i8),
            format: 8,
            flags: 0,
            bitmap_offset: u32::from(u16::from_le_bytes([metrics[0], metrics[1]])),
            cache_entry: 0,
        };
    }
    true
}

// 将字形逐行写入固件的 A8 缓冲区，再同步给绘制引擎。
unsafe extern "C" fn bitmap(glyph: *const Glyph, buffer: *mut DrawBuffer) -> *mut DrawBuffer {
    let glyph = unsafe { &*glyph };
    let Some(output) = (unsafe { buffer.as_ref() }) else {
        return ptr::null_mut();
    };
    let width = usize::from(glyph.width);
    let height = usize::from(glyph.height);
    let stride = usize::from(output.stride);
    let end = stride * height;
    if width == 0
        || height == 0
        || stride < width
        || end > output.size as usize
        || output.data.is_null()
    {
        return ptr::null_mut();
    }
    unsafe {
        let font = (*glyph.font).data;
        let offset = font.bitmap_offset() + glyph.bitmap_offset as usize;
        let source = &font.bytes[offset..offset + width * height];
        let target = core::slice::from_raw_parts_mut(output.data, end);
        for (row, alpha) in target
            .chunks_exact_mut(stride)
            .zip(source.chunks_exact(width))
        {
            row[..width].copy_from_slice(alpha);
        }
        if let Some(flush) = output
            .handlers
            .as_ref()
            .and_then(|handlers| handlers.flush_cache)
        {
            let area = Area {
                x1: 0,
                y1: 0,
                x2: glyph.width as i32 - 1,
                y2: glyph.height as i32 - 1,
            };
            flush(buffer, &area);
        }
    }
    buffer
}
