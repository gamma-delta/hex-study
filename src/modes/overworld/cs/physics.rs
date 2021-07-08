use hecs::{Entity, World};
use rapier2d::prelude::*;

use crate::modes::overworld::{
    cs::{
        explosions::{handler_explosion, Explosion},
        player::Player,
    },
    physics::PhysicsWorld,
    WorldExt,
};

/// Normal step time for the world
const STEP_TIME_NORMAL: f32 = 1.0 / 60.0;
/// Step time in bullet time when drawing spells
const STEP_TIME_BULLET: f32 = STEP_TIME_NORMAL * 0.2;

/// Component for entities with a collider.
#[derive(Debug)]
pub struct HasCollider(pub ColliderHandle);

impl std::ops::Deref for HasCollider {
    type Target = ColliderHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Component for entities with a rigid body.
#[derive(Debug)]
pub struct HasRigidBody(pub RigidBodyHandle);

impl std::ops::Deref for HasRigidBody {
    type Target = RigidBodyHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn system_run_physics(world: &mut World, physics: &mut PhysicsWorld) {
    let PhysicsWorld {
        rigid_bodies,
        colliders,
        joints,

        integration_params,
        island_manager,
        broad_phase,
        narrow_phase,
        ccd_solver,

        physics_pipeline,
        query_pipeline,

        elapsed_time,
    } = physics;

    integration_params.dt = if let Some(player_id) = world.get_player() {
        let player = world.get::<Player>(player_id).unwrap();
        if player.wip_spell.is_some() {
            STEP_TIME_BULLET
        } else {
            STEP_TIME_NORMAL
        }
    } else {
        STEP_TIME_NORMAL
    };

    // use channels to get events... why
    let (intersect_tx, intersect_rx) = crossbeam::channel::unbounded();
    let (contact_tx, contact_rx) = crossbeam::channel::unbounded();
    let cec = ChannelEventCollector::new(intersect_tx, contact_tx);

    physics_pipeline.step(
        &vector![0.0, 0.0],
        integration_params,
        island_manager,
        broad_phase,
        narrow_phase,
        rigid_bodies,
        colliders,
        joints,
        ccd_solver,
        &(),
        &cec,
    );
    query_pipeline.update(&island_manager, &rigid_bodies, &colliders);
    *elapsed_time += integration_params.dt;

    // Now do events!
    while let Ok(ev) = intersect_rx.try_recv() {
        let c1 = physics.colliders.get(ev.collider1).unwrap();
        let e1 = Entity::from_bits(c1.user_data as _);
        let c2 = physics.colliders.get(ev.collider2).unwrap();
        let e2 = Entity::from_bits(c2.user_data as _);

        // Explode? check both permutations
        for (e1, e2) in [(e1, e2), (e2, e1)] {
            let e1_explodes = world.get::<Explosion>(e1).is_ok();
            let e2_has_physics =
                world.get::<HasRigidBody>(e2).is_ok() && world.get::<HasCollider>(e2).is_ok();
            if e1_explodes && e2_has_physics {
                handler_explosion(e1, e2, world, physics);
                break;
            }
        }
    }

    // don't care about contacts
    while contact_rx.try_recv().is_ok() {}
}
