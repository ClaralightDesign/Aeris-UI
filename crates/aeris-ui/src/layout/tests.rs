use super::*;
use crate::{geometry::Rect, utilities};

struct Metrics;

impl Measure<(i32, i32)> for Metrics {
    fn preferred_width(&self, leaf: &(i32, i32)) -> i32 {
        leaf.0
    }

    fn height_for_width(&self, leaf: &(i32, i32), width: i32) -> i32 {
        let width = width.max(1);
        leaf.1 * ((leaf.0 + width - 1) / width).max(1)
    }
}

fn rects(root: &Node<'_, (i32, i32)>, width: i32) -> Vec<Rect> {
    let mut slots = [LayoutSlot::EMPTY; 16];
    compute(root, Rect::new(0, 0, width, 480), &Metrics, &mut slots)
        .unwrap()
        .rects()
        .collect()
}

#[test]
fn grid_spans_keep_source_order_on_a_narrow_safe_area() {
    let nodes = [
        Leaf::new((10, 20)),
        Leaf::new((10, 25)).class(utilities!("col-span-2")),
        Leaf::new((10, 20)),
        Leaf::new((10, 20)),
    ];
    let root = Grid::columns(2, &nodes).class(utilities!("gap-2 w-full"));
    let mut slots = [LayoutSlot::EMPTY; 5];
    let plan = compute(&root, Rect::new(36, 20, 239, 400), &Metrics, &mut slots).unwrap();
    assert_eq!(
        plan.rects().collect::<Vec<_>>(),
        [
            Rect::new(36, 20, 239, 81),
            Rect::new(0, 0, 116, 20),
            Rect::new(0, 28, 239, 25),
            Rect::new(0, 61, 116, 20),
            Rect::new(124, 61, 115, 20),
        ]
    );
}

#[test]
fn wrap_uses_hypothetical_sizes_and_respects_the_one_pixel_boundary() {
    let nodes = [Leaf::new((100, 20)), Leaf::new((100, 20))];
    let root = Flex::row(&nodes).class(utilities!("w-full gap-2 flex-wrap"));
    assert_eq!(rects(&root, 208)[2], Rect::new(108, 0, 100, 20));
    assert_eq!(rects(&root, 207)[2], Rect::new(0, 28, 100, 20));
    assert_eq!(rects(&root, 80)[2], Rect::new(0, 48, 80, 40));
}

#[test]
fn flex_redistributes_after_clamping_and_measures_after_width_assignment() {
    let children = [
        Leaf::new((100, 20)).class(utilities!("grow max-w-[118px] min-w-[90px]")),
        Leaf::new((100, 20)).class(utilities!("grow")),
    ];
    let root = Flex::row(&children).class(utilities!("w-full items-start"));
    let grown = rects(&root, 260);
    assert_eq!(grown[1].w, 118);
    assert_eq!(grown[2].w, 142);
    let shrunk = rects(&root, 150);
    assert_eq!(shrunk[1], Rect::new(0, 0, 90, 40));
    assert_eq!(shrunk[2], Rect::new(90, 0, 60, 40));
}

#[test]
fn multiple_lines_and_children_align_independently() {
    let children = [
        Leaf::new((60, 20)),
        Leaf::new((40, 10)).class(utilities!("self-end")),
        Leaf::new((60, 20)),
    ];
    let root = Flex::row(&children).class(utilities!(
        "w-full h-[100px] gap-y-2 flex-wrap justify-center content-between items-start"
    ));
    let positions = rects(&root, 110);
    assert_eq!(positions[1], Rect::new(5, 0, 60, 20));
    assert_eq!(positions[2], Rect::new(65, 10, 40, 10));
    assert_eq!(positions[3], Rect::new(25, 80, 60, 20));
}

#[test]
fn wrapped_rows_account_for_full_height_items_in_a_definite_container() {
    let children = [
        Leaf::new((80, 20)).class(utilities!("h-full")),
        Leaf::new((80, 20)),
    ];
    let root = Flex::row(&children).class(utilities!("w-full h-[100px] flex-wrap gap-y-2"));
    let positions = rects(&root, 100);
    assert_eq!(positions[1], Rect::new(0, 0, 80, 100));
    assert_eq!(positions[2], Rect::new(0, 108, 80, 20));
}

#[test]
fn column_flex_allocates_height_and_box_auto_height_handles_unresolved_full() {
    let children = [
        Leaf::new((20, 40)).class(utilities!("grow max-h-[60px]")),
        Leaf::new((20, 40)).class(utilities!("grow")),
    ];
    let root = Flex::column(&children).class(utilities!("w-full h-full"));
    let positions = rects(&root, 100);
    assert_eq!(positions[1], Rect::new(0, 0, 100, 60));
    assert_eq!(positions[2], Rect::new(0, 60, 100, 420));
    let children = [Leaf::new((20, 40)).class(utilities!("h-full"))];
    let root = Box::children(&children).class(utilities!("w-full px-3 py-2 border"));
    let positions = rects(&root, 100);
    assert_eq!(positions[0].h, 58);
    assert_eq!(positions[1], Rect::new(13, 9, 74, 40));
}

#[test]
fn classes_resolve_in_order_and_explicit_styles_override_even_later_classes() {
    const CLASSES: &[ClassDef] = &[ClassDef::new(
        "settings-card",
        utilities!("p-3 m-2 rounded-[20px] bg-[#242428]"),
    )];
    let style = utilities!(CLASSES; "settings-card px-4 ml-[7px] gap-2 gap-x-3");
    assert_eq!(style.padding.top, Some(12));
    assert_eq!(style.padding.left, Some(16));
    assert_eq!(style.margin.left, Some(7));
    assert_eq!(style.margin.top, Some(8));
    assert_eq!(style.gap_x, Some(12));
    assert_eq!(style.gap_y, Some(8));
    assert!(matches!(style.background, Some(Paint::Rgb(0x242428))));
    let leaf = Leaf::new((10, 10))
        .style(StylePatch {
            width: Some(Size::Px(30)),
            ..StylePatch::EMPTY
        })
        .class(utilities!("w-full"));
    assert_eq!(rects(&leaf, 100)[0].w, 30);
}

#[test]
fn invalid_geometry_and_styles_fail_before_mount() {
    let leaf = Leaf::new((10, 10));
    let input = Rect::new(0, 0, 100, 100);
    let mut slots = [LayoutSlot::EMPTY; 16];
    let kind = |result: Result<LayoutPlan<'_, '_, (i32, i32)>, Error>| result.err().unwrap();
    assert_eq!(
        kind(compute(&leaf, input, &Metrics, &mut [])).kind,
        ErrorKind::Capacity
    );
    let bad = Flex::column(&[]).class(utilities!("flex-wrap"));
    assert_eq!(
        kind(compute(&bad, input, &Metrics, &mut slots)).kind,
        ErrorKind::InvalidStyle
    );
    let nodes = [Leaf::new((10, 10)).class(utilities!("col-span-3"))];
    let grid = Grid::columns(2, &nodes);
    assert_eq!(kind(compute(&grid, input, &Metrics, &mut slots)).node, 1);
    let overflow = Rect::new(i32::MAX, 0, 100, 100);
    assert_eq!(
        kind(compute(&leaf, overflow, &Metrics, &mut slots)).kind,
        ErrorKind::CoordinateOverflow
    );
    let level8 = [leaf];
    let level7 = [Box::children(&level8)];
    let level6 = [Box::children(&level7)];
    let level5 = [Box::children(&level6)];
    let level4 = [Box::children(&level5)];
    let level3 = [Box::children(&level4)];
    let level2 = [Box::children(&level3)];
    let level1 = [Box::children(&level2)];
    assert_eq!(
        kind(compute(
            &Box::children(&level1),
            input,
            &Metrics,
            &mut slots
        ))
        .kind,
        ErrorKind::Depth
    );
}
