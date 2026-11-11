//! 比例数字布局与滚动动画，供渲染端按帧采样。

#![no_std]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Clone, Copy)]
pub struct Metrics {
    pub digits: [u16; 10],
    pub colon: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub byte: u8,
    pub center_x: i32,
    pub y: i32,
    pub opacity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame<const N: usize> {
    pub outgoing: [Option<Glyph>; N],
    pub current: [Glyph; N],
    pub width: i32,
}

#[derive(Clone, Copy)]
struct Layout<const N: usize> {
    bytes: [u8; N],
    centers: [i32; N],
    width: i32,
}

impl<const N: usize> Layout<N> {
    fn new(bytes: [u8; N], metrics: Metrics) -> Option<Self> {
        let mut centers = [0; N];
        let mut width = 0i32;
        for (index, &byte) in bytes.iter().enumerate() {
            let advance = i32::from(match byte {
                b'0'..=b'9' => metrics.digits[usize::from(byte - b'0')],
                b':' => metrics.colon,
                _ => return None,
            });
            width = width.checked_add(advance)?;
            centers[index] = width - (advance + 1) / 2;
        }
        Some(Self {
            bytes,
            centers,
            width,
        })
    }
}

pub struct RollingDigits<const N: usize> {
    metrics: Metrics,
    from: Layout<N>,
    to: Layout<N>,
    pending: Option<(Layout<N>, Direction)>,
    direction: Direction,
    started: Option<u32>,
    duration_ms: u16,
    travel_px: u16,
}

impl<const N: usize> RollingDigits<N> {
    /// 初始值由 ASCII 数字和冒号组成，字宽以像素为单位。
    pub fn new(value: [u8; N], metrics: Metrics, duration_ms: u16, travel_px: u16) -> Option<Self> {
        let layout = Layout::new(value, metrics)?;
        Some(Self {
            metrics,
            from: layout,
            to: layout,
            pending: None,
            direction: Direction::Up,
            started: None,
            duration_ms,
            travel_px,
        })
    }

    /// 保持格式中的冒号索引不变。动画进行时只保留最新目标。
    pub fn set(&mut self, value: [u8; N], direction: Direction, now_ms: u32) -> bool {
        let Some(layout) = self.layout_for(value) else {
            return false;
        };
        if self.started.is_some() {
            self.pending = (value != self.to.bytes).then_some((layout, direction));
            self.advance(now_ms);
        } else {
            self.start(layout, direction, now_ms);
        }
        true
    }

    /// 直接显示新值并清除当前动画及待播目标。无效值不改变状态。
    pub fn set_immediate(&mut self, value: [u8; N]) -> bool {
        let Some(layout) = self.layout_for(value) else {
            return false;
        };
        self.pending = None;
        self.to = layout;
        self.finish();
        true
    }

    pub fn is_animating(&self) -> bool {
        self.started.is_some()
    }

    /// 时间戳使用可回绕的单调毫秒计数。
    pub fn sample(&mut self, now_ms: u32) -> Frame<N> {
        self.advance(now_ms);
        let Some(start) = self.started else {
            return Frame {
                outgoing: [None; N],
                current: core::array::from_fn(|index| Glyph {
                    byte: self.to.bytes[index],
                    center_x: self.to.centers[index],
                    y: 0,
                    opacity: 255,
                }),
                width: self.to.width,
            };
        };
        let remaining =
            1000 - (now_ms.wrapping_sub(start) * 1000 / u32::from(self.duration_ms)) as i32;
        let progress = 1000 - remaining * remaining * remaining / 1_000_000;
        let opacity = interpolate(0, 255, progress) as u8;
        let travel = i32::from(self.travel_px)
            * if self.direction == Direction::Up {
                -1
            } else {
                1
            };
        let mut outgoing = [None; N];
        let current = core::array::from_fn(|index| {
            let x = interpolate(self.from.centers[index], self.to.centers[index], progress);
            let byte = self.to.bytes[index];
            let changed = self.from.bytes[index] != byte;
            if changed && opacity < 255 {
                outgoing[index] = Some(Glyph {
                    byte: self.from.bytes[index],
                    center_x: x,
                    y: travel * progress / 1000,
                    opacity: 255 - opacity,
                });
            }
            Glyph {
                byte,
                center_x: x,
                y: if changed {
                    travel * (progress - 1000) / 1000
                } else {
                    0
                },
                opacity: if changed { opacity } else { 255 },
            }
        });
        Frame {
            outgoing,
            current,
            width: interpolate(self.from.width, self.to.width, progress),
        }
    }

    /// 跳到最新目标并结束动画。
    pub fn finish(&mut self) {
        if let Some((layout, _)) = self.pending.take() {
            self.to = layout;
        }
        self.from = self.to;
        self.started = None;
    }

    fn layout_for(&self, value: [u8; N]) -> Option<Layout<N>> {
        if value
            .iter()
            .zip(self.to.bytes)
            .any(|(&new, old)| (new == b':') != (old == b':'))
        {
            return None;
        }
        Layout::new(value, self.metrics)
    }

    fn start(&mut self, layout: Layout<N>, direction: Direction, now_ms: u32) {
        self.from = self.to;
        self.to = layout;
        self.direction = direction;
        if self.duration_ms > 0 && self.from.bytes != self.to.bytes {
            self.started = Some(now_ms);
        } else {
            self.finish();
        }
    }

    fn advance(&mut self, now_ms: u32) {
        if let Some(start) = self.started {
            if now_ms.wrapping_sub(start) >= u32::from(self.duration_ms) {
                self.started = None;
                if let Some((layout, direction)) = self.pending.take() {
                    self.start(layout, direction, now_ms);
                } else {
                    self.finish();
                }
            }
        }
    }
}

fn interpolate(from: i32, to: i32, progress: i32) -> i32 {
    from + ((i64::from(to) - i64::from(from)) * i64::from(progress) / 1000) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    const METRICS: Metrics = Metrics {
        digits: [18, 12, 17, 17, 18, 17, 18, 16, 18, 18],
        colon: 8,
    };

    #[test]
    fn width_and_following_glyphs_move_with_the_roll() {
        let mut digits = RollingDigits::new(*b"1:0", METRICS, 240, 14).unwrap();
        let first = digits.sample(0);
        assert!(digits.set(*b"0:0", Direction::Down, 0));
        let middle = digits.sample(120);
        let last = digits.sample(240);

        assert!(first.width < middle.width && middle.width < last.width);
        assert!(middle.current[1].center_x > first.current[1].center_x);
        assert_eq!(middle.current[1].y, 0);
        assert!(middle.outgoing[0].unwrap().y > 0);
        assert!(middle.current[0].y < 0);
        assert_eq!(last.outgoing, [None; 3]);
        assert!(!digits.is_animating());
    }

    #[test]
    fn keeps_latest_update_and_handles_clock_wrap() {
        let mut digits = RollingDigits::new(*b"0", METRICS, 100, 14).unwrap();
        digits.set(*b"1", Direction::Up, u32::MAX - 49);
        digits.set(*b"2", Direction::Up, 0);
        digits.set(*b"3", Direction::Up, 50);
        let frame = digits.sample(50);

        assert_eq!(frame.outgoing[0].unwrap().byte, b'1');
        assert_eq!(frame.current[0].byte, b'3');
        assert_eq!(digits.sample(150).outgoing, [None]);
    }

    #[test]
    fn rejects_invalid_characters_and_separator_changes() {
        let mut digits = RollingDigits::new(*b"1:0", METRICS, 240, 14).unwrap();
        assert!(!digits.set(*b":10", Direction::Down, 0));
        assert!(!digits.set(*b"1:x", Direction::Down, 0));
        assert_eq!(digits.sample(0).current.map(|glyph| glyph.byte), *b"1:0");
    }

    #[test]
    fn immediate_update_replaces_active_and_pending_values() {
        let mut digits = RollingDigits::new(*b"1:0", METRICS, 240, 14).unwrap();
        digits.set(*b"1:1", Direction::Down, 0);
        digits.set(*b"1:2", Direction::Down, 50);
        assert!(digits.set_immediate(*b"0:3"));
        let expected = RollingDigits::new(*b"0:3", METRICS, 240, 14)
            .unwrap()
            .sample(0);
        assert_eq!(digits.sample(50), expected);
        assert_eq!(digits.sample(300), expected);
        assert!(!digits.is_animating());

        digits.set(*b"0:4", Direction::Down, 300);
        assert_eq!(digits.sample(300).outgoing[2].unwrap().byte, b'3');
    }

    #[test]
    fn invalid_immediate_update_keeps_active_and_pending_values() {
        let mut digits = RollingDigits::new(*b"1:0", METRICS, 240, 14).unwrap();
        digits.set(*b"1:1", Direction::Down, 0);
        digits.set(*b"1:2", Direction::Down, 50);
        let frame = digits.sample(50);
        assert!(!digits.set_immediate(*b":10"));
        assert!(!digits.set_immediate(*b"1:x"));
        assert!(digits.is_animating());
        assert_eq!(digits.sample(50), frame);
        assert_eq!(digits.sample(240).current[2].byte, b'2');
    }
}
