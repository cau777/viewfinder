use nalgebra::Vector2;

/// Avoid mixing up coordinates and relative positions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinates(pub Vector2<f64>);
