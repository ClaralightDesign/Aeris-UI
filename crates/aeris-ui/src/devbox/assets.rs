//! 浏览器预览的字形和位图注册。

use crate::device::packed_assets;
use crate::{free, gallery_asset, gallery_font, malloc};

pub(crate) fn load_fonts() {
    macro_rules! font {
        ($family:expr, $size:expr, $file:literal) => {
            unsafe {
                gallery_font(
                    $family,
                    $size,
                    include_bytes!(concat!("../../assets/linoval-", $file, ".font")).as_ptr(),
                )
            }
        };
    }
    font!(0, 24, "regular-24");
    font!(0, 25, "regular-25");
    font!(0, 26, "regular-26");
    font!(1, 24, "medium-24");
    font!(1, 25, "medium-25");
    font!(1, 26, "medium-26");
    font!(2, 28, "medium-28");
    font!(2, 30, "medium-30");
    font!(3, 32, "medium-32");
    font!(3, 40, "digits-40");
    font!(4, 44, "rounded-44");
}

pub(crate) unsafe fn register(name: &str, data: &[u8], scratch: &mut [u8]) -> bool {
    let Some(len) = packed_assets::decoded_len(data) else {
        return false;
    };
    let bytes = unsafe { malloc(len).cast::<u8>() };
    if bytes.is_null() {
        return false;
    }
    let out = unsafe { core::slice::from_raw_parts_mut(bytes, len) };
    let mut offset = 0;
    let decoded = packed_assets::decode(data, scratch, |chunk| {
        out.get_mut(offset..offset + chunk.len())?
            .copy_from_slice(chunk);
        offset += chunk.len();
        Some(())
    });
    if decoded.is_some() {
        if unsafe { gallery_asset(name.as_ptr(), name.len(), bytes, len) } {
            return true;
        }
    }
    unsafe { free(bytes.cast()) };
    false
}
