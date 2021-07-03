use std::f32::consts::TAU;

use hecs::{Entity, World};
use macroquad::{
    color::{hsl_to_rgb, rgb_to_hsl},
    prelude::{info, vec2, Color},
};
use macroquad_particles::{ColorCurve, EmissionShape, EmitterConfig, ParticleShape};
use rapier2d::prelude::*;

use crate::modes::overworld::{cs::dazing::Dazeable, physics::PhysicsWorld, WorldExt};

use super::{
    particles::ParticleEmitter,
    physics::{HasCollider, HasRigidBody},
};

/// Component for things that are explosions! Boom!
///
/// These have a sensor Collider component associated with them,
/// and the exploding is done in the Intersection Events.
pub struct Explosion {
    /// Strength of the explosion.
    /// Motion is added to other rigid bodies caught in the explosion according to this.
    strength: f32,
    // Color of the exposion
    color: Color,
}

impl Explosion {
    /// Make a new explosion and add it to the world.
    pub fn add(
        pos: Vector<f32>,
        shape: SharedShape,
        strength: f32,
        color: Color,
        world: &mut World,
        physics: &mut PhysicsWorld,
    ) -> Entity {
        let collider = ColliderBuilder::new(shape)
            .position(Isometry::from(pos))
            .sensor(true)
            .active_events(ActiveEvents::INTERSECTION_EVENTS)
            .build();

        world.spawn_with_physics(physics, (Explosion { strength, color },), collider, None)
    }
}

/// Call this when an explosion intersects with something else with a rigidbody.
pub fn handler_explosion(
    explosion_handle: Entity,
    target: Entity,
    world: &mut World,
    physics: &mut PhysicsWorld,
) {
    // make a new scope for borrow checknt
    {
        let explosion = world.get::<Explosion>(explosion_handle).unwrap();
        let explosion_coll_handle = world.get::<HasCollider>(explosion_handle).unwrap();
        let explosion_coll = physics.colliders.get(explosion_coll_handle.0).unwrap();

        let target_coll_handle = world.get::<HasCollider>(target).unwrap();
        let target_coll = physics.colliders.get(target_coll_handle.0).unwrap();
        let target_rb_handle = target_coll.parent().unwrap();
        let target_rb = physics.rigid_bodies.get_mut(target_rb_handle).unwrap();

        let delta = target_coll.compute_aabb().center() - explosion_coll.compute_aabb().center();
        let force = delta.normalize() * explosion.strength;
        info!("Boom, applying {:?} to {:?}", &force, &target);
        target_rb.apply_force(force, true);

        if let Ok(mut damping) = world.get_mut::<Dazeable>(target) {
            damping.add_time(target, (explosion.strength / 1000.0).atan(), world, physics);
        }
    }
}

/// System that despawns explosions and adds a particle emitter.
/// Run this before any explosions are created.
pub fn system_explosions(world: &mut World, physics: &mut PhysicsWorld) {
    let config = EmitterConfig {
        // thanks for this comment fedor
        local_coords: true,
        one_shot: true,
        lifetime: 0.5,
        lifetime_randomness: 0.2,
        explosiveness: 0.8,
        shape: ParticleShape::Circle { subdivisions: 20 },
        initial_direction_spread: TAU,
        initial_velocity_randomness: 3.0,
        linear_accel: -1.0,
        size: 2.0,
        size_randomness: 2.0,
        gravity: vec2(0.0, 0.5),
        ..Default::default()
    };

    let mut add = Vec::new();
    let mut remove = Vec::new();
    for (e, (explosion, coll_handle)) in world.query_mut::<(&Explosion, &HasCollider)>() {
        let coll = physics.colliders.get(coll_handle.0).unwrap();
        let center = coll.compute_aabb().center();

        let color = explosion.color;
        let (h, s, l) = rgb_to_hsl(color);
        let mid = hsl_to_rgb(h + 0.1, s, l + 0.2);
        let mut end = hsl_to_rgb(h - 0.1, s * 0.5, l - 0.2);
        end.a = 0.0;

        let config = EmitterConfig {
            emission_shape: EmissionShape::Sphere {
                radius: explosion.strength.sqrt(),
            },
            amount: explosion.strength.sqrt() as u32,
            initial_velocity: explosion.strength.sqrt() * 10.0,
            colors_curve: ColorCurve {
                start: color,
                mid,
                end,
            },
            ..config.clone()
        };

        add.push((
            ParticleEmitter::new(config, true),
            ColliderBuilder::ball(1.0)
                .translation(center.coords)
                .sensor(true)
                .build(),
        ));

        remove.push(e);
    }

    for (emitter, collider) in add {
        world.spawn_with_physics(physics, (emitter,), collider, None);
    }

    for e in remove {
        world.despawn_with_physics(physics, e).unwrap();
    }
}
