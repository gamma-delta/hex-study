//! `macroquad_particles::Emitter` is a component that goes on things.
//! The entity must also have a collider, and the particles will appear
//! around its center.

use std::{borrow::Borrow, cell::RefCell, sync::RwLock};

use hecs::World;
use macroquad_particles::{Emitter, EmitterConfig};
use nalgebra::{Matrix3, Similarity, Similarity2};

use crate::modes::overworld::{physics::PhysicsWorld, WorldExt};

use super::physics::HasCollider;

/// Component for things that emit particles
pub struct ParticleEmitter {
    /// Internal macroquad emitter.
    /// Emitters have to have `&mut self` to draw...
    emitter: RwLock<Emitter>,
    /// If this is `true`, the only purpose of this entity is to spawn particles,
    /// and it should be despawned after it is through with emitting.
    /// Otherwise, just remove the emitter.
    disposable: bool,
}

impl ParticleEmitter {
    pub fn new(config: EmitterConfig, disposable: bool) -> Self {
        Self {
            emitter: RwLock::new(Emitter::new(config)),
            disposable,
        }
    }
}

pub fn system_draw_particles(world: &World, physics: &PhysicsWorld, cam: &Similarity2<f32>) {
    for (e, (emitter, coll_handle)) in world
        .query::<(&ParticleEmitter, &HasCollider)>()
        .into_iter()
    {
        {
            let coll = physics.colliders.get(coll_handle.0).unwrap();
            let center = coll.compute_aabb().center();
            emitter
                .emitter
                .write()
                .unwrap()
                .draw((cam.transform_point(&center)).into());
        }
    }
}

pub fn system_cleanup_particles(world: &mut World, physics: &mut PhysicsWorld) {
    let mut removes = Vec::new();
    for (e, emitter) in world.query_mut::<&ParticleEmitter>().into_iter() {
        let inner = emitter.emitter.read().unwrap();
        if !inner.config.emitting {
            removes.push((e, emitter.disposable));
        }
    }

    for (e, disposable) in removes {
        if disposable {
            world.despawn_with_physics(physics, e).unwrap();
        } else {
            world.remove_one::<ParticleEmitter>(e).unwrap();
        }
    }
}
