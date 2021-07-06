use hecs::World;

use crate::modes::overworld::{physics::PhysicsWorld, WorldExt};

/// Components for things that expire after a given time.
pub struct LimitedTimeOffer {
    time_left: f32,
}

impl LimitedTimeOffer {
    pub fn new(time_left: f32) -> Self {
        Self { time_left }
    }

    /// Get a reference to the limited time offer's time left.
    pub fn time_left(&self) -> f32 {
        self.time_left
    }

    /// Get a mutable reference to the limited time offer's time left.
    pub fn time_left_mut(&mut self) -> &mut f32 {
        &mut self.time_left
    }
}

/// Tick down times and if there's no time left kill it
pub fn system_cleanup_limited_timers(world: &mut World, physics: &mut PhysicsWorld) {
    let mut remove = Vec::new();

    for (e, lto) in world.query_mut::<&mut LimitedTimeOffer>() {
        lto.time_left -= physics.integration_params.dt;
        if lto.time_left < 0.0 {
            remove.push(e);
        }
    }

    for e in remove {
        world.despawn_with_physics(physics, e).unwrap();
    }
}
