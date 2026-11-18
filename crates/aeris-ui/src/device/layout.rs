//! Mounts a computed layout using the same LVGL entry points as existing components.

use canopus_target_private::*;

use crate::{
    geometry::Rect,
    layout::{Kind, LayoutPlan, Node, Paint, Radius},
    theme::Color,
};

use super::lv::{self, Obj};

const ALIGN_TOP_LEFT: u32 = 1;
const PADDING_PROPERTIES: [u32; 4] = [16, 17, 18, 19];
const LAYOUT: u32 = 22;
const BORDER_WIDTH: u32 = 48;
const BORDER_COLOR: u32 = 49;
const BORDER_OPACITY: u32 = 50;
const BORDER_SIDE: u32 = 52;
const BORDER_FULL: u32 = 0x0f;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MountError {
    InvalidParent,
    Create { node: usize },
}

/// Creates the layout and returns its root, owned by the supplied parent.
/// The root rectangle uses the parent's LVGL content origin. Nested leaf
/// rectangles are already adjusted for the layout parent's LVGL content origin.
///
/// # Safety
/// Call on the UI thread with a live parent. The callback must attach its
/// objects to the supplied parent, return a live leaf root on success, and
/// clean up its own partial objects on failure. Do not delete or reparent
/// existing objects from the callback.
pub unsafe fn mount<T>(
    parent: Obj,
    plan: &LayoutPlan<'_, '_, T>,
    mut create_leaf: impl FnMut(Obj, Rect, &T) -> Option<Obj>,
) -> Result<Obj, MountError> {
    if parent.is_null() {
        return Err(MountError::InvalidParent);
    }
    unsafe {
        mount_node(
            parent,
            plan.root(),
            &mut plan.rects().enumerate(),
            (0, 0),
            &mut create_leaf,
        )
    }
}

unsafe fn mount_node<T>(
    parent: Obj,
    node: &Node<'_, T>,
    rects: &mut impl Iterator<Item = (usize, Rect)>,
    parent_inset: (i32, i32),
    create_leaf: &mut impl FnMut(Obj, Rect, &T) -> Option<Obj>,
) -> Result<Obj, MountError> {
    let (index, mut rect) = rects.next().expect("computed plan matches the tree");
    // Native alignment starts at the parent's content edge. The pure layout
    // already includes the border and padding in its child rectangles.
    rect.x -= parent_inset.0;
    rect.y -= parent_inset.1;
    if let Some(leaf) = &node.leaf {
        let object = create_leaf(parent, rect, leaf)
            .filter(|object| !object.is_null())
            .ok_or(MountError::Create { node: index })?;
        inspect_utilities(object, node);
        return Ok(object);
    }

    let style = node.resolved();
    let background = style.background.unwrap_or(Paint::Transparent);
    let border = style.border_width.unwrap_or(0);
    let radius = match style.radius.unwrap_or(Radius::Px(0)) {
        Radius::Px(px) => u32::from(px),
        Radius::Full => (rect.w.min(rect.h) / 2) as u32,
    };
    let object = unsafe { lv::background(parent, rect.w, rect.h, rgb(background), radius) };
    if object.is_null() {
        return Err(MountError::Create { node: index });
    }
    inspect_utilities(object, node);
    let name = match node.kind {
        Kind::Box => c"AerisUI.Box",
        Kind::Flex => c"AerisUI.Flex",
        Kind::Grid => c"AerisUI.Grid",
        Kind::Leaf => unreachable!(),
    };
    let border_color = style
        .border_color
        .unwrap_or(Paint::Theme(Color::PrimaryText));
    let clip = style.clip == Some(true);

    let padding = style.padding;
    let edges = [padding.top, padding.bottom, padding.left, padding.right];
    for (property, px) in PADDING_PROPERTIES.into_iter().zip(edges) {
        lv::style(object, property, u32::from(px.unwrap_or(0)));
    }
    let child_inset = (
        i32::from(border) + i32::from(padding.left.unwrap_or(0)),
        i32::from(border) + i32::from(padding.top.unwrap_or(0)),
    );
    lv::style(object, LAYOUT, 0);
    lv::style(object, lv::BG_OPA, opacity(background));
    lv::style(object, BORDER_SIDE, BORDER_FULL);
    lv::style(object, BORDER_WIDTH, u32::from(border));
    lv::style(object, BORDER_COLOR, rgb(border_color));
    lv::style(
        object,
        BORDER_OPACITY,
        if border == 0 {
            0
        } else {
            opacity(border_color)
        },
    );
    lv::style(object, LV_STYLE_CLIP_CORNER, u32::from(clip && radius != 0));
    unsafe {
        lv::inspect_name(object, name);
        lvx_object_align(object, ALIGN_TOP_LEFT, rect.x, rect.y);
        lvx_object_clear_flag(object, lv::CLICKABLE | lv::SCROLLABLE);
        // A passive container must let drags over its controls reach the page.
        lvx_object_add_flag(object, lv::GESTURE_BUBBLE | lv::SCROLL_CHAIN);
        if clip {
            lvx_object_clear_flag(object, lv::OVERFLOW_VISIBLE);
        } else {
            lvx_object_add_flag(object, lv::OVERFLOW_VISIBLE);
        }

        for child in node.children {
            if let Err(error) = mount_node(object, child, rects, child_inset, create_leaf) {
                lvx_object_delete(object);
                return Err(error);
            }
        }
    }
    Ok(object)
}

fn rgb(paint: Paint) -> u32 {
    match paint {
        Paint::Transparent => 0,
        Paint::Rgb(value) => value,
        Paint::Theme(color) => color.resolve(),
    }
}

fn opacity(paint: Paint) -> u32 {
    if matches!(paint, Paint::Transparent) {
        0
    } else {
        255
    }
}

fn inspect_utilities<T>(_object: Obj, _node: &Node<'_, T>) {
    #[cfg(target_os = "emscripten")]
    {
        extern "C" {
            fn gallery_add_utility_classes(object: Obj, source: *const u8, len: usize, first: u32);
        }
        for (index, source) in _node.utility_sources.iter().enumerate() {
            unsafe {
                gallery_add_utility_classes(
                    _object,
                    source.as_ptr(),
                    source.len(),
                    u32::from(index == 0),
                );
            }
        }
    }
}
