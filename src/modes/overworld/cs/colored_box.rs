use hecs::World;
use macroquad::prelude::Color;

use crate::modes::overworld::physics::PhysicsWorld;

/// Component for things I can't be bothered to texture right now.
/// Draws a color over its collider's AABB.
pub struct ColoredBox {
    pub color: Color,
}

pub fn system_draw_colored_boxes(world: &World, physics: &PhysicsWorld) {}
