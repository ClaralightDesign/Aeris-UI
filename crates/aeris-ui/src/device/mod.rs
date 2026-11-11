//! 小米手环 10 Pro 3.101.043 的原生界面组件。

use canopus_abi::CanopusContext;
use core::ffi::CStr;
use core::sync::atomic::{AtomicPtr, Ordering};

static CONTEXT: AtomicPtr<CanopusContext> = AtomicPtr::new(core::ptr::null_mut());

/// 设置当前模块的资源上下文。
///
/// # 安全要求
/// 上下文须在模块使用界面资源期间保持有效。
pub unsafe fn set_context(context: *const CanopusContext) {
    CONTEXT.store(context.cast_mut(), Ordering::Release);
}

fn context() -> *const CanopusContext {
    CONTEXT.load(Ordering::Acquire)
}

macro_rules! asset_path {
    ($name:literal) => {
        concat!($name, "\0").as_bytes()
    };
}

fn image_path<'a>(name: &'a [u8], output: &'a mut [u8]) -> Option<&'a [u8]> {
    CStr::from_bytes_with_nul(name).ok()?;
    if name.starts_with(b"/") {
        return Some(name);
    }
    let ctx = unsafe { context().as_ref()? };
    if ctx.module_id.is_null() {
        return None;
    }
    let module_id = unsafe { CStr::from_ptr(ctx.module_id) }.to_bytes();
    write_image_path(module_id, name, output)
}

fn write_image_path<'a>(module_id: &[u8], name: &[u8], output: &'a mut [u8]) -> Option<&'a [u8]> {
    const ROOT: &[u8] = b"/data/canopus/m.";
    if module_id.is_empty() {
        return None;
    }
    let separator = ROOT.len() + module_id.len();
    let output = output.get_mut(..separator + 1 + name.len())?;
    output[..ROOT.len()].copy_from_slice(ROOT);
    output[ROOT.len()..separator].copy_from_slice(module_id);
    output[separator] = b'/';
    output[separator + 1..].copy_from_slice(name);
    Some(output)
}

pub mod anim;
pub mod assets;
mod backdrop;
pub mod card_menu;
pub mod cards;
pub mod checkbox;
pub mod choice_list;
pub mod confirm_slider;
pub mod countdown_card;
pub mod dialog;
pub mod font;
mod font_assets;
pub mod header;
pub mod keyboard;
#[cfg(feature = "gallery")]
mod linoval;
pub mod lv;
pub mod numeric_wheel;
pub mod option_grid;
mod packed_assets;
pub mod page;
pub mod page_indicator;
pub mod progress_bar;
pub mod progress_ring;
pub mod rolling_number;
pub mod stepped_slider;

#[cfg(test)]
mod tests {
    use super::write_image_path;

    #[test]
    fn resolves_assets_under_each_module_data_directory() {
        let mut output = [0; 64];
        assert_eq!(
            write_image_path(b"sample", b"card-face.bin\0", &mut output),
            Some(b"/data/canopus/m.sample/card-face.bin\0".as_slice())
        );
        assert_eq!(
            write_image_path(b"another", b"font-card.bin\0", &mut output),
            Some(b"/data/canopus/m.another/font-card.bin\0".as_slice())
        );
        assert!(write_image_path(b"", b"card-face.bin\0", &mut output).is_none());
        assert!(write_image_path(b"sample", b"card-face.bin\0", &mut output[..8]).is_none());
    }
}
