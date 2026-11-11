//! Band 10 Pro 3.101.043 对话框背景快照。

use core::{arch::asm, ffi::c_void, mem::transmute, ptr, slice};

use canopus_target_private::generated_symbols::{
    P67_BLUR_CACHE_CLEAN_CALLABLE, P67_BLUR_CACHE_INVALIDATE_CALLABLE,
    P67_BLUR_DRAW_BUF_CREATE_CALLABLE, P67_BLUR_DRAW_BUF_DESTROY_CALLABLE,
    P67_BLUR_GPU_FINISH_CALLABLE, P67_BLUR_PM_LOCK_ENTRY, P67_BLUR_PM_RELAX_CALLABLE,
    P67_BLUR_PM_STAY_CALLABLE, P67_BLUR_SNAPSHOT_TO_BUFFER_CALLABLE,
};

use super::lv::Obj;

const DOWNSAMPLE: usize = 4;
pub const IMAGE_SCALE: i32 = DOWNSAMPLE as i32 * 256;
const WIDTH: usize = super::page::WIDTH as usize;
const HEIGHT: usize = super::page::HEIGHT as usize;
const SMALL_W: usize = WIDTH / DOWNSAMPLE;
const SMALL_H: usize = HEIGHT / DOWNSAMPLE;
const PIXELS: usize = SMALL_W * SMALL_H;
const RADIUS: usize = 2;
const DARKEN_PERCENT: u32 = 65;
const CHANNEL_SCALE: u32 = 256;
const IMAGE_MAGIC: u32 = 0x19;
const BGR888: u32 = 15;
const RGB565: u32 = 18;
const DISPLAY_SLOT: *const usize = 0x2010318c as _;
const DRAW_BUF_HANDLERS: usize = 0x20103244;

#[repr(C)]
struct DrawBuffer {
    format: u32,
    dimensions: u32,
    stride: u32,
    len: u32,
    data: *mut u8,
    allocation: usize,
    handlers: usize,
}

type Rgb = [u16; 3];

static mut WORK: [Rgb; PIXELS] = [[0; 3]; PIXELS];
static mut PENDING: *mut DrawBuffer = ptr::null_mut();
static mut IMAGE: *mut DrawBuffer = ptr::null_mut();

struct SnapshotPower;

impl SnapshotPower {
    fn stay() -> Self {
        unsafe {
            let stay: extern "C" fn(usize) -> i32 = transmute(P67_BLUR_PM_STAY_CALLABLE);
            stay(P67_BLUR_PM_LOCK_ENTRY);
        }
        Self
    }
}

impl Drop for SnapshotPower {
    fn drop(&mut self) {
        unsafe {
            let relax: extern "C" fn(usize) -> i32 = transmute(P67_BLUR_PM_RELAX_CALLABLE);
            relax(P67_BLUR_PM_LOCK_ENTRY);
        }
    }
}

// 调用时弹窗必须隐藏，且当前线程持有 UI 所有权。
pub unsafe fn capture(root: Obj) -> *const c_void {
    unsafe {
        if !display_idle() {
            return ptr::null();
        }
        let _power = SnapshotPower::stay();
        if !fence() || !display_idle() {
            return ptr::null();
        }
        release_pending();

        let create: extern "C" fn(u32, u32, u32, u32) -> *mut DrawBuffer =
            transmute(P67_BLUR_DRAW_BUF_CREATE_CALLABLE);
        let snapshot: extern "C" fn(Obj, u32, *mut DrawBuffer) -> i32 =
            transmute(P67_BLUR_SNAPSHOT_TO_BUFFER_CALLABLE);
        let input = create(WIDTH as u32, HEIGHT as u32, BGR888, 0);
        if view(input, BGR888, WIDTH, HEIGHT).is_none() {
            if !input.is_null() {
                destroy(input);
            }
            return ptr::null();
        }

        clean_cache();
        let result = snapshot(root, BGR888, input);
        // 快照返回值不包含 GPU 状态，失败时保留缓冲区直至安全释放。
        if !fence() || !display_idle() {
            PENDING = input;
            return ptr::null();
        }
        let pixels = if result == 1 {
            view(input, BGR888, WIDTH, HEIGHT)
        } else {
            None
        };
        let Some((data, stride, len)) = pixels else {
            destroy(input);
            return ptr::null();
        };

        clean_cache();
        let invalidate: extern "C" fn(u32, *mut u8, usize) -> i32 =
            transmute(P67_BLUR_CACHE_INVALIDATE_CALLABLE);
        invalidate(1, data, len);
        barrier();
        let work = &mut *ptr::addr_of_mut!(WORK);
        downsample(slice::from_raw_parts(data, len), stride, work);

        // 复用快照空间进行两轴滤波，完成后压暗并转换为 RGB565。
        let scratch = &mut *data.cast::<[Rgb; PIXELS]>();
        box_pass(work, scratch, false);
        box_pass(scratch, work, true);
        // 采用固件分配的行跨度，图片解码器可直接读取更新后的像素。
        let first_image = IMAGE.is_null();
        let image = if first_image {
            create(SMALL_W as u32, SMALL_H as u32, RGB565, 0)
        } else {
            IMAGE
        };
        let Some((pixels, stride, len)) = view(image, RGB565, SMALL_W, SMALL_H) else {
            if first_image && !image.is_null() {
                destroy(image);
            }
            destroy(input);
            return ptr::null();
        };
        let dst = slice::from_raw_parts_mut(pixels, len);
        for (row, colors) in dst.chunks_exact_mut(stride).zip(work.chunks_exact(SMALL_W)) {
            for (pixel, &color) in row.chunks_exact_mut(2).zip(colors) {
                pixel.copy_from_slice(&darken_rgb565(color).to_le_bytes());
            }
        }
        destroy(input);
        clean_cache();
        IMAGE = image;
        image.cast()
    }
}

pub fn retire() {
    unsafe {
        if !PENDING.is_null() && display_idle() {
            let _power = SnapshotPower::stay();
            if fence() && display_idle() {
                release_pending();
            }
        }
    }
}

unsafe fn view(
    input: *const DrawBuffer,
    format: u32,
    width: usize,
    height: usize,
) -> Option<(*mut u8, usize, usize)> {
    unsafe {
        if !ram(input as usize, size_of::<DrawBuffer>()) {
            return None;
        }
        let buffer = &*input;
        let stride = (buffer.stride & 0xffff) as usize;
        let len = buffer.len as usize;
        let data = buffer.data as usize;
        let pixel_size = if format == BGR888 { 3 } else { 2 };
        if buffer.format & 0xffff != IMAGE_MAGIC | format << 8
            || buffer.dimensions != width as u32 | (height as u32) << 16
            || !(width * pixel_size..=2048).contains(&stride)
            || (format == RGB565 && stride % 4 != 0)
            || !(stride * height..=1024 * 1024).contains(&len)
            || data % 64 != 0
            || data.wrapping_sub(buffer.allocation) > 63
            || !ram(buffer.allocation, len + data - buffer.allocation)
            || buffer.handlers != DRAW_BUF_HANDLERS
        {
            return None;
        }
        Some((buffer.data, stride, len))
    }
}

fn ram(address: usize, size: usize) -> bool {
    [(0x3c271400, 0x3dfc0000), (0x200765c0, 0x2015fec0)]
        .iter()
        .any(|&(start, end)| (start..end).contains(&address) && size <= end - address)
}

unsafe fn display_idle() -> bool {
    unsafe {
        let display = DISPLAY_SLOT.read_volatile();
        if !ram(display, 684) || ((display + 58) as *const u8).read_volatile() & 2 != 0 {
            return false;
        }
        // 确认 LVGL 已经提交图层任务，GPU 栅栏才能覆盖这些任务。
        let mut layer = ((display + 680) as *const usize).read_volatile();
        for _ in 0..16 {
            if layer == 0 {
                return DISPLAY_SLOT.read_volatile() == display
                    && ((display + 58) as *const u8).read_volatile() & 2 == 0;
            }
            if !ram(layer, 120) || ((layer + 100) as *const usize).read_volatile() != 0 {
                return false;
            }
            layer = ((layer + 108) as *const usize).read_volatile();
        }
        false
    }
}

unsafe fn release_pending() {
    unsafe {
        if !PENDING.is_null() {
            let input = PENDING;
            PENDING = ptr::null_mut();
            destroy(input);
        }
    }
}

unsafe fn destroy(input: *mut DrawBuffer) {
    let destroy: extern "C" fn(*mut DrawBuffer) =
        unsafe { transmute(P67_BLUR_DRAW_BUF_DESTROY_CALLABLE) };
    destroy(input);
}

fn fence() -> bool {
    unsafe {
        let finish: extern "C" fn() -> i32 = transmute(P67_BLUR_GPU_FINISH_CALLABLE);
        let status = finish();
        barrier();
        status == 0
    }
}

fn clean_cache() {
    unsafe {
        let clean: extern "C" fn(u32, usize, usize) -> i32 =
            transmute(P67_BLUR_CACHE_CLEAN_CALLABLE);
        clean(1, 0, 0x10000);
    }
    barrier();
}

fn barrier() {
    unsafe { asm!("dsb sy", "isb sy", options(nostack, preserves_flags)) };
}

fn downsample(input: &[u8], stride: usize, output: &mut [Rgb; PIXELS]) {
    const BLOCK_BYTES: usize = DOWNSAMPLE * 3;
    const SAMPLES: u32 = (DOWNSAMPLE * DOWNSAMPLE) as u32;
    for (y, row) in output.chunks_exact_mut(SMALL_W).enumerate() {
        for (x, pixel) in row.iter_mut().enumerate() {
            let start = y * DOWNSAMPLE * stride + x * BLOCK_BYTES;
            let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
            for dy in 0..DOWNSAMPLE {
                let offset = start + dy * stride;
                for bgr in input[offset..offset + BLOCK_BYTES].chunks_exact(3) {
                    b += u32::from(bgr[0]);
                    g += u32::from(bgr[1]);
                    r += u32::from(bgr[2]);
                }
            }
            *pixel = [r, g, b].map(|sum| (sum * CHANNEL_SCALE / SAMPLES) as u16);
        }
    }
}

fn box_pass(input: &[Rgb; PIXELS], output: &mut [Rgb; PIXELS], vertical: bool) {
    let (lines, length, line_step, pixel_step) = if vertical {
        (SMALL_W, SMALL_H, 1, SMALL_W)
    } else {
        (SMALL_H, SMALL_W, SMALL_W, 1)
    };
    for line in 0..lines {
        let base = line * line_step;
        for pos in 0..length {
            let start = pos.saturating_sub(RADIUS);
            let end = (pos + RADIUS + 1).min(length);
            let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
            for i in start..end {
                let [red, green, blue] = input[base + i * pixel_step];
                r += u32::from(red);
                g += u32::from(green);
                b += u32::from(blue);
            }
            let count = (end - start) as u32;
            output[base + pos * pixel_step] =
                [r, g, b].map(|sum| ((sum + count / 2) / count) as u16);
        }
    }
}

fn darken_rgb565(color: Rgb) -> u16 {
    const DIVISOR: u32 = CHANNEL_SCALE * 100;
    let [r, g, b] = color
        .map(|value| ((u32::from(value) * (100 - DARKEN_PERCENT) + DIVISOR / 2) / DIVISOR) as u16);
    (r >> 3) << 11 | (g >> 2) << 5 | (b >> 3)
}
