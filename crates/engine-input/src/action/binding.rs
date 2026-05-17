use crate::keyboard::KeyCode;

#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    Key(KeyCode),
    Axis {
        positive: KeyCode,
        negative: KeyCode,
    },
}
