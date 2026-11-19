//! DevBox 的浏览器运行环境及应用启动接口。

extern crate std;

pub(crate) mod assets;
pub mod bindings;

pub use bindings::{LvxEventCallback, LvxTimerCallback, Obj};

/// 将应用的正常启动函数注册给 DevBox。函数接收 LVGL 根对象，成功时返回 true。
#[macro_export]
macro_rules! devbox_app {
    ($start:path) => {
        $crate::devbox_app!($start, before_assets = || {});
    };
    ($start:path, before_assets = $prepare:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn aeris_devbox_mount() -> i32 {
            ($prepare)();
            unsafe { $crate::devbox::boot($start) }
        }
    };
}

/// # 安全要求
/// 必须从 DevBox 的 LVGL 主线程调用，且显示和输入设备须已初始化。
#[doc(hidden)]
pub unsafe fn boot(start: impl FnOnce(Obj) -> bool) -> i32 {
    std::panic::set_hook(std::boxed::Box::new(|info| {
        if let Some(location) = info.location() {
            let file = location.file().as_bytes();
            unsafe { bindings::gallery_panic(file.as_ptr(), file.len(), location.line()) };
        }
    }));
    crate::device::assets::ensure_fonts();
    if !crate::device::assets::ensure() {
        return -1;
    }
    let root = unsafe { bindings::gallery_screen() };
    if root.is_null() || !start(root) {
        return -1;
    }
    0
}

/// 挂载预览场景。运行时仅在当前预览实例初始化时调用一次。
///
/// # 安全要求
/// 必须从 DevBox 的 LVGL 主线程调用，且显示和输入设备须已初始化。
#[doc(hidden)]
pub unsafe fn mount(render: fn(Obj, &str) -> bool) -> i32 {
    let mut id = [0; 128];
    let len = unsafe { bindings::devbox_scene_id(id.as_mut_ptr(), id.len()) };
    let Some(scene) = id.get(..len).and_then(|id| core::str::from_utf8(id).ok()) else {
        return -1;
    };

    unsafe { boot(|root| render(root, scene)) }
}

/// 将项目场景函数注册给 DevBox。函数接收 LVGL 根对象和场景 ID，挂载成功时返回 true。
#[macro_export]
macro_rules! devbox_scenes {
    ($render:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn aeris_devbox_mount() -> i32 {
            unsafe { $crate::devbox::mount($render) }
        }
    };
}
