//! Default typography is expressed in apx and resolved by the current viewport.

use crate::{geometry::Viewport, Apx};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSize {
    #[default]
    Body,
    Heading,
    Display,
    LargeTitle,
    Number,
}

impl TextSize {
    pub const fn apx(self) -> Apx {
        match self {
            Self::Body => Apx::new(4),
            Self::Heading => Apx::from_halves(9),
            Self::Display => Apx::new(5),
            Self::LargeTitle => Apx::new(7),
            Self::Number => Apx::from_halves(13),
        }
    }

    pub const fn pixels(self, viewport: Viewport) -> i32 {
        viewport.apx(self.apx())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::DisplayProfile;

    #[test]
    fn default_type_sizes_follow_the_three_device_variable_sets() {
        for (profile, expected) in [
            (DisplayProfile::BAND_10_PRO, [25, 28, 31, 44, 41]),
            (DisplayProfile::BAND_11, [22, 25, 28, 39, 36]),
            (DisplayProfile::REDMI_WATCH, [30, 34, 38, 53, 49]),
        ] {
            let sizes = [
                TextSize::Body,
                TextSize::Heading,
                TextSize::Display,
                TextSize::LargeTitle,
                TextSize::Number,
            ];
            assert_eq!(sizes.map(|size| size.pixels(profile.viewport)), expected);
        }
    }
}
