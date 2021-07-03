use hecs::{Entity, World};

use crate::modes::overworld::physics::PhysicsWorld;

use super::physics::{HasCollider, HasRigidBody};

/// Components for things that can be dazed / prone / blown into the air.
/// While dazed:
///
/// - Damping is greatly reduced
/// - Player input is less effective
///
/// Explosions and damage make things dazed.
///
/// Note changing the damping while something is dazed won't actually update it.
pub struct Dazeable {
    /// Time till we're undazed. If this is Some we're dazed,
    /// otherwise we're not.
    ///
    /// The first number is the time we are dazed left,
    /// and the second one is the original damping.
    time_left: Option<(f32, f32)>,
}

impl Dazeable {
    pub fn new() -> Self {
        Self { time_left: None }
    }

    /// Add daze time to an entity.
    pub fn add_time(&mut self, entity: Entity, time: f32, world: &World, physics: &PhysicsWorld) {
        let time_left = if let Some((time_left, _)) = &mut self.time_left {
            time_left
        } else {
            // We need to gather the original damping
            let damp = if let Ok(rb_handle) = world.get::<HasRigidBody>(entity) {
                let rb = physics.rigid_bodies.get(rb_handle.0).unwrap();
                rb.linear_damping()
            } else {
                // doesn't matter
                0.0
            };
            &mut self.time_left.insert((0.0, damp)).0
        };
        *time_left += time;
    }

    /// Return how much longer we're dazed, or None if we're not
    pub fn time_left(&self) -> Option<f32> {
        self.time_left.map(|x| x.0)
    }
}

pub fn system_dazed(world: &mut World, physics: &mut PhysicsWorld) {
    for (e, (dazed, rb_handle)) in world.query_mut::<(&mut Dazeable, &HasRigidBody)>() {
        let rb = physics.rigid_bodies.get_mut(rb_handle.0).unwrap();
        if let Some((time_left, prev_damping)) = &mut dazed.time_left {
            let damp = *prev_damping / (1.0 + *time_left * DAMP_TIME_COEFFICIENT);
            rb.set_linear_damping(damp);

            *time_left -= physics.integration_params.dt;
            if *time_left < 0.0 {
                // ok time to quit!
                rb.set_linear_damping(*prev_damping);
                dazed.time_left = None;
            }
        }
    }
}

const DAMP_TIME_COEFFICIENT: f32 = 4.0;
