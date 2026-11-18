//! Strongly typed layout and paint overrides.

use crate::theme::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    #[default]
    Auto,
    Px(u16),
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distribution {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignContent {
    Distribute(Distribution),
    Stretch,
}

#[derive(Clone, Copy, Debug)]
pub enum Paint {
    Transparent,
    Rgb(u32),
    Theme(Color),
}

#[derive(Clone, Copy, Debug)]
pub enum Radius {
    Px(u16),
    Full,
}

/// Unset fields preserve earlier styles, including individual inset edges.
#[derive(Clone, Copy, Debug, Default)]
pub struct Edges {
    pub top: Option<u16>,
    pub right: Option<u16>,
    pub bottom: Option<u16>,
    pub left: Option<u16>,
}

impl Edges {
    pub const EMPTY: Self = Self {
        top: None,
        right: None,
        bottom: None,
        left: None,
    };

    pub const fn all(px: u16) -> Self {
        Self {
            top: Some(px),
            right: Some(px),
            bottom: Some(px),
            left: Some(px),
        }
    }

    pub(super) const fn overlay(self, other: Self) -> Self {
        Self {
            top: choose(self.top, other.top),
            right: choose(self.right, other.right),
            bottom: choose(self.bottom, other.bottom),
            left: choose(self.left, other.left),
        }
    }

    pub(super) fn horizontal(self) -> i64 {
        i64::from(self.left.unwrap_or(0)) + i64::from(self.right.unwrap_or(0))
    }

    pub(super) fn vertical(self) -> i64 {
        i64::from(self.top.unwrap_or(0)) + i64::from(self.bottom.unwrap_or(0))
    }
}

const fn choose<T: Copy>(old: Option<T>, new: Option<T>) -> Option<T> {
    match new {
        Some(_) => new,
        None => old,
    }
}

macro_rules! style_patch {
    ($($field:ident: $ty:ty),* $(,)?) => {
        /// A set of overrides. Use `..StylePatch::EMPTY` to set only the required fields.
        #[derive(Clone, Copy, Debug, Default)]
        pub struct StylePatch {
            pub padding: Edges,
            pub margin: Edges,
            #[cfg(all(feature = "devbox", target_os = "emscripten"))]
            #[doc(hidden)]
            pub utility_source: Option<&'static str>,
            $(pub $field: Option<$ty>,)*
        }

        impl StylePatch {
            pub const EMPTY: Self = Self {
                padding: Edges::EMPTY,
                margin: Edges::EMPTY,
                #[cfg(all(feature = "devbox", target_os = "emscripten"))]
                utility_source: None,
                $($field: None,)*
            };

            /// Overlays the supplied properties on this patch.
            pub const fn overlay(self, other: Self) -> Self {
                Self {
                    padding: self.padding.overlay(other.padding),
                    margin: self.margin.overlay(other.margin),
                    #[cfg(all(feature = "devbox", target_os = "emscripten"))]
                    utility_source: choose(self.utility_source, other.utility_source),
                    $($field: choose(self.$field, other.$field),)*
                }
            }
        }
    };
}

style_patch! {
    width: Size,
    height: Size,
    min_width: u16,
    min_height: u16,
    max_width: u16,
    max_height: u16,
    gap_x: u16,
    gap_y: u16,
    direction: Direction,
    wrap: bool,
    items: Align,
    justify: Distribution,
    content: AlignContent,
    align_self: Align,
    grow: u16,
    shrink: u16,
    basis: Size,
    columns: u8,
    span: u8,
    justify_items: Align,
    background: Paint,
    border_color: Paint,
    border_width: u16,
    radius: Radius,
    clip: bool,
}
