use hecs::World;
use macroquad::prelude::Color;
use nalgebra::{Similarity2, Similarity3};

use crate::modes::overworld::{physics::PhysicsWorld, WorldExt};

use super::{
    particles::ParticleEmitter,
    physics::{HasCollider, HasRigidBody},
};

/// Component for projectiles.
///
/// Because I'm very lazy right now these are just rendered as lines.
#[derive(Debug)]
pub struct Projectile {
    /// Color of the projectile body.
    color: Color,

    /// The entity is removed if it goes below this speed.
    /// Aka, the Speed movie
    min_speed: f32,

    /// If this is Some, then a ParticleEmitter attached to this
    /// has its emission direction updated.
    /// If `true` particles go in the same direction as the velocity;
    /// otherwise they go the opposite direction.
    particles_match_vel: Option<bool>,
}

impl Projectile {
    pub fn new(color: Color, min_speed: f32, particles_match_vel: Option<bool>) -> Self {
        Self {
            color,
            min_speed,
            particles_match_vel,
        }
    }
}

/// Delete projectiles going too slow, and if they have particles
/// update them to shoot alongside the projectile.
pub fn system_projectiles(world: &mut World, physics: &mut PhysicsWorld) {
    let mut remove = Vec::new();
    for (e, (proj, rb_h, particles)) in world
        .query_mut::<(&Projectile, &HasRigidBody, Option<&mut ParticleEmitter>)>()
        .into_iter()
    {
        let rb = physics.rigid_bodies.get(rb_h.0).unwrap();
        let vel = *rb.linvel();

        if vel.magnitude() < proj.min_speed {
            remove.push(e);
        } else if let Some(pmv) = proj.particles_match_vel {
            if let Some(particles) = particles {
                let face_to = vel.normalize() * if pmv { 1.0 } else { -1.0 };
                let cfg = particles.get_config_mut();
                cfg.initial_direction = face_to.into();
            }
        }
    }
    for e in remove {
        world.despawn_with_physics(physics, e).unwrap();
    }
}

pub fn system_draw_projectiles(world: &World, physics: &PhysicsWorld) {
    use macroquad::prelude::*;

    for (e, (projectile, coll_h, rb_h)) in world
        .query::<(&Projectile, &HasCollider, &HasRigidBody)>()
        .into_iter()
    {
        let coll = physics.colliders.get(coll_h.0).unwrap();
        let rb = physics.rigid_bodies.get(rb_h.0).unwrap();

        let pos = coll.compute_aabb().center();

        // Put a streak behind the projectile based on its speed
        let streak = -rb.linvel() / 16.0;
        let streak_end = pos + streak;

        draw_line(
            pos.x,
            pos.y,
            streak_end.x,
            streak_end.y,
            1.0 / 16.0,
            projectile.color,
        );
        draw_circle(pos.x, pos.y, 2.0 / 16.0, projectile.color);
    }
}
