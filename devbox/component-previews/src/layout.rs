use std::{cell::Cell, ffi::CStr};

use aeris_ui::{
    device::{cards, layout as renderer, lv::Obj, page, text_block},
    geometry::{Rect, Viewport},
    layout::{self, Box as LayoutBox, ClassDef, Flex, Grid, LayoutSlot, Leaf, Measure},
    lvx_event_get_user_data, lvx_label_set_text,
    theme::Color,
    utilities,
};

thread_local! {
    static SELECTION: Cell<Obj> = const { Cell::new(core::ptr::null_mut()) };
}

enum Item {
    Title,
    Selection,
    Button(&'static CStr, page::ButtonStyle),
    Note,
}

struct Metrics;
const NOTE: &str = "选择一个配置来更新手表设置。更改只会应用到当前预览。";

impl Measure<Item> for Metrics {
    fn preferred_width(&self, item: &Item) -> i32 {
        match item {
            Item::Button(_, _) => 132,
            _ => 0,
        }
    }

    fn height_for_width(&self, item: &Item, width: i32) -> i32 {
        match item {
            Item::Title => 40,
            Item::Selection => 32,
            Item::Button(_, style) => style.height(),
            Item::Note => {
                let columns = ((width - 36) / 26).max(1) as usize;
                68 + 34 * NOTE.chars().count().div_ceil(columns) as i32
            }
        }
    }
}

extern "C" fn select(event: Obj) {
    let title = unsafe { lvx_event_get_user_data(event) } as *const u8;
    SELECTION.with(|label| {
        if !label.get().is_null() && !title.is_null() {
            unsafe { lvx_label_set_text(label.get(), title) };
        }
    });
}

fn button(title: &'static CStr, style: page::ButtonStyle) -> layout::Node<'static, Item> {
    Leaf::new(Item::Button(title, style))
}

pub(super) fn settings(root: Obj, viewport: Viewport) -> Option<i32> {
    const CARD: &[ClassDef] = &[ClassDef::new(
        "settings-card",
        utilities!("p-3 rounded-[20px] bg-[#242428] border border-[#45454A] overflow-hidden"),
    )];

    let bounds = viewport.content();
    let columns = if bounds.w >= 300 { 2 } else { 1 };
    let scroll = unsafe { page::scroll_in(root, bounds) };
    if scroll.is_null() {
        return None;
    }

    let tags = [
        button(c"Daily", page::ButtonStyle::Regular).class(utilities!("w-[120px]")),
        button(c"Workout", page::ButtonStyle::Regular).class(utilities!("w-[144px]")),
        button(c"Sleep", page::ButtonStyle::Regular).class(utilities!("w-[120px]")),
    ];
    let note = Leaf::new(Item::Note);
    let tiles = [
        button(c"Display", page::ButtonStyle::Secondary),
        button(c"Sound", page::ButtonStyle::Secondary),
        button(c"Focus", page::ButtonStyle::Secondary),
        button(c"System", page::ButtonStyle::Secondary),
        if columns == 2 { note.class(utilities!("col-span-2")) } else { note },
    ];
    let children = [
        Leaf::new(Item::Title),
        Leaf::new(Item::Selection),
        Flex::row(&tags).class(utilities!("w-full flex-wrap gap-2 items-center")),
        Grid::columns(columns, &tiles).class(utilities!("w-full gap-2")),
    ];
    let nodes = [LayoutBox::children(&children)
        .class(utilities!(CARD; "settings-card w-full"))
        .class(utilities!("gap-y-3"))];
    let content = LayoutBox::children(&nodes).class(utilities!("w-full p-2"));

    let mut scratch = [LayoutSlot::EMPTY; 16];
    let local = Rect::new(0, 0, bounds.w, bounds.h);
    let plan = layout::compute(&content, local, &Metrics, &mut scratch).ok()?;
    SELECTION.with(|label| label.set(core::ptr::null_mut()));
    let mounted = unsafe {
        renderer::mount(scroll, &plan, |parent, area, item| {
            let object = match item {
                Item::Title => cards::text_font(
                    parent,
                    "Settings",
                    area.x,
                    area.y,
                    area.w,
                    area.h,
                    Color::PrimaryText.resolve(),
                    aeris_ui::device::lv::Font::Demibold32,
                    32,
                ),
                Item::Selection => {
                    let label = cards::text(
                        parent,
                        "Choose a profile",
                        area.x,
                        area.y,
                        area.w,
                        area.h,
                        Color::Accent.resolve(),
                        false,
                    );
                    SELECTION.with(|slot| slot.set(label));
                    label
                }
                Item::Button(title, style) => page::button_with_options(
                    parent,
                    area,
                    std::str::from_utf8(title.to_bytes_with_nul()).ok()?,
                    select,
                    title.as_ptr() as usize,
                    page::ButtonOptions {
                        style: *style,
                        ..Default::default()
                    },
                ),
                Item::Note => {
                    return text_block::create(
                        parent,
                        area,
                        "Quick tip",
                        NOTE,
                        text_block::TextBlockOptions::default(),
                    )
                }
            };
            Some(object)
        })
    };
    if mounted.is_err() {
        SELECTION.with(|label| label.set(core::ptr::null_mut()));
        return None;
    }
    Some(viewport.bounds().h)
}

pub(super) fn empty(root: Obj, viewport: Viewport) -> Option<i32> {
    let empty = [
        LayoutBox::children(&[]).class(utilities!("h-8 rounded-md bg-accent")),
        Flex::row(&[]).class(utilities!("h-8 rounded-md bg-[#45454A]")),
        Grid::columns(2, &[]).class(utilities!("h-8 rounded-md bg-[#45454A]")),
        LayoutBox::children(&[]),
    ];
    let container = Flex::column(&empty).class(utilities!("w-full p-3 gap-2"));
    let mut scratch = [LayoutSlot::EMPTY; 5];
    let plan = layout::compute(
        &container,
        viewport.content(),
        &Metrics,
        &mut scratch,
    )
    .ok()?;
    unsafe { renderer::mount(root, &plan, |_, _, _| None) }.ok()?;
    plan.rect(0).map(|area| area.bottom())
}
