//! Device-independent layout for Aeris containers and existing components.

mod compute;
mod style;
mod utilities;

pub use compute::{compute, Error, ErrorKind, LayoutPlan, LayoutSlot, MAX_DEPTH};
pub use style::{
    Align, AlignContent, Direction, Distribution, Edges, Paint, Radius, Size, StylePatch,
};
#[doc(hidden)]
pub use utilities::compile_utilities;
pub use utilities::ClassDef;

/// A leaf measures its preferred width independently of height, then reports the
/// height it needs at the assigned width, in device pixels.
pub trait Measure<T> {
    fn preferred_width(&self, leaf: &T) -> i32;
    fn height_for_width(&self, leaf: &T, width: i32) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Box,
    Flex,
    Grid,
    Leaf,
}

pub struct Node<'a, T> {
    pub(crate) kind: Kind,
    pub(crate) children: &'a [Self],
    pub(crate) leaf: Option<T>,
    class: StylePatch,
    style: StylePatch,
    #[cfg(all(feature = "devbox", target_os = "emscripten"))]
    pub(crate) utility_sources: alloc::vec::Vec<&'static str>,
}

impl<'a, T> Node<'a, T> {
    const fn container(kind: Kind, children: &'a [Self], defaults: StylePatch) -> Self {
        Self {
            kind,
            children,
            leaf: None,
            style: StylePatch::EMPTY,
            #[cfg(all(feature = "devbox", target_os = "emscripten"))]
            utility_sources: alloc::vec::Vec::new(),
            class: defaults,
        }
    }

    /// Adds classes in declaration order. Explicit styles take precedence.
    pub fn class(mut self, patch: StylePatch) -> Self {
        #[cfg(all(feature = "devbox", target_os = "emscripten"))]
        if let Some(source) = patch.utility_source {
            self.utility_sources.push(source);
        }
        self.class = self.class.overlay(patch);
        self
    }

    /// Applies explicit properties, independently of where classes were added.
    pub const fn style(mut self, patch: StylePatch) -> Self {
        self.style = self.style.overlay(patch);
        self
    }

    pub(crate) const fn resolved(&self) -> StylePatch {
        self.class.overlay(self.style)
    }
}

pub struct Box;

impl Box {
    /// Arranges children vertically and stretches their automatic widths.
    pub const fn children<'a, T>(children: &'a [Node<'a, T>]) -> Node<'a, T> {
        Node::container(Kind::Box, children, StylePatch::EMPTY)
    }
}

pub struct Flex;

impl Flex {
    pub const fn row<'a, T>(children: &'a [Node<'a, T>]) -> Node<'a, T> {
        Self::direction(Direction::Row, children)
    }

    pub const fn column<'a, T>(children: &'a [Node<'a, T>]) -> Node<'a, T> {
        Self::direction(Direction::Column, children)
    }

    const fn direction<'a, T>(direction: Direction, children: &'a [Node<'a, T>]) -> Node<'a, T> {
        Node::container(
            Kind::Flex,
            children,
            StylePatch {
                direction: Some(direction),
                ..StylePatch::EMPTY
            },
        )
    }
}

pub struct Grid;

impl Grid {
    /// Uses 1–8 equal-width columns and automatic rows.
    pub const fn columns<'a, T>(columns: u8, children: &'a [Node<'a, T>]) -> Node<'a, T> {
        Node::container(
            Kind::Grid,
            children,
            StylePatch {
                columns: Some(columns),
                ..StylePatch::EMPTY
            },
        )
    }
}

pub struct Leaf;

impl Leaf {
    #[allow(clippy::new_ret_no_self)]
    pub const fn new<'a, T>(payload: T) -> Node<'a, T> {
        Node {
            kind: Kind::Leaf,
            children: &[],
            leaf: Some(payload),
            style: StylePatch::EMPTY,
            #[cfg(all(feature = "devbox", target_os = "emscripten"))]
            utility_sources: alloc::vec::Vec::new(),
            class: StylePatch::EMPTY,
        }
    }
}

#[cfg(test)]
mod tests;
