use hecs::World;

use crate::modes::overworld::{
    damage::{DamagePacket, DamageSensitivities},
    physics::PhysicsWorld,
    WorldExt,
};

/// Component for things that can take damage
/// (so, everything hopefully).
#[derive(Debug)]
pub struct Damageable {
    hp: HpStyle,
    /// When the entity runs out of HP it dies.
    /// Use this flag to check for death, not the hp.
    is_dead: bool,
    sensitivities: DamageSensitivities,
}

#[derive(Debug)]
enum HpStyle {
    /// A pool of HP that is depleted incrementally by attacks.
    /// Each attack subtracts from hp.
    /// When it reaches 0, this entity is despawned.
    Pool { hp: u64, max_hp: u64 },
    /// You have to get rid of all the HP at once.
    /// If the damage dealt is less than hp, it stays alive.
    OneShot { max_hp: u64 },
}

impl Damageable {
    pub fn take_damage(&mut self, dmg: DamagePacket) {
        let scale = self.sensitivities.0[dmg.source];
        let amount = (dmg.amount as f32 * scale) as u64;

        match &mut self.hp {
            HpStyle::Pool { hp, max_hp } => {
                *hp = hp.saturating_sub(amount);
                if *hp == 0 {
                    self.is_dead = true;
                }
            }
            HpStyle::OneShot { max_hp } => {
                if amount >= *max_hp {
                    self.is_dead = true;
                }
            }
        }
    }

    /// Am I dead
    pub fn is_dead(&self) -> bool {
        self.is_dead
    }
}

pub fn system_cleanup_dead(world: &mut World, physics: &mut PhysicsWorld) {
    let mut remove = Vec::new();
    for (e, dmg) in world.query_mut::<&Damageable>() {
        if dmg.is_dead() {
            remove.push(e);
        }
    }

    for e in remove {
        world.despawn_with_physics(physics, e).unwrap();
    }
}

/// Component for things that should hurt when they intersect/collide with something else.
pub struct Hurtbox {
    /// Damage that I do
    ouchie: DamagePacket,
    /// Whether this should be removed as soon as it does any damage
    fragile: bool,
}
