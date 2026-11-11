//! 倒计时圆环的进度与裁切范围。

pub const STEPS: u32 = 256;
const QUARTER: u32 = STEPS / 4;

pub fn quadrant_lengths(steps: u32, half: i32, active_length: i32) -> [i32; 4] {
    let active = (steps / QUARTER) as usize;
    core::array::from_fn(|index| match index.cmp(&active) {
        core::cmp::Ordering::Less => half,
        core::cmp::Ordering::Equal => active_length,
        core::cmp::Ordering::Greater => 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_active_quadrant_uses_the_frame_cut() {
        let half = 162;
        assert_eq!(quadrant_lengths(0, half, 0), [0; 4]);
        assert_eq!(quadrant_lengths(63, half, 141), [141, 0, 0, 0]);
        assert_eq!(quadrant_lengths(64, half, 0), [half, 0, 0, 0]);
        assert_eq!(quadrant_lengths(129, half, 4), [half, half, 4, 0]);
        assert_eq!(quadrant_lengths(255, half, 143), [half, half, half, 143]);
        assert_eq!(quadrant_lengths(STEPS, half, 0), [half; 4]);
    }
}
