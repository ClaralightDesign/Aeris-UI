//! 锚定卡片的双操作菜单与局部遮罩。

use canopus_target_private::*;

use super::{
    anim,
    lv::{self, Font, Obj},
    page,
};

const NULL: Obj = core::ptr::null_mut();
const WIDTH: i32 = 224;
const ROW_HEIGHT: i32 = 68;
const HEIGHT: i32 = 2 * ROW_HEIGHT + 1;
const LABEL_HEIGHT: i32 = 36;
const GAP: i32 = 8;
const PANEL: u32 = 0x242424;

#[derive(Clone, Copy)]
pub struct CardMenu {
    root: Obj,
    overlay: Obj,
    shades: [Obj; 4],
    menu: Obj,
    labels: [Obj; 2],
}

impl CardMenu {
    pub unsafe fn new(
        root: Obj,
        outside: LvxEventCallback,
        rows: [(&str, u32, LvxEventCallback); 2],
    ) -> Option<Self> {
        unsafe {
            let overlay = lv::background(root, page::WIDTH, page::HEIGHT, 0, 0);
            if overlay.is_null() {
                return None;
            }
            lvx_object_add_flag(overlay, lv::CLICKABLE);
            lvx_object_clear_flag(
                overlay,
                lv::SCROLLABLE | lv::SCROLL_CHAIN | lv::GESTURE_BUBBLE,
            );
            lv::style(overlay, lv::BG_OPA, 0);
            lv::bind(overlay, outside, 0);

            let mut shades = [NULL; 4];
            for shade in &mut shades {
                *shade = lv::background(overlay, 1, 1, 0, 0);
                if shade.is_null() {
                    lvx_object_delete(overlay);
                    return None;
                }
                lvx_object_clear_flag(*shade, lv::CLICKABLE | lv::SCROLLABLE);
                lv::style(*shade, lv::BG_OPA, 128);
            }

            let menu = lv::background(overlay, WIDTH, HEIGHT, PANEL, 20);
            if menu.is_null() {
                lvx_object_delete(overlay);
                return None;
            }
            lvx_object_add_flag(menu, lv::CLICKABLE);
            lvx_object_clear_flag(menu, lv::SCROLLABLE);
            lv::style(menu, LV_STYLE_CLIP_CORNER, 1);

            let divider = lv::background(menu, WIDTH - 2 * GAP, 1, 0x414141, 0);
            if !divider.is_null() {
                lvx_object_align(divider, 1, GAP, ROW_HEIGHT);
                lvx_object_clear_flag(divider, lv::CLICKABLE | lv::SCROLLABLE);
            }
            let labels = core::array::from_fn(|index| {
                let (title, color, action) = rows[index];
                row(menu, index as i32 * (ROW_HEIGHT + 1), title, color, action)
            });
            if labels.iter().any(|label| label.is_null()) {
                lvx_object_delete(overlay);
                return None;
            }
            lvx_set_hidden(overlay, 1);
            Some(Self {
                root,
                overlay,
                shades,
                menu,
                labels,
            })
        }
    }

    pub unsafe fn show(&self, card: Obj) {
        unsafe {
            anim::settle();
            let [root_x, root_y, _, _] = lvx_object_coords(self.root);
            let [left, top, right, bottom] = lvx_object_coords(card);
            let left = (left - root_x).clamp(0, page::WIDTH);
            let top = (top - root_y).clamp(page::PAGE_TOP, page::HEIGHT);
            let right = (right - root_x + 1).clamp(left, page::WIDTH);
            let bottom = (bottom - root_y + 1).clamp(top, page::HEIGHT);
            self.shade_around(left, top, right, bottom);

            let x = (left + GAP).clamp(GAP, page::WIDTH - WIDTH - GAP);
            let y = if bottom + GAP + HEIGHT <= page::HEIGHT - GAP {
                bottom + GAP
            } else {
                top - HEIGHT - GAP
            }
            .clamp(GAP, page::HEIGHT - HEIGHT - GAP);
            lvx_object_align(self.menu, 1, x, y);

            lvx_set_hidden(self.overlay, 0);
        }
    }

    pub unsafe fn hide(&self) {
        unsafe { lvx_set_hidden(self.overlay, 1) };
    }

    pub unsafe fn set_label(&self, index: usize, text: &core::ffi::CStr) {
        if let Some(&label) = self.labels.get(index) {
            unsafe { lvx_label_set_text(label, text.as_ptr().cast()) };
        }
    }

    unsafe fn shade_around(&self, left: i32, top: i32, right: i32, bottom: i32) {
        // 遮罩覆盖卡片四周，卡片保留在列表中。
        let areas = [
            (0, 0, page::WIDTH, top),
            (0, bottom, page::WIDTH, page::HEIGHT - bottom),
            (0, top, left, bottom - top),
            (right, top, page::WIDTH - right, bottom - top),
        ];
        for (shade, (x, y, width, height)) in self.shades.into_iter().zip(areas) {
            let empty = width == 0 || height == 0;
            unsafe {
                lvx_set_hidden(shade, u32::from(empty));
                if !empty {
                    lvx_object_set_size(shade, width, height);
                    lvx_object_align(shade, 1, x, y);
                }
            }
        }
    }
}

unsafe fn row(parent: Obj, y: i32, title: &str, color: u32, action: LvxEventCallback) -> Obj {
    unsafe {
        let button = lv::background(parent, WIDTH, ROW_HEIGHT, PANEL, 0);
        if button.is_null() {
            return NULL;
        }
        lvx_object_align(button, 1, 0, y);
        lvx_object_clear_flag(button, lv::SCROLLABLE);
        anim::attach(button, WIDTH / 2, ROW_HEIGHT / 2, anim::Feedback::Scale);
        lv::bind(button, action, 0);
        let label = lv::label(
            button,
            title.as_bytes(),
            20,
            (ROW_HEIGHT - LABEL_HEIGHT) / 2,
            WIDTH - 40,
            LABEL_HEIGHT,
            color,
            Font::Medium28,
        );
        lv::style(label, 94, 1);
        label
    }
}
