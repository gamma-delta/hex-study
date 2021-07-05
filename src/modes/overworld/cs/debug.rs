use hecs::World;

use crate::modes::overworld::{
    cs::physics::{HasCollider, HasRigidBody},
    physics::PhysicsWorld,
    WorldExt,
};

/// Draw rectangles around collision AABBs.
pub fn system_draw_collision(world: &World, physics: &PhysicsWorld) {
    use macroquad::prelude::*;

    let player_id = world.get_player();

    for (e, (HasCollider(coll_handle), rb_handle)) in
        world.query::<(&_, Option<&HasRigidBody>)>().iter()
    {
        let coll = physics.colliders.get(*coll_handle).unwrap();
        let aabb = coll.compute_aabb();
        let mins = aabb.mins;
        let maxes = aabb.maxs;
        let size = maxes - mins;

        // Always draw a gray background for the collider
        draw_rectangle(
            mins.x,
            mins.y,
            size.x,
            size.y,
            Color::new(1.0, 1.0, 1.0, 0.5),
        );

        let outline = if rb_handle.is_none() {
            BLANK
        } else if e == player_id {
            ORANGE
        } else {
            WHITE
        };

        // everything is in physics space right now, so we need to draw
        // 1/16 = 1 pixel
        draw_rectangle_lines(mins.x, mins.y, size.x, size.y, 2.0 / 16.0, outline);
    }
}
