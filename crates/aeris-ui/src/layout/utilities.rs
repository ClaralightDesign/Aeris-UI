//! Compile-time utility syntax for the same properties exposed by StylePatch.

use super::{Align, AlignContent, Direction, Distribution, Edges, Paint, Radius, Size, StylePatch};
use crate::theme::Color;

#[derive(Clone, Copy)]
pub struct ClassDef {
    name: &'static str,
    style: StylePatch,
}

impl ClassDef {
    pub const fn new(name: &'static str, style: StylePatch) -> Self {
        Self { name, style }
    }
}

/// Compiles Aeris utility classes into a [StylePatch](crate::layout::StylePatch).
/// An optional application class table is supplied before a semicolon.
///
/// ```
/// use aeris_ui::{layout::ClassDef, utilities};
/// const CLASSES: &[ClassDef] = &[ClassDef::new("panel", utilities!("p-3 rounded-xl"))];
/// let style = utilities!(CLASSES; "panel px-4 w-full");
/// assert_eq!(style.padding.left, Some(16));
/// assert_eq!(style.padding.top, Some(12));
/// ```
///
/// ```compile_fail
/// use aeris_ui::utilities;
/// let _ = utilities!("unknown-utility");
/// ```
///
/// ```compile_fail
/// use aeris_ui::utilities;
/// let _ = utilities!("p-[18rem]");
/// ```
///
/// ```compile_fail
/// use aeris_ui::{layout::ClassDef, utilities};
/// const CLASSES: &[ClassDef] = &[
///     ClassDef::new("panel", utilities!("p-2")),
///     ClassDef::new("panel", utilities!("p-3")),
/// ];
/// let _ = utilities!(CLASSES; "panel");
/// ```
#[macro_export]
macro_rules! utilities {
    ($classes:literal) => {
        $crate::utilities!(&[]; $classes)
    };
    ($table:expr; $classes:literal) => {{
        const STYLE: $crate::layout::StylePatch =
            $crate::layout::compile_utilities($table, $classes);
        STYLE
    }};
}

pub const fn compile_utilities(table: &[ClassDef], source: &'static str) -> StylePatch {
    let mut index = 0;
    while index < table.len() {
        let name = table[index].name.as_bytes();
        assert!(!name.is_empty(), "empty Aeris class name");
        let mut offset = 0;
        while offset < name.len() {
            let byte = name[offset];
            assert!(
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-',
                "invalid Aeris class name"
            );
            offset += 1;
        }
        let mut previous = 0;
        while previous < index {
            assert!(
                !same(name, table[previous].name.as_bytes()),
                "duplicate Aeris class name"
            );
            previous += 1;
        }
        index += 1;
    }

    let mut remaining = source.as_bytes();
    let mut result = StylePatch::EMPTY;
    while !remaining.is_empty() {
        let mut end = 0;
        while end < remaining.len() && !remaining[end].is_ascii_whitespace() {
            end += 1;
        }
        let (token, suffix) = remaining.split_at(end);
        if !token.is_empty() {
            let mut entry = 0;
            while entry < table.len() && !same(token, table[entry].name.as_bytes()) {
                entry += 1;
            }
            let patch = if entry < table.len() {
                table[entry].style
            } else {
                builtin(token)
            };
            result = result.overlay(patch);
        }
        remaining = if suffix.is_empty() {
            suffix
        } else {
            suffix.split_at(1).1
        };
    }
    #[cfg(all(feature = "devbox", target_os = "emscripten"))]
    {
        result.utility_source = Some(source);
    }
    result
}

const fn starts(value: &[u8], prefix: &[u8]) -> bool {
    if prefix.len() > value.len() {
        return false;
    }
    let mut index = 0;
    while index < prefix.len() {
        if value[index] != prefix[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn same(value: &[u8], expected: &[u8]) -> bool {
    value.len() == expected.len() && starts(value, expected)
}

const fn number(bytes: &[u8], radix: u32, limit: u32) -> u32 {
    assert!(!bytes.is_empty(), "missing Aeris utility value");
    let mut value = 0;
    let mut index = 0;
    while index < bytes.len() {
        let digit = match bytes[index] {
            b'0'..=b'9' => bytes[index] - b'0',
            b'a'..=b'f' => bytes[index] - b'a' + 10,
            b'A'..=b'F' => bytes[index] - b'A' + 10,
            _ => panic!("invalid Aeris utility number"),
        } as u32;
        assert!(digit < radix, "invalid Aeris utility number");
        value = value * radix + digit;
        assert!(value <= limit, "Aeris utility value out of range");
        index += 1;
    }
    value
}

const fn pixels(bytes: &[u8]) -> u16 {
    match bytes {
        [b'[', digits @ .., b'p', b'x', b']'] => number(digits, 10, u16::MAX as u32) as u16,
        [b'p', b'x'] => 1,
        _ => (number(bytes, 10, u16::MAX as u32 / 4) * 4) as u16,
    }
}

const fn size(bytes: &[u8]) -> Size {
    match bytes {
        b"auto" => Size::Auto,
        b"full" => Size::Full,
        _ => Size::Px(pixels(bytes)),
    }
}

const fn paint(bytes: &[u8]) -> Paint {
    match bytes {
        b"transparent" => Paint::Transparent,
        b"background" => Paint::Theme(Color::Background),
        b"accent" => Paint::Theme(Color::Accent),
        b"primary-text" => Paint::Theme(Color::PrimaryText),
        b"secondary-text" => Paint::Theme(Color::SecondaryText),
        [b'[', b'#', digits @ .., b']'] if digits.len() == 6 => {
            Paint::Rgb(number(digits, 16, 0xff_ffff))
        }
        _ => panic!("invalid Aeris utility color"),
    }
}

const fn align(bytes: &[u8]) -> Align {
    match bytes {
        b"start" => Align::Start,
        b"center" => Align::Center,
        b"end" => Align::End,
        b"stretch" => Align::Stretch,
        _ => panic!("invalid Aeris item alignment"),
    }
}

const fn distribute(bytes: &[u8]) -> Distribution {
    match bytes {
        b"start" => Distribution::Start,
        b"center" => Distribution::Center,
        b"end" => Distribution::End,
        b"between" => Distribution::Between,
        b"around" => Distribution::Around,
        b"evenly" => Distribution::Evenly,
        _ => panic!("invalid Aeris space distribution"),
    }
}

const fn spacing(side: u8, px: u16) -> Edges {
    let px = Some(px);
    match side {
        b'x' => Edges {
            left: px,
            right: px,
            ..Edges::EMPTY
        },
        b'y' => Edges {
            top: px,
            bottom: px,
            ..Edges::EMPTY
        },
        b't' => Edges {
            top: px,
            ..Edges::EMPTY
        },
        b'b' => Edges {
            bottom: px,
            ..Edges::EMPTY
        },
        b'l' => Edges {
            left: px,
            ..Edges::EMPTY
        },
        b'r' => Edges {
            right: px,
            ..Edges::EMPTY
        },
        _ => panic!("invalid Aeris spacing edge"),
    }
}

const fn builtin(token: &[u8]) -> StylePatch {
    let mut style = StylePatch::EMPTY;
    match token {
        [b'w', b'-', value @ ..] => style.width = Some(size(value)),
        [b'h', b'-', value @ ..] => style.height = Some(size(value)),
        [b'm', b'i', b'n', b'-', b'w', b'-', value @ ..] => style.min_width = Some(pixels(value)),
        [b'm', b'i', b'n', b'-', b'h', b'-', value @ ..] => style.min_height = Some(pixels(value)),
        [b'm', b'a', b'x', b'-', b'w', b'-', value @ ..] => style.max_width = Some(pixels(value)),
        [b'm', b'a', b'x', b'-', b'h', b'-', value @ ..] => style.max_height = Some(pixels(value)),
        [b'p', b'-', value @ ..] => style.padding = Edges::all(pixels(value)),
        [b'm', b'-', value @ ..] => style.margin = Edges::all(pixels(value)),
        [b'p', side, b'-', value @ ..] => style.padding = spacing(*side, pixels(value)),
        [b'm', side, b'-', value @ ..] => style.margin = spacing(*side, pixels(value)),
        b"flex-row" => style.direction = Some(Direction::Row),
        b"flex-col" => style.direction = Some(Direction::Column),
        b"flex-wrap" => style.wrap = Some(true),
        b"flex-nowrap" => style.wrap = Some(false),
        b"flex-1" => {
            style.grow = Some(1);
            style.shrink = Some(1);
            style.basis = Some(Size::Px(0));
        }
        b"grow" => style.grow = Some(1),
        b"shrink" => style.shrink = Some(1),
        b"border" => style.border_width = Some(1),
        b"rounded" => style.radius = Some(Radius::Px(8)),
        b"overflow-hidden" => style.clip = Some(true),
        b"overflow-visible" => style.clip = Some(false),
        _ => return prefixed(token),
    }
    style
}

// Match longer prefixes first where one utility family includes another.
const fn prefixed(token: &[u8]) -> StylePatch {
    let mut style = StylePatch::EMPTY;
    if starts(token, b"gap-x-") {
        style.gap_x = Some(pixels(token.split_at(6).1));
    } else if starts(token, b"gap-y-") {
        style.gap_y = Some(pixels(token.split_at(6).1));
    } else if starts(token, b"gap-") {
        let gap = pixels(token.split_at(4).1);
        style.gap_x = Some(gap);
        style.gap_y = Some(gap);
    } else if starts(token, b"justify-items-") {
        style.justify_items = Some(align(token.split_at(14).1));
    } else if starts(token, b"justify-") {
        style.justify = Some(distribute(token.split_at(8).1));
    } else if starts(token, b"items-") {
        style.items = Some(align(token.split_at(6).1));
    } else if starts(token, b"self-") {
        style.align_self = Some(align(token.split_at(5).1));
    } else if starts(token, b"content-") {
        let value = token.split_at(8).1;
        style.content = Some(match value {
            b"stretch" => AlignContent::Stretch,
            _ => AlignContent::Distribute(distribute(value)),
        });
    } else if starts(token, b"grow-") {
        style.grow = Some(number(token.split_at(5).1, 10, u16::MAX as u32) as u16);
    } else if starts(token, b"shrink-") {
        style.shrink = Some(number(token.split_at(7).1, 10, u16::MAX as u32) as u16);
    } else if starts(token, b"basis-") {
        style.basis = Some(size(token.split_at(6).1));
    } else if starts(token, b"grid-cols-") {
        style.columns = Some(columns(token.split_at(10).1));
    } else if starts(token, b"col-span-") {
        style.span = Some(columns(token.split_at(9).1));
    } else if starts(token, b"bg-") {
        style.background = Some(paint(token.split_at(3).1));
    } else if starts(token, b"border-") {
        match token.split_at(7).1 {
            [b'0'..=b'9', ..] => {
                style.border_width = Some(number(token.split_at(7).1, 10, u16::MAX as u32) as u16)
            }
            value => style.border_color = Some(paint(value)),
        }
    } else if starts(token, b"rounded-") {
        style.radius = Some(match token.split_at(8).1 {
            b"none" => Radius::Px(0),
            b"sm" => Radius::Px(6),
            b"md" => Radius::Px(12),
            b"lg" => Radius::Px(16),
            b"xl" => Radius::Px(24),
            b"2xl" => Radius::Px(32),
            b"full" => Radius::Full,
            value => Radius::Px(pixels(value)),
        });
    } else {
        panic!("unknown Aeris utility");
    }
    style
}

const fn columns(value: &[u8]) -> u8 {
    let count = number(value, 10, 8) as u8;
    assert!(count > 0, "Aeris column count must be positive");
    count
}
