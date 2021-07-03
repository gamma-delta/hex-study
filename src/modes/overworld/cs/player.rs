use std::{f32::consts::TAU as TAU32, mem};

use cogs_gamedev::controls::InputHandler;
use hecs::{With, World};
use hex2d::{Angle, Coordinate, Direction as Direction6};
use macroquad::{
    math::Vec2,
    prelude::{error, info, vec2, SKYBLUE},
};
use quad_rand::compat::QuadRand;
use rand::Rng;
use rapier2d::{na::Vector2, prelude::*};

use crate::{
    controls::{Control, InputSubscriber},
    modes::overworld::{
        cs::{explosions::Explosion, physics::HasRigidBody},
        physics::PhysicsWorld,
        spells::casting::{CastResult, SpellCaster},
        WorldExt,
    },
};

use super::dazing::Dazeable;

mod consts {
    pub const WIDTH: f32 = 0.5;
    pub const HEIGHT: f32 = 0.8;
    pub const DENSITY: f32 = 1.0;

    /// Walk force in squares per second.
    pub const WALK_IMPULSE: f32 = 1.0;
    /// Default damping for the player
    pub const DAMPING: f32 = 20.0;
}

/// Component for things that are the player.
///
/// I also slap lots of singleton info on here...
#[derive(Debug)]
pub struct Player {
    /// This is Some if we are currently drawing a spell.
    pub wip_spell: Option<SpellCaster>,
}

impl Player {
    pub fn new() -> Self {
        Self { wip_spell: None }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

/// Modifies the player's kinematic xor spell info in accordance with the player's inputs.
pub fn system_player_inputs(
    world: &mut World,
    physics: &mut PhysicsWorld,
    controls: &InputSubscriber,
) {
    let player_id = world.get_player();
    let mut player = world.get_mut::<Player>(player_id).unwrap();

    let handle = world.get::<HasRigidBody>(player_id).unwrap().0;
    let body = physics.rigid_bodies.get(handle).unwrap();

    if let Some(wip_spell) = &mut player.wip_spell {
        let cast = wip_spell.update(controls, &world, &physics);
        if !matches!(&cast, &CastResult::NotDone) {
            // we're done here
            player.wip_spell = None;
        }
        drop(player);
        match cast {
            CastResult::NotDone => {}
            CastResult::Success(spell) => {
                info!("Cast a spell! {:#?}", spell);
            }
            CastResult::Mistake => {
                // Make a big explosion a little bit offset from you so you go flying
                let offset_angle = QuadRand.gen_range(0.0..TAU32);
                let pos = body.position().translation.vector;
                let pos = pos + vector![offset_angle.cos() * 0.01, offset_angle.sin() * 0.01];

                Explosion::add(
                    pos,
                    SharedShape::ball(5.0),
                    1_000.0,
                    SKYBLUE,
                    world,
                    physics,
                );
            }
        }
    } else if controls.clicked_down(Control::Click) {
        player.wip_spell = Some(SpellCaster::new(player_id, controls));
    }

    // Do plain ol' motion
    let daze = world.get::<Dazeable>(player_id).unwrap();
    let daze_control_allowed = match daze.time_left() {
        Some(daze) => {
            if daze > 1.5 {
                0.0
            } else {
                1.0 / (1.0 + daze)
            }
        }
        None => 1.0,
    };
    let body = physics.rigid_bodies.get_mut(handle).unwrap();
    // Direction the player is inputting
    let input_vec: Vector2<_> = controls.pressed_vec().into();
    body.apply_impulse(
        input_vec * consts::WALK_IMPULSE * daze_control_allowed,
        true,
    );
}

/// Player body and collider settings.
pub fn player_body_collider() -> (Collider, RigidBody) {
    let collider = ColliderBuilder::cuboid(consts::WIDTH / 2.0, consts::HEIGHT / 2.0)
        .density(consts::DENSITY)
        .build();
    let rb = RigidBodyBuilder::new_dynamic()
        .lock_rotations()
        .linear_damping(consts::DAMPING)
        .build();

    (collider, rb)
}
