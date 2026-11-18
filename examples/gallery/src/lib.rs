//! Aeris UI Gallery 的独立模块入口。

#![cfg_attr(not(target_os = "emscripten"), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(not(target_os = "emscripten"))]
use canopus_abi::*;
#[cfg(not(target_os = "emscripten"))]
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[cfg(not(target_os = "emscripten"))]
use aeris_ui::device;

#[cfg(not(target_os = "emscripten"))]
mod app_assets;
mod confirmation_demo;
mod gallery;
mod inputs;
#[cfg(not(target_os = "emscripten"))]
mod native_app;
#[cfg(target_os = "emscripten")]
#[path = "browser.rs"]
mod native_app;

#[cfg(target_os = "emscripten")]
extern crate self as canopus_target_private;
#[cfg(target_os = "emscripten")]
pub use aeris_ui::devbox::bindings::*;
#[cfg(target_os = "emscripten")]
aeris_ui::devbox_app!(native_app::start);

#[cfg(not(target_os = "emscripten"))]
static ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(not(target_os = "emscripten"))]
static LAST_ERROR: AtomicU32 = AtomicU32::new(0);

#[cfg(not(target_os = "emscripten"))]
const APP_NAME: &core::ffi::CStr = c"Aeris UI Gallery";
#[cfg(not(target_os = "emscripten"))]
const MAGIC: u32 = 0x4145_5249;

#[cfg(all(feature = "device", not(target_os = "emscripten")))]
const fn pack<const N: usize>(value: &[u8]) -> [u8; N] {
    let mut output = [0; N];
    let mut index = 0;
    while index < value.len() && index < N {
        output[index] = value[index];
        index += 1;
    }
    output
}

#[cfg(not(target_os = "emscripten"))]
#[unsafe(no_mangle)]
pub extern "C" fn canopus_mod_prepare(ctx: *const ContextV1) -> i32 {
    unsafe { device::set_context(ctx.cast()) };
    ACTIVE.store(false, Ordering::Release);
    LAST_ERROR.store(0, Ordering::Release);
    0
}

#[cfg(not(target_os = "emscripten"))]
#[unsafe(no_mangle)]
pub extern "C" fn canopus_mod_activate(ctx: *const ContextV1) -> i32 {
    if !ctx.is_null() {
        unsafe { device::set_context(ctx.cast()) };
    }
    ACTIVE.store(true, Ordering::Release);
    0
}

#[cfg(not(target_os = "emscripten"))]
#[unsafe(no_mangle)]
pub extern "C" fn canopus_mod_deactivate(_ctx: *const ContextV1) -> i32 {
    ACTIVE.store(false, Ordering::Release);
    0
}

#[cfg(not(target_os = "emscripten"))]
#[unsafe(no_mangle)]
pub extern "C" fn canopus_mod_stop(ctx: *const ContextV1) -> i32 {
    canopus_mod_deactivate(ctx)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[cfg(not(target_os = "emscripten"))]
#[unsafe(no_mangle)]
pub extern "C" fn canopus_mod_query(writer: *mut StatusWriterV1) -> i32 {
    if writer.is_null() {
        return -1;
    }
    let writer = unsafe { &mut *writer };
    unsafe {
        if !canopus_runtime::status_put_u32(writer, MAGIC)
            || !canopus_runtime::status_put_u32(writer, ACTIVE.load(Ordering::Acquire) as u32)
            || !canopus_runtime::status_put_u32(writer, LAST_ERROR.load(Ordering::Acquire))
            || !canopus_runtime::status_put_u32(writer, native_app::app_id())
        {
            return -1;
        }
    }
    canopus_runtime::status_writer_publish(writer);
    0
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[cfg(not(target_os = "emscripten"))]
#[unsafe(no_mangle)]
pub extern "C" fn canopus_mod_publish_native_app_stage(ctx: *const ContextV1, stage: u32) -> i32 {
    let result = unsafe { native_app::install_stage(ctx, stage) }
        .map(|()| 0)
        .unwrap_or_else(|error| error);
    if result != 0 {
        LAST_ERROR.store(result as u32, Ordering::Release);
    }
    result
}

#[cfg(all(feature = "device", not(target_os = "emscripten")))]
#[unsafe(no_mangle)]
pub static canopus_module_descriptor: ModuleDescriptorV1 = ModuleDescriptorV1 {
    struct_size: core::mem::size_of::<ModuleDescriptorV1>() as u32,
    abi_major: ABI_MAJOR,
    abi_minor: ABI_MINOR,
    flags: FLAG_HAS_NATIVE_APP
        | FLAG_NATIVE_APP_STANDALONE
        | FLAG_REGISTERS_LAUNCHER_ENTRY
        | FLAG_REQUIRES_UI_DISPATCHER
        | FLAG_APP_UNREGISTER_REBOOT_REQUIRED,
    module_id: pack(b"aeris_gallery"),
    module_version: pack(env!("CARGO_PKG_VERSION").as_bytes()),
    build_id: pack(concat!("aeris-gallery-", env!("CARGO_PKG_VERSION")).as_bytes()),
    target_id: pack(canopus_target_private::TARGET_ID.as_bytes()),
    prepare: Some(canopus_mod_prepare),
    activate: Some(canopus_mod_activate),
    deactivate: Some(canopus_mod_deactivate),
    stop: Some(canopus_mod_stop),
    query: Some(canopus_mod_query),
    publish_native_app: None,
    publish_native_app_stage: Some(canopus_mod_publish_native_app_stage),
};

#[cfg(all(feature = "device", not(target_os = "emscripten")))]
#[unsafe(no_mangle)]
pub extern "C" fn canopus_module_descriptor_ptr() -> *const ModuleDescriptorV1 {
    &canopus_module_descriptor
}

#[cfg(all(feature = "device", not(target_os = "emscripten")))]
#[unsafe(no_mangle)]
pub extern "C" fn canopus_register_module_descriptor() -> i32 {
    if canopus_target_private::canopus_identity_guard() != 0 {
        return -1;
    }
    #[repr(C)]
    struct Registration {
        magic: u32,
        descriptor: u32,
        module_id: [u8; 32],
    }
    let registration = Registration {
        magic: 0x3152_4d43, // 注册协议标记
        descriptor: core::ptr::addr_of!(canopus_module_descriptor) as usize as u32,
        module_id: pack(b"aeris_gallery"),
    };
    let path = b"/dev/canopus\0";
    let fd = unsafe { canopus_target_private::nuttx_open(path.as_ptr(), 2) };
    if fd < 0 {
        return fd;
    }
    let written = unsafe {
        canopus_target_private::nuttx_write(
            fd,
            core::ptr::addr_of!(registration).cast(),
            core::mem::size_of::<Registration>() as u32,
        )
    };
    let close = unsafe { canopus_target_private::nuttx_close(fd) };
    if written != core::mem::size_of::<Registration>() as i32 {
        return if written < 0 { written } else { -1 };
    }
    close
}

#[cfg(all(feature = "device", not(test), not(target_os = "emscripten")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
