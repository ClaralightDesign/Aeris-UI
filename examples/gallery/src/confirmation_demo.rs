//! 三种底板及二级内容的组件示例。

use aeris_ui::{
    capsule::CapsuleKind,
    device::{
        confirmation_overlay::{self, FirstStageTap, Icon, Options, Style},
        lv::Obj,
        page,
    },
    geometry::Rect,
};
use canopus_target_private::lvx_event_get_user_data;

const WARNING_ICON: Icon<'_> = Icon {
    path: c"confirmation-warning.bin",
    width: 102,
    height: 102,
};
const FAILURE_ICON: Icon<'_> = Icon {
    path: c"confirmation-failure.bin",
    width: 102,
    height: 102,
};

pub const ASSETS: &[(&str, &[u8])] = &[
    (
        "confirmation-warning.bin",
        include_bytes!("../assets/confirmation-warning.bin.lz4"),
    ),
    (
        "confirmation-failure.bin",
        include_bytes!("../assets/confirmation-failure.bin.lz4"),
    ),
];

pub unsafe fn buttons(parent: Obj, root: Obj, y: i32, width: i32) {
    for (index, (label, kind, action)) in [
        (
            "Neutral\0",
            CapsuleKind::Large,
            neutral as canopus_target_private::LvxEventCallback,
        ),
        ("Soft Primary\0", CapsuleKind::Large, primary_soft),
        ("Primary\0", CapsuleKind::LargePrimary, primary),
        ("Show Details\0", CapsuleKind::Large, detail),
    ]
    .into_iter()
    .enumerate()
    {
        unsafe {
            page::button_in(
                parent,
                Rect::new(0, y + 88 * index as i32, width, kind.height()),
                label,
                kind,
                action,
                root as usize,
            )
        };
    }
}

extern "C" fn neutral(event: Obj) {
    show(event, Style::Neutral, None, c"Record saved", None);
}

extern "C" fn primary_soft(event: Obj) {
    show(
        event,
        Style::PrimarySoft,
        Some(WARNING_ICON),
        c"Check your settings",
        None,
    );
}

extern "C" fn primary(event: Obj) {
    show(
        event,
        Style::Primary,
        Some(FAILURE_ICON),
        c"Could not save",
        None,
    );
}

extern "C" fn detail(event: Obj) {
    show(
        event,
        Style::Neutral,
        None,
        c"Record saved",
        Some(
            c"Your timer record was saved. You can find it in your history and review it any time.",
        ),
    );
}

fn show(
    event: Obj,
    style: Style,
    icon: Option<Icon<'_>>,
    title: &core::ffi::CStr,
    description: Option<&core::ffi::CStr>,
) {
    let timeout = confirmation_overlay::DEFAULT_TIMEOUT_MS;
    let first_stage_ms = confirmation_overlay::DEFAULT_FIRST_STAGE_MS;
    let first_stage_tap = FirstStageTap::ShowDetails;
    let options = Options {
        description,
        first_stage_ms,
        first_stage_tap,
        timeout_ms: (timeout != 0).then_some(timeout),
    };
    unsafe {
        let root = lvx_event_get_user_data(event) as Obj;
        confirmation_overlay::show_with_options_in(
            root, aeris_ui::device::display_profile().viewport, style, icon, title, options,
        );
    }
}
