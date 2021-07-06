use hecs::World;
use macroquad::prelude::{draw_rectangle, Color};
use rapier2d::prelude::Collider;

use crate::modes::overworld::{physics::PhysicsWorld, WorldExt};

use super::physics::HasCollider;

/// Component for things I can't be bothered to texture right now.
/// Draws a color over its collider's AABB.
#[derive(Debug)]
pub struct ColoredBox(pub Color);

pub fn system_draw_colored_boxes(world: &World, physics: &PhysicsWorld) {
    fn draw(coll: &Collider, color: Color) {
        let aabb = coll.compute_aabb();
        let mins = aabb.mins;
        let maxes = aabb.maxs;
        let size = maxes - mins;

        draw_rectangle(mins.x, mins.y, size.x, size.y, color);
    }

    for (_, (color, coll_h)) in world.query::<(&ColoredBox, &HasCollider)>().into_iter() {
        let coll = physics.colliders.get(**coll_h).unwrap();
        draw(coll, color.0);
    }

    let player_h = world.get_player();
    // so ergonomic
    let mut query = world
        .query_one::<(&ColoredBox, &HasCollider)>(player_h)
        .unwrap();
    let (color, coll_h) = query.get().unwrap();
    let coll = physics.colliders.get(**coll_h).unwrap();
    draw(coll, color.0);
}
