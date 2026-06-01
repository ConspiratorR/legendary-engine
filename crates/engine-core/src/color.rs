use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// An RGBA color in linear space (0.0–1.0 per channel).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
    /// Alpha (opacity) channel.
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Color {
    /// Create a color from floating-point RGBA components.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque color from floating-point RGB components.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a color from 8-bit RGBA components (0–255).
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Create an opaque color from a 24-bit hex value (e.g. `0xFF8800`).
    pub const fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }

    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const YELLOW: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const CYAN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const MAGENTA: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const ORANGE: Self = Self {
        r: 1.0,
        g: 0.5,
        b: 0.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Return a copy with the alpha channel replaced.
    pub fn with_alpha(self, alpha: f32) -> Self {
        Self { a: alpha, ..self }
    }

    /// Convert to a `[f32; 4]` array.
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to a `[u8; 4]` array (0–255 per channel).
    pub fn to_u8_array(&self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }

    /// Linearly interpolate between `self` and `other` by factor `t` (0.0–1.0).
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Clamp all channels to the 0.0–1.0 range.
    pub fn saturate(&self) -> Self {
        Self {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }

    /// Compute the perceptual grayscale value (ITU-R BT.601 weights).
    pub fn grayscale(&self) -> f32 {
        0.299 * self.r + 0.587 * self.g + 0.114 * self.b
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Color({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }
}

impl Add for Color {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
            a: self.a + other.a,
        }
    }
}

impl Sub for Color {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            r: self.r - other.r,
            g: self.g - other.g,
            b: self.b - other.b,
            a: self.a - other.a,
        }
    }
}

impl Mul<f32> for Color {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            r: self.r * scalar,
            g: self.g * scalar,
            b: self.b * scalar,
            a: self.a * scalar,
        }
    }
}

impl Div<f32> for Color {
    type Output = Self;
    fn div(self, scalar: f32) -> Self {
        Self {
            r: self.r / scalar,
            g: self.g / scalar,
            b: self.b / scalar,
            a: self.a / scalar,
        }
    }
}
