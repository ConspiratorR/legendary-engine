use crate::keyboard::KeyCode;

/// A mapping from physical inputs to an action.
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    /// A single key triggers the action (value 0.0 or 1.0).
    Key(KeyCode),
    /// Two keys form an axis: `positive` yields +1.0, `negative` yields -1.0.
    Axis {
        /// The key that produces a positive axis value.
        positive: KeyCode,
        /// The key that produces a negative axis value.
        negative: KeyCode,
    },
}
