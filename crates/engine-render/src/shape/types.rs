#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FillMode {
    Solid(Color),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

impl Stroke {
    pub fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShapeCommand {
    Rect {
        position: [f32; 2],
        size: [f32; 2],
        fill: FillMode,
        stroke: Option<Stroke>,
        corner_radius: f32,
    },
    Circle {
        center: [f32; 2],
        radius: f32,
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    Ellipse {
        center: [f32; 2],
        radii: [f32; 2],
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    RoundedRectangle {
        position: [f32; 2],
        size: [f32; 2],
        corner_radius: [f32; 4],
        fill: FillMode,
        stroke: Option<Stroke>,
    },
    Line {
        start: [f32; 2],
        end: [f32; 2],
        color: Color,
        width: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_array() {
        let c = Color::new(0.5, 0.6, 0.7, 0.8);
        assert_eq!(c.to_array(), [0.5, 0.6, 0.7, 0.8]);
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::WHITE.to_array(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(Color::BLACK.to_array(), [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_shape_command_clone() {
        let cmd = ShapeCommand::Rect {
            position: [10.0, 20.0],
            size: [100.0, 50.0],
            fill: FillMode::Solid(Color::RED),
            stroke: Some(Stroke::new(Color::WHITE, 2.0)),
            corner_radius: 0.0,
        };
        let cmd2 = cmd.clone();
        assert_eq!(cmd, cmd2);
    }
}
