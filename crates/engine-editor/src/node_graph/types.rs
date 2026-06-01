use egui::Color32;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Pin data types for the node graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PinType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Color,
    Texture,
    Sampler,
    Bool,
    Int,
}

impl PinType {
    /// Display name for the pin type.
    pub fn display_name(&self) -> &'static str {
        match self {
            PinType::Float => "Float",
            PinType::Vec2 => "Vec2",
            PinType::Vec3 => "Vec3",
            PinType::Vec4 => "Vec4",
            PinType::Color => "Color",
            PinType::Texture => "Texture",
            PinType::Sampler => "Sampler",
            PinType::Bool => "Bool",
            PinType::Int => "Int",
        }
    }

    /// Color used for rendering pins in the node graph.
    pub fn color(&self) -> Color32 {
        match self {
            PinType::Float => Color32::from_rgb(100, 200, 100),
            PinType::Vec2 => Color32::from_rgb(100, 150, 255),
            PinType::Vec3 => Color32::from_rgb(255, 100, 100),
            PinType::Vec4 => Color32::from_rgb(200, 150, 255),
            PinType::Color => Color32::from_rgb(255, 200, 50),
            PinType::Texture => Color32::from_rgb(200, 100, 50),
            PinType::Sampler => Color32::from_rgb(150, 100, 50),
            PinType::Bool => Color32::from_rgb(255, 150, 150),
            PinType::Int => Color32::from_rgb(100, 200, 200),
        }
    }

    /// Check if this pin type can connect to another pin type.
    pub fn is_compatible_with(&self, other: &PinType) -> bool {
        if self == other {
            return true;
        }
        // Allow implicit conversions between numeric types
        matches!(
            (self, other),
            (PinType::Float, PinType::Int)
                | (PinType::Int, PinType::Float)
                | (PinType::Float, PinType::Vec2)
                | (PinType::Vec2, PinType::Float)
        )
    }
}

impl fmt::Display for PinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Runtime values that flow through node connections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color([f32; 4]),
    Bool(bool),
    Int(i32),
}

impl NodeValue {
    /// Get the pin type corresponding to this value.
    pub fn pin_type(&self) -> PinType {
        match self {
            NodeValue::Float(_) => PinType::Float,
            NodeValue::Vec2(_) => PinType::Vec2,
            NodeValue::Vec3(_) => PinType::Vec3,
            NodeValue::Vec4(_) => PinType::Vec4,
            NodeValue::Color(_) => PinType::Color,
            NodeValue::Bool(_) => PinType::Bool,
            NodeValue::Int(_) => PinType::Int,
        }
    }

    /// Convert value to float (for arithmetic operations).
    pub fn to_float(&self) -> f32 {
        match self {
            NodeValue::Float(v) => *v,
            NodeValue::Int(v) => *v as f32,
            NodeValue::Bool(v) if *v => 1.0,
            NodeValue::Bool(_) => 0.0,
            _ => 0.0,
        }
    }

    /// Convert value to vec4 (for color/position operations).
    pub fn to_vec4(&self) -> [f32; 4] {
        match self {
            NodeValue::Float(v) => [*v, *v, *v, *v],
            NodeValue::Vec2(v) => [v[0], v[1], 0.0, 0.0],
            NodeValue::Vec3(v) => [v[0], v[1], v[2], 0.0],
            NodeValue::Vec4(v) => *v,
            NodeValue::Color(v) => *v,
            NodeValue::Bool(v) => {
                let f = if *v { 1.0 } else { 0.0 };
                [f, f, f, f]
            }
            NodeValue::Int(v) => {
                let f = *v as f32;
                [f, f, f, f]
            }
        }
    }

    /// Default value for a given pin type.
    pub fn default_for_type(pin_type: PinType) -> Self {
        match pin_type {
            PinType::Float => NodeValue::Float(0.0),
            PinType::Vec2 => NodeValue::Vec2([0.0, 0.0]),
            PinType::Vec3 => NodeValue::Vec3([0.0, 0.0, 0.0]),
            PinType::Vec4 => NodeValue::Vec4([0.0, 0.0, 0.0, 0.0]),
            PinType::Color => NodeValue::Color([1.0, 1.0, 1.0, 1.0]),
            PinType::Bool => NodeValue::Bool(false),
            PinType::Int => NodeValue::Int(0),
            PinType::Texture | PinType::Sampler => NodeValue::Float(0.0),
        }
    }
}

impl fmt::Display for NodeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeValue::Float(v) => write!(f, "{:.2}", v),
            NodeValue::Vec2(v) => write!(f, "({:.2}, {:.2})", v[0], v[1]),
            NodeValue::Vec3(v) => write!(f, "({:.2}, {:.2}, {:.2})", v[0], v[1], v[2]),
            NodeValue::Vec4(v) => {
                write!(f, "({:.2}, {:.2}, {:.2}, {:.2})", v[0], v[1], v[2], v[3])
            }
            NodeValue::Color(v) => {
                write!(
                    f,
                    "rgba({:.2}, {:.2}, {:.2}, {:.2})",
                    v[0], v[1], v[2], v[3]
                )
            }
            NodeValue::Bool(v) => write!(f, "{}", v),
            NodeValue::Int(v) => write!(f, "{}", v),
        }
    }
}

/// Direction of a pin (input or output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PinDirection {
    Input,
    Output,
}

/// Unique identifier for a node.
pub type NodeId = u64;

/// Unique identifier for a pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinId {
    pub node_id: NodeId,
    pub index: usize,
}

impl PinId {
    pub fn new(node_id: NodeId, index: usize) -> Self {
        Self { node_id, index }
    }
}

/// A connection between two pins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub output_pin: PinId,
    pub input_pin: PinId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_type_compatibility() {
        assert!(PinType::Float.is_compatible_with(&PinType::Float));
        assert!(PinType::Float.is_compatible_with(&PinType::Int));
        assert!(PinType::Int.is_compatible_with(&PinType::Float));
        assert!(!PinType::Float.is_compatible_with(&PinType::Texture));
        assert!(!PinType::Vec3.is_compatible_with(&PinType::Bool));
    }

    #[test]
    fn test_node_value_default_for_type() {
        assert_eq!(
            NodeValue::default_for_type(PinType::Float),
            NodeValue::Float(0.0)
        );
        assert_eq!(
            NodeValue::default_for_type(PinType::Vec3),
            NodeValue::Vec3([0.0, 0.0, 0.0])
        );
        assert_eq!(
            NodeValue::default_for_type(PinType::Color),
            NodeValue::Color([1.0, 1.0, 1.0, 1.0])
        );
    }

    #[test]
    fn test_node_value_to_float() {
        assert_eq!(NodeValue::Float(3.5).to_float(), 3.5);
        assert_eq!(NodeValue::Int(42).to_float(), 42.0);
        assert_eq!(NodeValue::Bool(true).to_float(), 1.0);
        assert_eq!(NodeValue::Bool(false).to_float(), 0.0);
    }

    #[test]
    fn test_node_value_to_vec4() {
        assert_eq!(NodeValue::Float(2.0).to_vec4(), [2.0, 2.0, 2.0, 2.0]);
        assert_eq!(
            NodeValue::Vec3([1.0, 2.0, 3.0]).to_vec4(),
            [1.0, 2.0, 3.0, 0.0]
        );
    }

    #[test]
    fn test_pin_type_color() {
        let color = PinType::Float.color();
        assert_eq!(color, Color32::from_rgb(100, 200, 100));
    }

    #[test]
    fn test_pin_id_creation() {
        let pin = PinId::new(5, 2);
        assert_eq!(pin.node_id, 5);
        assert_eq!(pin.index, 2);
    }
}
