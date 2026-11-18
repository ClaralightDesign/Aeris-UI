//! 键盘布局与输入预览。

use canopus_target_private::*;

use crate::geometry::Rect;

use super::{
    lv::{self, Font, Obj},
    page,
};

pub struct Labels<'a> {
    pub title: &'a str,
    pub space: &'a str,
    pub delete: &'a str,
    pub done: &'a str,
}

pub struct Keyboard<const N: usize> {
    pub root: Obj,
    preview: Obj,
    text: [u8; N],
    len: usize,
}

impl<const N: usize> Keyboard<N> {
    pub unsafe fn new(
        parent: Obj,
        labels: Labels<'_>,
        on_key: LvxEventCallback,
        on_done: LvxEventCallback,
        on_back: LvxEventCallback,
    ) -> Option<Self> {
        if N < 2 {
            return None;
        }
        unsafe {
            let root = page::screen(parent);
            if root.is_null() {
                return None;
            }
            lv::inspect_name(root, c"AerisUI.Keyboard");
            lvx_object_add_flag(root, lv::CLICKABLE);
            let field = lv::card(root, 4, 84);
            if field.is_null() {
                lvx_object_delete(root);
                return None;
            }
            let preview = lv::label(field, b"\0", 16, 30, 296, 38, 0xFFFFFF, Font::Medium24);
            if !preview.is_null() {
                lvx_label_set_long_mode(preview, LV_LABEL_LONG_SCROLL_CIRCULAR);
            }
            for (row, letters) in [b"qwertyuiop".as_slice(), b"asdfghjkl", b"zxcvbnm"]
                .into_iter()
                .enumerate()
            {
                let x = (page::WIDTH - letters.len() as i32 * 33) / 2;
                for (column, byte) in letters.iter().copied().enumerate() {
                    let label = [byte, 0];
                    key_bytes(
                        root,
                        x + column as i32 * 33,
                        198 + row as i32 * 46,
                        31,
                        42,
                        &label,
                        on_key,
                        usize::from(byte),
                    );
                }
            }
            key(root, 4, 336, 210, 48, labels.space, on_key, b' ' as usize);
            key(root, 220, 336, 112, 48, labels.delete, on_key, 0);
            let style = page::ButtonStyle::Primary;
            page::button_with_options(
                root,
                Rect::new(0, 395, page::WIDTH, style.height()),
                labels.done,
                on_done,
                0,
                page::ButtonOptions {
                    style,
                    dimmed: false,
                },
            );
            page::header(root, labels.title, on_back, false);
            Some(Self {
                root,
                preview,
                text: [0; N],
                len: 0,
            })
        }
    }

    pub fn load(&mut self, text: &[u8]) {
        self.len = text
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(text.len())
            .min(N - 1);
        self.text[..self.len].copy_from_slice(&text[..self.len]);
        self.text[self.len] = 0;
        self.paint();
    }

    pub fn handle_key(&mut self, event: Obj) {
        if event.is_null() {
            return;
        }
        let byte = unsafe { lvx_event_get_user_data(event) } as u8;
        if byte == 0 {
            self.len = self.len.saturating_sub(1);
        } else if self.len < N - 1 {
            self.text[self.len] = byte;
            self.len += 1;
        }
        self.text[self.len] = 0;
        self.paint();
    }

    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.text[..self.len]).unwrap_or_default()
    }

    fn paint(&self) {
        if !self.preview.is_null() {
            unsafe { lvx_label_set_text(self.preview, self.text.as_ptr()) };
        }
    }
}

unsafe fn key(
    parent: Obj,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    title: &str,
    action: LvxEventCallback,
    data: usize,
) {
    unsafe { key_bytes(parent, x, y, width, height, title.as_bytes(), action, data) };
}

unsafe fn key_bytes(
    parent: Obj,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    title: &[u8],
    action: LvxEventCallback,
    data: usize,
) {
    unsafe {
        let button = lv::background(parent, width, height, 0x1F1F1F, 12);
        if button.is_null() {
            return;
        }
        lvx_object_align(button, 1, x, y);
        lvx_object_clear_flag(button, lv::SCROLLABLE);
        lv::label(
            button,
            title,
            0,
            (height - 32) / 2,
            width,
            32,
            0xFFFFFF,
            Font::Medium24,
        );
        lv::bind(button, action, data);
    }
}
