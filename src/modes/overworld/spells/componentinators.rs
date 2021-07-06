//! Turn `RenderedSpell`s into entities.

use hecs::World;
use macroquad::prelude::{vec2, PURPLE};
use macroquad_particles::{ColorCurve, Curve, EmitterConfig, ParticleShape};
use rapier2d::prelude::*;

use crate::{
    modes::overworld::{
        cs::{
            limited_time_offer::LimitedTimeOffer, particles::ParticleEmitter, physics::HasCollider,
            projectiles::Projectile,
        },
        physics::PhysicsWorld,
        WorldExt,
    },
    utils::draw::hexcolor,
};

use super::{RenderedSpell, RenderedSpellKind};

impl RenderedSpell {
    /// Add this spell to the world.
    ///
    /// IMPORTANT: There's no guarantee any of the entities in the spell actually exist anymore!
    /// Everything must be fallible.
    pub fn add(self, world: &mut World, physics: &mut PhysicsWorld) {
        match self.kind {
            RenderedSpellKind::Starburst { direction } => {
                if world.contains(self.context.caster) {
                    let parent_coll_h = world.get::<HasCollider>(self.context.caster).unwrap();
                    let parent_coll = physics.colliders.get(parent_coll_h.0).unwrap();

                    let config = EmitterConfig {
                        amount: 30,
                        initial_direction_spread: 1.0,
                        initial_velocity: 3.0,
                        initial_velocity_randomness: 1.0,
                        size: 0.1,
                        size_randomness: 1.0,
                        shape: ParticleShape::Circle { subdivisions: 20 },
                        one_shot: false,
                        local_coords: true,
                        linear_accel: 0.1,
                        colors_curve: ColorCurve {
                            start: hexcolor(0xa565ab_aa),
                            mid: hexcolor(0xa565ab_ff),
                            end: hexcolor(0xd6a95a_00),
                        },
                        lifetime: 0.7,
                        lifetime_randomness: 1.0,
                        ..Default::default()
                    };
                    let particles = ParticleEmitter::new(config, false);

                    let proj = Projectile::new(PURPLE, 0.1, Some(false));

                    let timer = LimitedTimeOffer::new(2.0);

                    let (dy, dx) = direction.sin_cos();
                    let vel = vector![dx, dy] * 12.0; // make up some velocity
                                                      // place this a little out from the caster so as to not hit them
                    let aabb = parent_coll.compute_aabb();
                    let dist = aabb.extents().max() * 1.01;
                    let coll = ColliderBuilder::ball(1.0 / 32.0).density(0.5).build();
                    let rb = RigidBodyBuilder::new_dynamic()
                        .translation(parent_coll.translation() + vel.normalize() * dist)
                        .linvel(vel)
                        .build();

                    drop(parent_coll_h);

                    world.spawn_with_physics(physics, (particles, proj, timer), coll, Some(rb));
                }
            }
            RenderedSpellKind::Shield { pos } => {}
            RenderedSpellKind::Light { pos } => {
                let config = EmitterConfig {
                    amount: 20,
                    initial_direction: vec2(0.0, -1.0),
                    initial_direction_spread: 1.0,
                    initial_velocity: 1.0,
                    initial_velocity_randomness: 0.5,
                    size: 0.15,
                    size_randomness: 0.5,
                    shape: ParticleShape::Circle { subdivisions: 20 },
                    one_shot: false,
                    local_coords: true,
                    linear_accel: -0.05,
                    colors_curve: ColorCurve {
                        start: hexcolor(0xfabc37_55),
                        mid: hexcolor(0xffde38_cc),
                        end: hexcolor(0x383bff_22),
                    },
                    gravity: vec2(0.0, -0.5),
                    explosiveness: 0.1,
                    lifetime: 2.0,
                    size_curve: Some(Curve {
                        points: vec![(0.0, 0.9), (2.0, 2.0), (3.0, 0.1)],
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                let particles = ParticleEmitter::new(config, false);

                let collider = ColliderBuilder::ball(0.1)
                    .translation(pos.into())
                    .collision_groups(InteractionGroups::none())
                    .build();
                world.spawn_with_physics(physics, (particles,), collider, None);
            }
            RenderedSpellKind::Wayfinder { pos, towards } => {
                let (dy, dx) = towards.sin_cos();
                let towards = vec2(dx, dy);
                let config = EmitterConfig {
                    amount: 30,
                    initial_direction: towards,
                    initial_direction_spread: 0.5,
                    gravity: towards * 8.0,
                    initial_velocity: 2.0,
                    initial_velocity_randomness: 0.3,
                    lifetime: 0.8,
                    lifetime_randomness: 1.0,
                    size: 0.2,
                    size_randomness: 1.0,
                    shape: ParticleShape::Circle { subdivisions: 8 },
                    explosiveness: 0.9,
                    one_shot: true,
                    local_coords: true,
                    colors_curve: ColorCurve {
                        start: hexcolor(0xd738ff_22),
                        mid: hexcolor(0x8682ff_dd),
                        end: hexcolor(0x38ebff_22),
                    },
                    size_curve: Some(Curve {
                        points: vec![(0.2, 1.0), (0.7, 2.0), (1.0, 0.0)],
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                let particles = ParticleEmitter::new(config, true);

                let collider = ColliderBuilder::ball(0.1)
                    .translation((pos + towards / 4.0).into())
                    .collision_groups(InteractionGroups::none())
                    .build();
                world.spawn_with_physics(physics, (particles,), collider, None);
            }
        }
    }
}
