//! Adaptive dimensions measured in half-apx increments.

/// A logical distance. Values remain exact until resolved against a viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Apx(i32);

impl Apx {
    pub const fn new(units: i16) -> Self {
        Self(units as i32 * 2)
    }

    /// For example, `Apx::from_halves(5)` represents 2.5 apx.
    pub const fn from_halves(halves: i16) -> Self {
        Self(halves as i32)
    }

    pub const fn halves(self) -> i32 {
        self.0
    }
}

/// Pixel size of four apx, taken from the device's design profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApxScale(u8);

impl ApxScale {
    pub const fn new(pixels_per_four_apx: u8) -> Option<Self> {
        if pixels_per_four_apx == 0 {
            None
        } else {
            Some(Self(pixels_per_four_apx))
        }
    }

    /// Exact pixel distance expressed in eighths of a pixel.
    pub const fn eighth_pixels(self, value: Apx) -> i32 {
        value.0 * self.0 as i32
    }

    /// Rounds the final distance to the nearest pixel, away from zero at ties.
    pub const fn px(self, value: Apx) -> i32 {
        let eighths = self.eighth_pixels(value);
        (eighths + 4 * eighths.signum()) / 8
    }
}
