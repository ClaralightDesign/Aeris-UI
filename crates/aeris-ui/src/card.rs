//! 卡片状态过渡。

pub const TRANSITION_MS: u32 = 220;

#[derive(Clone, Copy)]
pub struct Transition {
    pub value: i32,
    from: i32,
    to: i32,
    started: u32,
}

impl Transition {
    pub const fn new(selected: bool) -> Self {
        let value = if selected { 1000 } else { 0 };
        Self {
            value,
            from: value,
            to: value,
            started: 0,
        }
    }

    pub fn target(&self) -> bool {
        self.to == 1000
    }

    pub fn retarget(&mut self, selected: bool, now: u32) {
        self.sample(now);
        self.from = self.value;
        self.to = if selected { 1000 } else { 0 };
        self.started = now;
    }

    pub fn sample(&mut self, now: u32) -> bool {
        let elapsed = now.wrapping_sub(self.started).min(TRANSITION_MS);
        let remaining = 1000 - elapsed as i32 * 1000 / TRANSITION_MS as i32;
        let eased = 1000 - remaining * remaining * remaining / 1_000_000;
        self.value = self.from + (self.to - self.from) * eased / 1000;
        self.value != self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversal_preserves_current_position() {
        let mut t = Transition::new(false);
        t.retarget(true, 10);
        t.sample(80);
        let current = t.value;
        t.retarget(false, 80);
        assert_eq!(t.value, current);
        assert!(!t.sample(300));
        assert_eq!(t.value, 0);
    }

    #[test]
    fn transition_handles_clock_wrap() {
        let mut t = Transition::new(false);
        t.retarget(true, u32::MAX - 20);
        assert!(!t.sample(220));
        assert_eq!(t.value, 1000);
    }
}
