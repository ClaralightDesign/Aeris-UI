#![cfg(target_os = "emscripten")]

mod layout;

use std::cell::RefCell;

use aeris_ui::{
    device::{
        assets, cards, confirmation_overlay as overlay, dialog, round_button,
        header::{HeaderBar, HeaderOptions, HeaderStyle},
        lv::{self, Obj},
        numeric_wheel::{Column, NumericWheel, NumericWheelStyle},
        page,
        progress_ring::{CircularProgressIndicator, ProgressRingOptions, ProgressRingStyle},
        text_block::{self, TextBlockOptions, TextBlockStyle},
    },
    gallery_set_crop_height,
    geometry::{Rect, Viewport},
    header::HEADER_HEIGHT,
    lvx_event_add, lvx_event_get_user_data, lvx_object_align, lvx_object_scroll_y,
    progress_ring::STEPS,
    theme,
};

thread_local! {
    static HEADER: RefCell<Option<HeaderBar>> = const { RefCell::new(None) };
    static CONFIRMATION: RefCell<Option<(Obj, overlay::Style, overlay::Options<'static>)>> = const { RefCell::new(None) };
    static WHEEL: RefCell<Option<Box<dyn Fn(Obj)>>> = const { RefCell::new(None) };
    static DIALOG: RefCell<Option<dialog::Layers>> = const { RefCell::new(None) };
}

#[unsafe(no_mangle)]
pub extern "C" fn aeris_devbox_mount() -> i32 {
    theme::set_colors([None, None, None, theme::Rgb::parse(b"#0088FF")]);
    unsafe { aeris_ui::devbox::mount(mount) }
}

extern "C" fn noop(_: Obj) {}

extern "C" fn on_scroll(event: Obj) {
    let scroll = unsafe { lvx_event_get_user_data(event) as Obj };
    HEADER.with_borrow(|header| {
        if let Some(header) = header {
            header.update_scroll(unsafe { lvx_object_scroll_y(scroll) });
        }
    });
}

fn header(root: Obj, viewport: Viewport, variant: &str, large_title_layout: Option<&str>) -> Option<i32> {
    let style = match variant {
        "bottom" => HeaderStyle::Bottom,
        "left" => HeaderStyle::Left,
        "right" => HeaderStyle::Right,
        "simple" => HeaderStyle::Simple,
        "center-simple" => HeaderStyle::CenterSimple,
        "header-button" => HeaderStyle::HeaderButton,
        "footer-button" => HeaderStyle::FooterButton,
        "top" => HeaderStyle::Top,
        "large-title" => match large_title_layout {
            Some("left") => HeaderStyle::Left,
            Some("right") => HeaderStyle::Right,
            _ => HeaderStyle::Top,
        },
        _ => return None,
    };
    let large_title = variant == "large-title";
    let footer = style == HeaderStyle::Bottom;
    let bounds = viewport.content();
    let scroll = unsafe { page::scroll_in(root, bounds) };
    if scroll.is_null() {
        return None;
    }
    let header = unsafe {
        HeaderBar::new_for_viewport(
            root,
            viewport,
            "Settings",
            Some(scroll),
            HeaderOptions {
                style,
                large_title,
                on_left: Some(noop),
                on_right: Some(noop),
            },
        )
    }?;

    let top_inset = if footer { 8 } else { HEADER_HEIGHT };
    if let Some(area) = header.large_title_area() {
        // The full-screen scroll viewport sits behind the fixed bar's gradient.
        unsafe { lvx_object_align(area, 1, 0, top_inset) };
    }
    let mut y = top_inset + header.content_top();
    for (title, detail) in [
        ("Notifications", "Allowed apps"),
        ("Display", "Brightness"),
        ("Sound", "Volume and alerts"),
        ("Privacy", "Permissions"),
        ("Language", "English"),
        ("System", "About this device"),
    ] {
        let (card, _) = unsafe {
            cards::jump_card_in(
                scroll,
                Rect::new(4, y, bounds.w - 8, 98),
                title,
                detail,
                cards::JumpCardOptions::default(),
            )
        };
        if card.is_null() {
            return None;
        }
        y += 106;
    }
    if footer {
        // Keep the final row clear of the footer when scrolled all the way down.
        unsafe { cards::container(scroll, 0, y, bounds.w, HEADER_HEIGHT) };
    }
    if large_title {
        HEADER.with_borrow_mut(|saved| *saved = Some(header));
        unsafe { lvx_event_add(scroll, on_scroll, 12, scroll) };
    }
    Some(viewport.bounds().h)
}

fn status_cards(
    root: Obj,
    style: cards::StatusCardStyle,
    show_disclosure_indicator: bool,
) -> Option<i32> {
    for (index, (kind, title, subtitle)) in [
        (cards::StatusKind::Success, "All Set", "Ready to go"),
        (cards::StatusKind::Warning, "Attention", "Check settings"),
        (cards::StatusKind::Error, "Error", "Please try again"),
    ]
    .into_iter()
    .enumerate()
    {
        let card = unsafe {
            cards::status_card_with_style(
                root,
                8 + index as i32 * (cards::STATUS_CARD_HEIGHT + 12),
                title,
                subtitle,
                style,
                cards::StatusCardOptions {
                    kind,
                    show_disclosure_indicator,
                    ..Default::default()
                },
            )
        };
        if card.is_null() {
            return None;
        }
    }
    Some(3 * cards::STATUS_CARD_HEIGHT + 40)
}

fn rings(root: Obj, style: ProgressRingStyle, steps: u32, color: Option<u32>) -> Option<i32> {
    let is_determinate = style == ProgressRingStyle::Determinate;
    for (i, show_background) in [true, false].into_iter().enumerate() {
        for (column, progress) in (if is_determinate {
            vec![0, steps, STEPS]
        } else {
            vec![0]
        })
        .into_iter()
        .enumerate()
        {
            let x = if is_determinate {
                23 + column as i32 * 114
            } else {
                137
            };
            let mut ring = unsafe {
                CircularProgressIndicator::new(
                    root,
                    x,
                    38 + i as i32 * 100,
                    ProgressRingOptions {
                        style,
                        show_background,
                    },
                )
            }?;
            ring.set(progress);
            if let Some(color) = color {
                ring.set_color(color);
            }
        }
        unsafe {
            lv::label(
                root,
                if show_background {
                    b"Track shown\0"
                } else {
                    b"Track hidden\0"
                },
                0,
                4 + i as i32 * 100,
                336,
                30,
                0xFFFFFF,
                lv::Font::Small24,
            );
        }
    }
    Some(208)
}

extern "C" fn wheel_event(event: Obj) {
    WHEEL.with_borrow(|handle| {
        if let Some(handle) = handle {
            handle(event);
        }
    });
}

fn wheel<const N: usize>(root: Obj, style: NumericWheelStyle) -> Option<i32> {
    let columns = [
        Column {
            min: 0,
            max: 23,
            initial: 7,
            label: b"h\0",
            badge_width: 43,
        },
        Column {
            min: 0,
            max: 59,
            initial: 35,
            label: b"m\0",
            badge_width: 43,
        },
        Column {
            min: 0,
            max: 59,
            initial: 12,
            label: b"s\0",
            badge_width: 43,
        },
    ];
    let wheel = unsafe {
        NumericWheel::mount_with_styles(
            root,
            Rect::new(4, 0, 328, 200),
            core::array::from_fn(|i| columns[i]),
            [style; N],
            0,
            wheel_event,
        )
    }?;
    WHEEL
        .with_borrow_mut(|handle| *handle = Some(Box::new(move |event| wheel.handle_event(event))));
    Some(200)
}

extern "C" fn open_confirmation(_: Obj) {
    CONFIRMATION.with_borrow(|preview| {
        if let Some((root, style, options)) = *preview {
            unsafe { overlay::show_with_options(root, style, None, c"Timer Saved", options) };
        }
    });
}

fn confirmation(
    root: Obj,
    style: overlay::Style,
    detailed: bool,
    settings: &[&str],
) -> Option<i32> {
    let options = overlay::Options {
        description: detailed.then_some(c"Your timer is ready to use."),
        first_stage_ms: setting(settings, 1, 550u32).min(60000),
        first_stage_tap: if settings.get(2) == Some(&"0") {
            overlay::FirstStageTap::Dismiss
        } else {
            overlay::FirstStageTap::ShowDetails
        },
        timeout_ms: match setting(settings, 0, 4u32).min(60) {
            0 => None,
            n => Some(n * 1000),
        },
    };
    let viewport = aeris_ui::device::display_profile().viewport;
    let safe = viewport.content();
    let height = page::ButtonStyle::Primary.height_in(viewport);
    let button = unsafe {
        page::button_with_options(
            root,
            Rect::new(safe.x, safe.center_y() - height / 2, safe.w, height),
            "Show confirmation\0",
            open_confirmation,
            0,
            page::ButtonOptions {
                style: page::ButtonStyle::Primary,
                dimmed: false,
            },
        )
    };
    if button.is_null() {
        return None;
    }
    CONFIRMATION.with_borrow_mut(|saved| *saved = Some((root, style, options)));
    open_confirmation(std::ptr::null_mut());
    Some(viewport.bounds().h)
}

extern "C" fn close_dialog(_: Obj) {
    DIALOG.with_borrow(|saved| {
        if let Some(layers) = *saved {
            dialog::close(layers);
        }
    });
}

fn confirm_dialog(root: Obj, viewport: Viewport, double: bool) -> Option<i32> {
    let layers = unsafe {
        dialog::confirmation(
            root,
            viewport,
            "Save Changes?",
            "Do you want to save your changes before leaving?",
            dialog::Action { label: "Continue", callback: close_dialog },
            double.then_some("Cancel"),
        )
    }?;
    DIALOG.with_borrow_mut(|saved| *saved = Some(layers));
    dialog::open(root, layers);
    Some(viewport.bounds().h)
}

fn round_action(root: Obj, viewport: Viewport, variant: &str, settings: &[&str]) -> Option<i32> {
    let screen = viewport.bounds();
    let position = match settings.first().copied() {
        Some("top") => round_button::Position::Top,
        Some("bottom") => round_button::Position::Bottom,
        _ => round_button::Position::Center,
    };
    let options = round_button::Options {
        position,
        status: if variant.ends_with("primary") { round_button::Status::Primary } else { round_button::Status::Neutral },
        theme: if variant.starts_with("normal") { page::ButtonTheme::Normal } else { page::ButtonTheme::Glass },
    };
    let size = if viewport == aeris_ui::geometry::DisplayProfile::REDMI_WATCH.viewport { 76 } else { 68 };
    let slot = match position {
        round_button::Position::Center => Rect::new((screen.w - size) / 2, (screen.h - size) / 2, size, size),
        _ => Rect::new(0, 4, screen.w, screen.h - 8),
    };
    unsafe { round_button::create(root, slot, round_button::Icon::Settings, options, Some(noop), 0) }?;
    Some(screen.h)
}

fn setting<T: std::str::FromStr>(settings: &[&str], index: usize, default: T) -> T {
    settings
        .get(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn mount(parent: Obj, scene: &str) -> bool {
    let viewport = aeris_ui::device::display_profile().viewport;
    let scene = match scene {
        "layout-settings" => "layout:settings",
        "layout-empty" => "layout:empty",
        "layout-header" => "header:large-title",
        "adaptive-dialog" => "dialog:double",
        "adaptive-confirmation" => "confirmation-overlay:complex:0",
        "adaptive-round-button" => "round-button:glass-primary:bottom",
        _ => scene,
    };
    let root = unsafe { page::screen_in(parent, viewport) };
    if root.is_null() {
        return false;
    }
    let params: Vec<_> = scene.split(':').collect();
    let [component, variant, settings @ ..] = params.as_slice() else {
        return false;
    };
    let height = match (*component, *variant) {
        ("layout", "settings") => layout::settings(root, viewport),
        ("layout", "empty") => layout::empty(root, viewport),
        ("header", variant) => header(root, viewport, variant, settings.first().copied()),
        ("status-card", variant @ ("default" | "simple")) => {
            let style = if variant == "default" {
                cards::StatusCardStyle::Default
            } else {
                cards::StatusCardStyle::Simple
            };
            status_cards(root, style, setting(settings, 0, true))
        }
        ("progress-ring", variant @ ("determinate" | "indeterminate")) => rings(
            root,
            if variant == "determinate" {
                ProgressRingStyle::Determinate
            } else {
                ProgressRingStyle::Indeterminate
            },
            setting(settings, 0, 128u32).min(STEPS),
            settings
                .get(1)
                .and_then(|color| u32::from_str_radix(color, 16).ok()),
        ),
        ("text-block", variant @ ("standard" | "compact")) => {
            let compact = variant == "compact";
            let (style, height) = if compact {
                (TextBlockStyle::Compact, 80)
            } else {
                (TextBlockStyle::Standard, 152)
            };
            unsafe {
                text_block::create(
                    root,
                    Rect::new(4, 4, 328, height),
                    "Quick tip",
                    "Swipe down to open Quick Settings.",
                    TextBlockOptions {
                        style,
                        ..Default::default()
                    },
                )
            }
            .map(|_| height + 8)
        }
        ("capsule-button", variant @ ("regular" | "secondary" | "primary")) => {
            let style = match variant {
                "primary" => page::ButtonStyle::Primary,
                "secondary" => page::ButtonStyle::Secondary,
                _ => page::ButtonStyle::Regular,
            };
            let height = style.height_in(viewport);
            let safe = viewport.content();
            let button = unsafe {
                page::button_with_theme(
                    root,
                    viewport,
                    Rect::new(safe.x, safe.y, safe.w, height),
                    "Continue\0",
                    noop,
                    0,
                    page::ButtonOptions {
                        style,
                        dimmed: settings.contains(&"dimmed"),
                    },
                    if settings.contains(&"normal") { page::ButtonTheme::Normal } else { page::ButtonTheme::Glass },
                )
            };
            (!button.is_null()).then_some(viewport.bounds().h)
        }
        ("dialog", variant @ ("double" | "single")) => confirm_dialog(root, viewport, variant == "double"),
        ("round-button", variant) => round_action(root, viewport, variant, settings),
        ("icon-button", variant @ ("neutral" | "accent")) => {
            let style = if variant == "accent" {
                lv::RoundButtonStyle::Accent
            } else {
                lv::RoundButtonStyle::Neutral
            };
            let button =
                unsafe { lv::round_button_with_style(root, 134, 6, assets::ICON_ADD, style) };
            (!button.root.is_null()).then_some(80)
        }
        ("card", variant @ ("chevron" | "up-down" | "none")) => {
            let indicator = match variant {
                "up-down" => cards::JumpIndicator::UpDown,
                "none" => cards::JumpIndicator::None,
                _ => cards::JumpIndicator::Chevron,
            };
            let (card, _) = unsafe {
                cards::jump_card_with_options(
                    root,
                    4,
                    "Settings",
                    "Display",
                    cards::JumpCardOptions {
                        indicator,
                        ..Default::default()
                    },
                )
            };
            (!card.is_null()).then_some(106)
        }
        ("numeric-wheel", variant @ ("number-only" | "with-unit")) => {
            let style = if variant == "number-only" {
                NumericWheelStyle::NumberOnly
            } else {
                NumericWheelStyle::WithUnit
            };
            match setting(settings, 0, 2u32) {
                1 => wheel::<1>(root, style),
                3 => wheel::<3>(root, style),
                _ => wheel::<2>(root, style),
            }
        }
        ("confirmation-overlay", variant @ ("simple" | "complex" | "primary-soft" | "primary")) => {
            confirmation(
                root,
                match variant {
                    "primary" => overlay::Style::Primary,
                    "primary-soft" => overlay::Style::PrimarySoft,
                    _ => overlay::Style::Neutral,
                },
                variant != "simple",
                settings,
            )
        }
        _ => None,
    };
    if let Some(height) = height {
        unsafe { gallery_set_crop_height(height) };
        true
    } else {
        false
    }
}
