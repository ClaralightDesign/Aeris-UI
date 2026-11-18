use crate::geometry::Rect;

use super::{
    Align, AlignContent, Direction, Distribution, Kind, Measure, Node, Paint, Size, StylePatch,
};

pub const MAX_DEPTH: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidBounds,
    InvalidStyle,
    InvalidMeasurement,
    Capacity,
    Depth,
    CoordinateOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    /// Index in the tree's preorder traversal.
    pub node: usize,
    pub kind: ErrorKind,
}

fn error(node: usize, kind: ErrorKind) -> Error {
    Error { node, kind }
}

/// Caller-owned workspace for one node. Initialize a slice with `LayoutSlot::EMPTY`.
#[derive(Clone, Copy)]
pub struct LayoutSlot {
    style: StylePatch,
    next: usize,
    lane: usize,
    preferred: i64,
    natural: i64,
    basis: i64,
    frozen: bool,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

impl LayoutSlot {
    pub const EMPTY: Self = Self {
        style: StylePatch::EMPTY,
        next: 0,
        lane: 0,
        preferred: 0,
        natural: 0,
        basis: 0,
        frozen: false,
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };

    fn rect(&self) -> Rect {
        Rect::new(self.x as i32, self.y as i32, self.w as i32, self.h as i32)
    }
}

pub struct LayoutPlan<'tree, 'scratch, T> {
    root: &'tree Node<'tree, T>,
    slots: &'scratch [LayoutSlot],
}

impl<T> LayoutPlan<'_, '_, T> {
    /// Bounds are relative to each node's parent, in declaration order (preorder).
    pub fn rects(&self) -> impl ExactSizeIterator<Item = Rect> + '_ {
        self.slots.iter().map(LayoutSlot::rect)
    }

    pub fn root(&self) -> &Node<'_, T> {
        self.root
    }

    pub fn rect(&self, index: usize) -> Option<Rect> {
        self.slots.get(index).map(LayoutSlot::rect)
    }
}

/// Computes the tree using caller-supplied storage and no heap allocation.
///
/// The root receives the supplied containing block. Auto height may extend beyond
/// that block, for example when the block belongs to a scrolling page.
pub fn compute<'tree, 'scratch, T>(
    root: &'tree Node<'tree, T>,
    bounds: Rect,
    measure: &impl Measure<T>,
    scratch: &'scratch mut [LayoutSlot],
) -> Result<LayoutPlan<'tree, 'scratch, T>, Error> {
    if bounds.w < 0 || bounds.h < 0 {
        return Err(error(0, ErrorKind::InvalidBounds));
    }
    let used = initialize(root, measure, scratch, 0, None, 1)?;
    let slots = &mut scratch[..used];
    let style = slots[0].style;
    let available = (i64::from(bounds.w) - style.margin.horizontal()).max(0);
    let width = resolve(style.width, Some(i64::from(bounds.w)))
        .unwrap_or(slots[0].preferred.min(available));
    widths(root, slots, 0, width);
    measure_heights(root, measure, slots, 0)?;
    let height = resolve(style.height, Some(i64::from(bounds.h)));
    slots[0].x = i64::from(bounds.x) + left(style.margin);
    slots[0].y = i64::from(bounds.y) + top(style.margin);
    heights(root, slots, 0, height);
    check_coordinates(slots, 0, 0, 0)?;
    Ok(LayoutPlan { root, slots })
}

fn initialize<T>(
    node: &Node<'_, T>,
    measure: &impl Measure<T>,
    slots: &mut [LayoutSlot],
    index: usize,
    parent: Option<(Kind, u8)>,
    depth: usize,
) -> Result<usize, Error> {
    if depth > MAX_DEPTH {
        return Err(error(index, ErrorKind::Depth));
    }
    let slot = slots
        .get_mut(index)
        .ok_or(error(index, ErrorKind::Capacity))?;
    let style = node.resolved();
    validate(node.kind, style, parent).map_err(|kind| error(index, kind))?;
    *slot = LayoutSlot {
        style,
        ..LayoutSlot::EMPTY
    };

    let mut next = index + 1;
    for child in node.children {
        next = initialize(
            child,
            measure,
            slots,
            next,
            Some((node.kind, columns(style))),
            depth + 1,
        )?;
    }
    slots[index].next = next;
    let size = if let Some(leaf) = &node.leaf {
        measured(index, measure.preferred_width(leaf))?
    } else {
        intrinsic_width(node.kind, slots, index)
    };
    slots[index].preferred = clamp(
        style,
        Axis::X,
        resolve(style.width, None).unwrap_or(size + inset(style, Axis::X)),
    );
    Ok(next)
}

fn validate(kind: Kind, style: StylePatch, parent: Option<(Kind, u8)>) -> Result<(), ErrorKind> {
    let flex = kind == Kind::Flex;
    let grid = kind == Kind::Grid;
    let invalid_flex = !flex
        && (style.direction.is_some()
            || style.wrap.is_some()
            || style.justify.is_some()
            || style.content.is_some());
    let invalid_grid = !grid && (style.columns.is_some() || style.justify_items.is_some());
    let invalid_items = match parent {
        Some((Kind::Flex, _)) => style.span.is_some(),
        Some((Kind::Grid, count)) => {
            style.grow.is_some()
                || style.shrink.is_some()
                || style.basis.is_some()
                || !(1..=count).contains(&style.span.unwrap_or(1))
        }
        _ => {
            style.grow.is_some()
                || style.shrink.is_some()
                || style.basis.is_some()
                || style.span.is_some()
                || style.align_self.is_some()
        }
    };
    let invalid_leaf = kind == Kind::Leaf
        && (style.padding.horizontal() != 0
            || style.padding.vertical() != 0
            || style.gap_x.is_some()
            || style.gap_y.is_some()
            || style.background.is_some()
            || style.border_width.is_some()
            || style.border_color.is_some()
            || style.radius.is_some()
            || style.clip.is_some());
    let invalid_box = kind == Kind::Box && style.items.is_some();
    let invalid_bounds = style.min_width.unwrap_or(0) > style.max_width.unwrap_or(u16::MAX)
        || style.min_height.unwrap_or(0) > style.max_height.unwrap_or(u16::MAX);
    let invalid_paint = [style.background, style.border_color]
        .into_iter()
        .any(|paint| matches!(paint, Some(Paint::Rgb(value)) if value > 0xff_ffff));
    if invalid_flex
        || invalid_grid
        || invalid_items
        || invalid_leaf
        || invalid_box
        || invalid_bounds
        || invalid_paint
        || (kind == Kind::Leaf && style.items.is_some())
        || (grid && !(1..=8).contains(&columns(style)))
        || (style.wrap == Some(true) && style.direction != Some(Direction::Row))
    {
        Err(ErrorKind::InvalidStyle)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

fn resolve(size: Option<Size>, available: Option<i64>) -> Option<i64> {
    match size.unwrap_or_default() {
        Size::Auto => None,
        Size::Full => available,
        Size::Px(px) => Some(i64::from(px)),
    }
}

fn clamp(style: StylePatch, axis: Axis, value: i64) -> i64 {
    let (min, max) = match axis {
        Axis::X => (style.min_width, style.max_width),
        Axis::Y => (style.min_height, style.max_height),
    };
    value
        .max(i64::from(min.unwrap_or(0)))
        .min(max.map_or(i64::MAX, i64::from))
}

fn inset(style: StylePatch, axis: Axis) -> i64 {
    let padding = match axis {
        Axis::X => style.padding.horizontal(),
        Axis::Y => style.padding.vertical(),
    };
    padding + 2 * border(style)
}

fn border(style: StylePatch) -> i64 {
    i64::from(style.border_width.unwrap_or(0))
}

fn left(edges: super::Edges) -> i64 {
    i64::from(edges.left.unwrap_or(0))
}

fn top(edges: super::Edges) -> i64 {
    i64::from(edges.top.unwrap_or(0))
}

fn outer(slot: LayoutSlot, axis: Axis) -> i64 {
    match axis {
        Axis::X => slot.w + slot.style.margin.horizontal(),
        Axis::Y => slot.h + slot.style.margin.vertical(),
    }
}

fn columns(style: StylePatch) -> u8 {
    style.columns.unwrap_or(1)
}

fn measured(index: usize, value: i32) -> Result<i64, Error> {
    if value < 0 {
        Err(error(index, ErrorKind::InvalidMeasurement))
    } else {
        Ok(i64::from(value))
    }
}

struct Siblings {
    next: usize,
    end: usize,
}

impl Siblings {
    fn new(start: usize, end: usize) -> Self {
        Self { next: start, end }
    }

    fn take(&mut self, slots: &[LayoutSlot]) -> Option<usize> {
        if self.next == self.end {
            return None;
        }
        let index = self.next;
        self.next = slots[index].next;
        Some(index)
    }
}

fn intrinsic_width(kind: Kind, slots: &[LayoutSlot], index: usize) -> i64 {
    let style = slots[index].style;
    let gap = i64::from(style.gap_x.unwrap_or(0));
    let mut sum = 0;
    let mut widest = 0;
    let mut count = 0;
    let mut siblings = Siblings::new(index + 1, slots[index].next);
    while let Some(child) = siblings.take(slots) {
        let slot = slots[child];
        let preferred = if style.direction == Some(Direction::Row) {
            clamp(
                slot.style,
                Axis::X,
                resolve(slot.style.basis, None).unwrap_or(slot.preferred),
            )
        } else {
            slot.preferred
        };
        let width = preferred + slot.style.margin.horizontal();
        let per_column = if kind == Kind::Grid {
            let span = i64::from(slot.style.span.unwrap_or(1));
            ((width - (span - 1) * gap).max(0) + span - 1) / span
        } else {
            width
        };
        widest = widest.max(per_column);
        sum += width;
        count += 1;
    }
    match kind {
        Kind::Grid if count > 0 => {
            widest * i64::from(columns(style)) + gap * i64::from(columns(style) - 1)
        }
        Kind::Flex if style.direction == Some(Direction::Row) => sum + gap * (count - 1).max(0),
        _ => widest,
    }
}

fn widths<T>(node: &Node<'_, T>, slots: &mut [LayoutSlot], index: usize, width: i64) {
    let style = slots[index].style;
    let width = clamp(style, Axis::X, width);
    slots[index].w = width;
    let inner = (width - inset(style, Axis::X)).max(0);
    let gap = i64::from(style.gap_x.unwrap_or(0));
    let start = index + 1;
    let end = slots[index].next;

    match node.kind {
        Kind::Grid => grid_widths(slots, start, end, style, inner),
        Kind::Flex if style.direction == Some(Direction::Row) => {
            row_widths(slots, start, end, style, inner)
        }
        _ => {
            let origin = left(style.padding) + border(style);
            let mut siblings = Siblings::new(start, end);
            while let Some(child) = siblings.take(slots) {
                let slot = &mut slots[child];
                let align = slot
                    .style
                    .align_self
                    .or(style.items)
                    .unwrap_or(Align::Stretch);
                align_width(slot, align, origin, inner);
            }
        }
    }
    let mut siblings = Siblings::new(start, end);
    for child in node.children {
        let child_index = siblings.take(slots).expect("initialized child");
        widths(child, slots, child_index, slots[child_index].w);
    }

    if style.direction == Some(Direction::Row) {
        let mut lane = start;
        while lane < end {
            let limit = lane_end(slots, lane, end);
            position_main(
                slots,
                lane,
                limit,
                MainAxis {
                    axis: Axis::X,
                    origin: left(style.padding) + border(style),
                    available: inner,
                    gap,
                    justify: style.justify.unwrap_or(Distribution::Start),
                },
            );
            lane = limit;
        }
    }
}

fn align_width(slot: &mut LayoutSlot, align: Align, origin: i64, width: i64) {
    let style = slot.style;
    let available = (width - style.margin.horizontal()).max(0);
    let desired = resolve(style.width, Some(width)).unwrap_or_else(|| {
        if align == Align::Stretch {
            available
        } else {
            available.min(slot.preferred)
        }
    });
    slot.w = clamp(style, Axis::X, desired);
    slot.x = origin + left(style.margin) + offset(align, width - outer(*slot, Axis::X));
}

fn grid_widths(slots: &mut [LayoutSlot], start: usize, end: usize, style: StylePatch, width: i64) {
    let columns = i64::from(columns(style));
    let gap = i64::from(style.gap_x.unwrap_or(0));
    let track_pixels = (width - gap * (columns - 1)).max(0);
    let prefix =
        |column: i64| column * (track_pixels / columns) + column.min(track_pixels % columns);
    let mut lane = start;
    let mut column = 0;
    let mut siblings = Siblings::new(start, end);
    while let Some(child) = siblings.take(slots) {
        let slot = &mut slots[child];
        let span = i64::from(slot.style.span.unwrap_or(1));
        if column + span > columns {
            column = 0;
            lane = child;
        }
        slot.lane = lane;
        let cell_width = prefix(column + span) - prefix(column) + gap * (span - 1);
        let origin = left(style.padding) + border(style) + prefix(column) + gap * column;
        align_width(
            slot,
            style.justify_items.unwrap_or(Align::Stretch),
            origin,
            cell_width,
        );
        column += span;
    }
}

fn row_widths(slots: &mut [LayoutSlot], start: usize, end: usize, style: StylePatch, width: i64) {
    let gap = i64::from(style.gap_x.unwrap_or(0));
    let mut lane = start;
    let mut used = 0;
    let mut siblings = Siblings::new(start, end);
    while let Some(child) = siblings.take(slots) {
        let slot = &mut slots[child];
        slot.basis = resolve(slot.style.basis, Some(width))
            .or(resolve(slot.style.width, Some(width)))
            .unwrap_or(slot.preferred);
        slot.w = clamp(slot.style, Axis::X, slot.basis);
        let outer_width = outer(*slot, Axis::X);
        if child != lane && style.wrap == Some(true) && used + gap + outer_width > width {
            lane = child;
            used = 0;
        }
        slot.lane = lane;
        used += outer_width + if child == lane { 0 } else { gap };
    }

    let mut lane = start;
    while lane < end {
        let limit = lane_end(slots, lane, end);
        flex_sizes(slots, lane, limit, Axis::X, width, gap);
        lane = limit;
    }
}

fn lane_end(slots: &[LayoutSlot], lane: usize, end: usize) -> usize {
    let mut next = slots[lane].next;
    while next < end && slots[next].lane == lane {
        next = slots[next].next;
    }
    next
}

fn measure_heights<T>(
    node: &Node<'_, T>,
    measure: &impl Measure<T>,
    slots: &mut [LayoutSlot],
    index: usize,
) -> Result<(), Error> {
    let style = slots[index].style;
    let start = index + 1;
    let end = slots[index].next;
    let mut siblings = Siblings::new(start, end);
    for child in node.children {
        let child_index = siblings.take(slots).expect("initialized child");
        measure_heights(child, measure, slots, child_index)?;
    }
    let natural = if let Some(leaf) = &node.leaf {
        let width = i32::try_from(slots[index].w)
            .map_err(|_| error(index, ErrorKind::CoordinateOverflow))?;
        measured(index, measure.height_for_width(leaf, width))?
    } else if uses_lanes(node.kind, style) {
        let mut height = 0;
        let mut lane = start;
        while lane < end {
            let limit = lane_end(slots, lane, end);
            height += lane_height(slots, lane, limit, None);
            if lane != start {
                height += i64::from(style.gap_y.unwrap_or(0));
            }
            lane = limit;
        }
        height
    } else {
        let mut height = 0;
        let mut siblings = Siblings::new(start, end);
        while let Some(child) = siblings.take(slots) {
            let slot = slots[child];
            let height_basis = if node.kind == Kind::Flex {
                resolve(slot.style.basis, None).unwrap_or(slot.natural)
            } else {
                slot.natural
            };
            height += clamp(slot.style, Axis::Y, height_basis) + slot.style.margin.vertical();
            if child != start {
                height += i64::from(style.gap_y.unwrap_or(0));
            }
        }
        height
    };

    slots[index].natural = clamp(
        style,
        Axis::Y,
        resolve(style.height, None).unwrap_or(natural + inset(style, Axis::Y)),
    );
    Ok(())
}

fn uses_lanes(kind: Kind, style: StylePatch) -> bool {
    kind == Kind::Grid || style.direction == Some(Direction::Row)
}

fn lane_height(slots: &[LayoutSlot], start: usize, end: usize, full: Option<i64>) -> i64 {
    let mut height = 0;
    let mut siblings = Siblings::new(start, end);
    while let Some(child) = siblings.take(slots) {
        let slot = slots[child];
        let desired = resolve(slot.style.height, full).unwrap_or(slot.natural);
        height = height.max(clamp(slot.style, Axis::Y, desired) + slot.style.margin.vertical());
    }
    height
}

fn heights<T>(node: &Node<'_, T>, slots: &mut [LayoutSlot], index: usize, height: Option<i64>) {
    let style = slots[index].style;
    slots[index].h = clamp(style, Axis::Y, height.unwrap_or(slots[index].natural));
    let inner = (slots[index].h - inset(style, Axis::Y)).max(0);
    let start = index + 1;
    let end = slots[index].next;
    let origin = top(style.padding) + border(style);
    let gap = i64::from(style.gap_y.unwrap_or(0));
    if uses_lanes(node.kind, style) {
        lane_heights(
            slots,
            start,
            end,
            node.kind,
            style,
            origin,
            inner,
            gap,
            height.is_some(),
        );
    } else {
        let mut siblings = Siblings::new(start, end);
        while let Some(child) = siblings.take(slots) {
            let slot = &mut slots[child];
            let explicit = resolve(slot.style.height, height.map(|_| inner));
            let basis = if node.kind == Kind::Flex {
                resolve(slot.style.basis, height.map(|_| inner)).or(explicit)
            } else {
                explicit
            };
            slot.basis = basis.unwrap_or(slot.natural);
            slot.h = clamp(slot.style, Axis::Y, slot.basis);
        }
        if node.kind == Kind::Flex {
            flex_sizes(slots, start, end, Axis::Y, inner, gap);
        }
        position_main(
            slots,
            start,
            end,
            MainAxis {
                axis: Axis::Y,
                origin,
                available: inner,
                gap,
                justify: style.justify.unwrap_or(Distribution::Start),
            },
        );
    }

    let mut siblings = Siblings::new(start, end);
    for child in node.children {
        let child_index = siblings.take(slots).expect("initialized child");
        let slot = slots[child_index];
        // A height assigned by a parent is definite when explicitly sized or
        // changed by stretch or flex. Natural heights do not resolve Size::Full.
        let child_height = (matches!(slot.style.height, Some(Size::Px(_)))
            || (slot.style.height == Some(Size::Full)
                && (height.is_some() || node.kind == Kind::Grid))
            || slot.h != slot.natural)
            .then_some(slot.h);
        heights(child, slots, child_index, child_height);
    }
}

#[allow(clippy::too_many_arguments)]
fn lane_heights(
    slots: &mut [LayoutSlot],
    start: usize,
    end: usize,
    kind: Kind,
    style: StylePatch,
    origin: i64,
    inner: i64,
    gap: i64,
    definite: bool,
) {
    let full = (kind == Kind::Flex && definite).then_some(inner);
    let mut lanes = 0;
    let mut used = 0;
    let mut lane = start;
    while lane < end {
        let limit = lane_end(slots, lane, end);
        used += lane_height(slots, lane, limit, full);
        lanes += 1;
        lane = limit;
    }
    used += gap * (lanes - 1).max(0);
    let free = (inner - used).max(0);
    let nowrap = kind == Kind::Flex && style.wrap != Some(true);
    let content = style
        .content
        .unwrap_or(AlignContent::Distribute(Distribution::Start));
    let mut position = origin;
    let mut ordinal = 0;
    let mut lane = start;
    while lane < end {
        let limit = lane_end(slots, lane, end);
        let natural = lane_height(slots, lane, limit, full);
        let (lane_size, extra_offset) = if nowrap && definite {
            (inner, 0)
        } else {
            match content {
                AlignContent::Stretch => {
                    let extra = free / lanes + i64::from(ordinal < free % lanes);
                    (natural + extra, 0)
                }
                AlignContent::Distribute(justify) => {
                    (natural, distributed(justify, free, ordinal, lanes))
                }
            }
        };
        let mut siblings = Siblings::new(lane, limit);
        while let Some(child) = siblings.take(slots) {
            let slot = &mut slots[child];
            let align = slot
                .style
                .align_self
                .or(style.items)
                .unwrap_or(Align::Stretch);
            let containing_height = if kind == Kind::Grid {
                Some(lane_size)
            } else {
                full
            };
            let desired = resolve(slot.style.height, containing_height).unwrap_or_else(|| {
                if align == Align::Stretch {
                    (lane_size - slot.style.margin.vertical()).max(0)
                } else {
                    slot.natural
                }
            });
            slot.h = clamp(slot.style, Axis::Y, desired);
            slot.y = position
                + extra_offset
                + top(slot.style.margin)
                + offset(align, lane_size - outer(*slot, Axis::Y));
        }
        position += lane_size + gap;
        ordinal += 1;
        lane = limit;
    }
}

fn offset(align: Align, free: i64) -> i64 {
    match align {
        Align::Center => free.max(0) / 2,
        Align::End => free.max(0),
        _ => 0,
    }
}

fn distributed(justify: Distribution, free: i64, index: i64, count: i64) -> i64 {
    let ratio =
        |part: i64, total: i64| ((i128::from(free) * i128::from(part)) / i128::from(total)) as i64;
    match justify {
        Distribution::Start => 0,
        Distribution::Center => free / 2,
        Distribution::End => free,
        Distribution::Between if count > 1 => ratio(index, count - 1),
        Distribution::Between => 0,
        Distribution::Around => ratio(2 * index + 1, 2 * count),
        Distribution::Evenly => ratio(index + 1, count + 1),
    }
}

struct MainAxis {
    axis: Axis,
    origin: i64,
    available: i64,
    gap: i64,
    justify: Distribution,
}

fn main_usage(slots: &[LayoutSlot], start: usize, end: usize, axis: Axis, gap: i64) -> (i64, i64) {
    let mut count = 0;
    let mut size = 0;
    let mut siblings = Siblings::new(start, end);
    while let Some(child) = siblings.take(slots) {
        size += outer(slots[child], axis);
        count += 1;
    }
    (size + gap * (count - 1).max(0), count)
}

fn position_main(slots: &mut [LayoutSlot], start: usize, end: usize, main: MainAxis) {
    let (used, count) = main_usage(slots, start, end, main.axis, main.gap);
    let free = (main.available - used).max(0);
    let mut position = main.origin;
    let mut ordinal = 0;
    let mut siblings = Siblings::new(start, end);
    while let Some(child) = siblings.take(slots) {
        let slot = &mut slots[child];
        let placed = position + distributed(main.justify, free, ordinal, count);
        match main.axis {
            Axis::X => slot.x = placed + left(slot.style.margin),
            Axis::Y => slot.y = placed + top(slot.style.margin),
        }
        position += outer(*slot, main.axis) + main.gap;
        ordinal += 1;
    }
}

fn flex_sizes(
    slots: &mut [LayoutSlot],
    start: usize,
    end: usize,
    axis: Axis,
    available: i64,
    gap: i64,
) {
    let (used, _) = main_usage(slots, start, end, axis, gap);
    let mut free = available - used;
    let grow = free > 0;
    let weight = |slot: LayoutSlot| {
        if slot.frozen {
            0
        } else if grow {
            i128::from(slot.style.grow.unwrap_or(0))
        } else {
            i128::from(slot.style.shrink.unwrap_or(1)) * i128::from(slot.basis)
        }
    };

    let mut siblings = Siblings::new(start, end);
    while let Some(child) = siblings.take(slots) {
        slots[child].frozen = false;
    }

    while free != 0 {
        let mut total = 0;
        let mut siblings = Siblings::new(start, end);
        while let Some(child) = siblings.take(slots) {
            total += weight(slots[child]);
        }
        if total == 0 {
            break;
        }

        // Freeze all proposals outside their bounds, then recompute the share
        // of the remaining unfrozen items from the same starting dimensions.
        let mut prefix = 0;
        let mut applied = 0;
        let mut froze = false;
        let mut siblings = Siblings::new(start, end);
        while let Some(child) = siblings.take(slots) {
            let slot = &mut slots[child];
            let share = weight(*slot);
            if share == 0 {
                continue;
            }
            let delta = share_delta(free, total, prefix, share);
            prefix += share;
            let old = match axis {
                Axis::X => slot.w,
                Axis::Y => slot.h,
            };
            let target = clamp(slot.style, axis, old + delta);
            if target != old + delta {
                froze = true;
                slot.frozen = true;
                set_extent(slot, axis, target);
                applied += target - old;
            }
        }
        if froze {
            free -= applied;
            continue;
        }

        let mut prefix = 0;
        let mut siblings = Siblings::new(start, end);
        while let Some(child) = siblings.take(slots) {
            let slot = &mut slots[child];
            let share = weight(*slot);
            let delta = share_delta(free, total, prefix, share);
            prefix += share;
            let old = match axis {
                Axis::X => slot.w,
                Axis::Y => slot.h,
            };
            set_extent(slot, axis, old + delta);
        }
        break;
    }
}

fn share_delta(free: i64, total: i128, prefix: i128, weight: i128) -> i64 {
    let free = i128::from(free);
    (((prefix + weight) * free) / total - (prefix * free) / total) as i64
}

fn set_extent(slot: &mut LayoutSlot, axis: Axis, extent: i64) {
    match axis {
        Axis::X => slot.w = extent,
        Axis::Y => slot.h = extent,
    }
}

fn check_coordinates(
    slots: &[LayoutSlot],
    index: usize,
    parent_x: i64,
    parent_y: i64,
) -> Result<(), Error> {
    let slot = slots[index];
    let x = parent_x + slot.x;
    let y = parent_y + slot.y;
    for value in [
        slot.x,
        slot.y,
        slot.w,
        slot.h,
        slot.x + slot.w,
        slot.y + slot.h,
        x,
        y,
        x + slot.w,
        y + slot.h,
    ] {
        i32::try_from(value).map_err(|_| error(index, ErrorKind::CoordinateOverflow))?;
    }
    let mut siblings = Siblings::new(index + 1, slot.next);
    while let Some(child) = siblings.take(slots) {
        check_coordinates(slots, child, x, y)?;
    }
    Ok(())
}
