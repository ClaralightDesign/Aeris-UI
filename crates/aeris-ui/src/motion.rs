//! Integer page-transition curves shared by the HTML preview and the LVGL
//! renderer. No floating point, no `lv_anim`, no heap.

pub const PAGE_TRANSITION_MS: u32 = 280;
pub const DOT_TRANSITION_MS: u32 = 180;
pub const SNAP_PERCENT: i32 = 18;
pub const SCALE_ONE: i32 = 1024;
pub const PAGE_INACTIVE_SCALE: i32 = 993; // ~97%
pub const PAGE_INACTIVE_OPACITY: u8 = 133; // 52%
pub const DOT_INACTIVE_SIZE: i32 = 10;
pub const DOT_ACTIVE_SIZE: i32 = 14;

fn clamp_0_1000(value: i32) -> i32 {
    value.clamp(0, 1000)
}

/// Ease-out cubic on a 0..1000 timeline.
pub fn ease_out_cubic(progress: i32) -> i32 {
    let t = clamp_0_1000(progress);
    let inverse = 1000 - t;
    1000 - (inverse * inverse * inverse) / 1_000_000
}

fn lerp(from: i32, to: i32, progress: i32) -> i32 {
    from + (to - from) * clamp_0_1000(progress) / 1000
}

/// Horizontal track offset for a transition from `from` to `to`.
pub fn page_offset(from: usize, to: usize, progress: i32, page_width: i32) -> i32 {
    let p = ease_out_cubic(progress);
    let start = -(from as i32) * page_width;
    let end = -(to as i32) * page_width;
    lerp(start, end, p)
}

/// Opacity (0..255) for one page during a transition.
pub fn page_opacity(from: usize, to: usize, index: usize, progress: i32) -> u8 {
    let p = ease_out_cubic(progress);
    if index == to && to != from {
        lerp(PAGE_INACTIVE_OPACITY as i32, 255, p) as u8
    } else if index == from && to != from {
        lerp(255, PAGE_INACTIVE_OPACITY as i32, p) as u8
    } else if index == to {
        255
    } else {
        PAGE_INACTIVE_OPACITY
    }
}

/// LVGL transform scale (1024 = 100%) for one page during a transition.
pub fn page_scale(from: usize, to: usize, index: usize, progress: i32) -> i32 {
    let p = ease_out_cubic(progress);
    if index == to && to != from {
        lerp(PAGE_INACTIVE_SCALE, SCALE_ONE, p)
    } else if index == from && to != from {
        lerp(SCALE_ONE, PAGE_INACTIVE_SCALE, p)
    } else if index == to {
        SCALE_ONE
    } else {
        PAGE_INACTIVE_SCALE
    }
}

/// Dot diameter during a page change. The outgoing dot shrinks while the
/// incoming dot grows.
pub fn dot_size(is_incoming: bool, progress: i32) -> i32 {
    let p = ease_out_cubic(progress);
    if is_incoming {
        lerp(DOT_INACTIVE_SIZE, DOT_ACTIVE_SIZE, p)
    } else {
        lerp(DOT_ACTIVE_SIZE, DOT_INACTIVE_SIZE, p)
    }
}

/// Snap to a neighbouring page when the drag passes 18% of the page width.
pub fn snap_target(from: usize, drag_dx: i32, page_width: i32, pages: usize) -> usize {
    if pages == 0 {
        return 0;
    }
    let threshold = page_width * SNAP_PERCENT / 100;
    let next = if drag_dx <= -threshold {
        from.saturating_add(1)
    } else if drag_dx >= threshold {
        from.saturating_sub(1)
    } else {
        from
    };
    next.min(pages - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_hits_both_ends_and_is_monotonic() {
        assert_eq!(ease_out_cubic(0), 0);
        assert_eq!(ease_out_cubic(1000), 1000);
        let mut previous = 0;
        for step in 0..=10 {
            let value = ease_out_cubic(step * 100);
            assert!(value >= previous);
            previous = value;
        }
    }

    #[test]
    fn page_offset_moves_from_current_to_next() {
        assert_eq!(page_offset(0, 1, 0, 336), 0);
        assert_eq!(page_offset(0, 1, 1000, 336), -336);
        assert_eq!(page_offset(1, 0, 0, 336), -336);
        assert_eq!(page_offset(1, 0, 1000, 336), 0);
    }

    #[test]
    fn incoming_page_fades_in_and_outgoing_page_fades_out() {
        assert_eq!(page_opacity(0, 1, 0, 0), 255);
        assert_eq!(page_opacity(0, 1, 1, 0), PAGE_INACTIVE_OPACITY);
        assert_eq!(page_opacity(0, 1, 1, 1000), 255);
        assert_eq!(page_opacity(0, 1, 0, 1000), PAGE_INACTIVE_OPACITY);
        assert_eq!(page_opacity(1, 1, 1, 500), 255);
    }

    #[test]
    fn incoming_page_scales_up() {
        assert_eq!(page_scale(0, 1, 1, 0), PAGE_INACTIVE_SCALE);
        assert_eq!(page_scale(0, 1, 1, 1000), SCALE_ONE);
        assert_eq!(page_scale(0, 1, 0, 1000), PAGE_INACTIVE_SCALE);
    }

    #[test]
    fn dot_grows_or_shrinks_between_ten_and_fourteen() {
        assert_eq!(dot_size(true, 0), 10);
        assert_eq!(dot_size(true, 1000), 14);
        assert_eq!(dot_size(false, 0), 14);
        assert_eq!(dot_size(false, 1000), 10);
    }

    #[test]
    fn snap_needs_eighteen_percent_of_the_page_width() {
        // 18% of 336 is 60.48 -> integer threshold 60.
        assert_eq!(snap_target(0, -59, 336, 2), 0);
        assert_eq!(snap_target(0, -61, 336, 2), 1);
        assert_eq!(snap_target(1, 61, 336, 2), 0);
        assert_eq!(snap_target(1, -1000, 336, 2), 1);
    }
}
