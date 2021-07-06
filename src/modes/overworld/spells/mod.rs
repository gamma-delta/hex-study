pub mod casting;
pub mod componentinators;
pub mod data;
pub mod patterns;

use std::convert::TryInto;

use data::{SpellData, SpellDataKind};

use hecs::{Entity, World};
use macroquad::prelude::Vec2;
use smallvec::{smallvec, SmallVec};
use strum_macros::EnumDiscriminants;

use crate::modes::overworld::cs::{physics::HasCollider, shrine::Shrine};

use self::casting::SpellContext;

use super::physics::PhysicsWorld;

/// A mapping between some number of inputs and some outputs.
/// This is used to implement:
/// - Math operations
/// - "Getters" like "get me the caster"
/// - And spells! Yes, spells are `Function`s that push RenderedSpells.
#[derive(Debug, Clone, Copy)]
pub enum Function {
    /// Get the caster
    GetCaster,
    /// Get the position of the given entity. (Specifically the center of its collider's AABB.)
    GetPosition,
    /// If the first argument is null, return the second, otherwise return the first
    CheckNull,
    /// Find the direction from the first position to the second position
    GetDeltaDirection,
    /// Find the closest shrine to the caster
    FindShrine,
    /// Duplicate the top of the stack
    Duplicate,
    /// Swap the top two positions of the stack
    Swap,
    /// All the functions that are spells
    Spell(SpellPrototype),
}

impl Function {
    /// Return the number of arguments this wants.
    pub fn argc(&self) -> usize {
        match self {
            Self::GetCaster => 0,
            Self::GetPosition => 1,
            Self::CheckNull => 2,
            Self::GetDeltaDirection => 2,
            Self::FindShrine => 0,
            Self::Duplicate => 1,
            Self::Swap => 2,
            Self::Spell(spell) => spell.argc(),
        }
    }

    /// Try to execute this.
    ///
    /// Give it the stack up until this function,
    /// with the length of the requested argc.
    /// For example, if the stack is `A B C D` and `D` requests argc of 2, `stack` is `B C`.
    ///
    /// If this returns Some, then it was a success. Push the returned value and
    /// remove the old ones.
    /// Otherwise it was a failure and some sort of magic explosion should occur probably.
    pub fn try_execute(
        self,
        stack: Vec<SpellData>,
        ctx: &mut SpellContext,
        world: &World,
        physics: &PhysicsWorld,
    ) -> Option<SmallVec<[SpellData; 4]>> {
        // It is always OK to unwrap the conversion to the array here, because it's checked to be the right size
        // by the caller.
        fn arr<const N: usize>(stack: Vec<SpellData>) -> [SpellData; N] {
            stack.try_into().unwrap()
        }
        match self {
            Function::GetCaster => {
                // this always succeeds with its 0 argc
                Some(smallvec![SpellData::Entity(ctx.caster)])
            }
            Function::GetPosition => {
                if let [SpellData::Entity(target)] = arr::<1>(stack) {
                    let coll_handle = world.get::<HasCollider>(target).ok()?.0;
                    let collider = physics.colliders.get(coll_handle)?;
                    Some(smallvec![SpellData::Position(
                        collider.compute_aabb().center().into()
                    )])
                } else {
                    None
                }
            }
            Function::CheckNull => {
                let [a, b] = arr::<2>(stack);
                Some(smallvec![if let SpellData::Null(()) = &a { b } else { a }])
            }
            Function::GetDeltaDirection => {
                if let [SpellData::Position(a), SpellData::Position(b)] = arr::<2>(stack) {
                    let delta = b - a;
                    Some(smallvec![if delta.length_squared() < 0.0001 {
                        SpellData::Null(())
                    } else {
                        SpellData::Direction(delta.y.atan2(delta.x))
                    }])
                } else {
                    None
                }
            }
            Function::FindShrine => {
                let coll_h = world.get::<HasCollider>(ctx.caster).ok()?;
                let coll = physics.colliders.get(**coll_h).unwrap();
                let caster_pos = coll.compute_aabb().center();

                let shrine = world
                    .query::<(&Shrine, &HasCollider)>()
                    .into_iter()
                    .map(|(e, (_, coll_h))| {
                        let coll = physics.colliders.get(**coll_h).unwrap();
                        let aabb = coll.compute_aabb();
                        (e, aabb.center())
                    })
                    .min_by(|(_, a), (_, b)| {
                        let dist_a = (a - caster_pos).magnitude_squared();
                        let dist_b = (b - caster_pos).magnitude_squared();
                        // NaN was a mistake
                        dist_a.total_cmp(&dist_b)
                    });
                Some(smallvec![if let Some((shrine, _)) = shrine {
                    SpellData::Entity(shrine)
                } else {
                    SpellData::Null(())
                }])
            }
            Function::Duplicate => {
                let [it] = arr::<1>(stack);
                Some(smallvec![it; 2])
            }
            Function::Swap => {
                let [a, b] = arr::<2>(stack);
                Some(smallvec![b, a])
            }
            Function::Spell(proto) => {
                // Pass this down to the spell prototype
                SpellPrototype::try_render(proto, stack, ctx.clone())
                    .map(|it| smallvec![SpellData::RenderedSpell(it)])
            }
        }
    }
}

/// A spell with all the data filled in
#[derive(Debug, Clone)]
pub struct RenderedSpell {
    /// The kind of spell (prototype + data)
    kind: RenderedSpellKind,
    /// Context this spell was cast in
    context: SpellContext,
}

#[derive(Debug, Clone, EnumDiscriminants)]
#[strum_discriminants(name(SpellPrototype))]
pub enum RenderedSpellKind {
    /// A small projectile like Spark Bolt, shot from the caster in the specified direction.
    Starburst { direction: f32 },
    /// Produces a shield around the given point.
    Shield { pos: Vec2 },
    /// A little light of mine (I'm gonna let it shine)
    Light { pos: Vec2 },
    /// Short particle effect that points in a direction
    Wayfinder { pos: Vec2, towards: f32 },
}

macro_rules! unwrap_arms {
    (
        $spell:expr,
        $data:expr,
        $(
            ($proto:ident => $count:expr; $($name:ident : $datatype:path),* $(,)*)
        ),*
    ) => {
        match $spell {
            $(
                SpellPrototype::$proto => {
                    if let Result::<[SpellData; $count], _>::Ok([
                        $(
                            $datatype($name),
                        )*
                    ]) = $data.try_into() {
                        Some(RenderedSpellKind::$proto {
                            $(
                                $name,
                            )*
                        })
                    } else {
                        None
                    }
                }
            )*
        }
    };
}

impl RenderedSpellKind {
    /// Given an argument list of SpellData, try to convert it to a RenderedSpellKind
    pub fn try_render(proto: SpellPrototype, data: Vec<SpellData>) -> Option<RenderedSpellKind> {
        // just think about all the seconds i saved in the hour i spent writing this mess
        // to use: each paren group is:
        //   name of spell => argc
        //   argname : arg type
        unwrap_arms! {
            proto,
            data,
            (
                Starburst => 1;
                direction: SpellData::Direction,
            ),
            (
                Shield => 1;
                pos: SpellData::Position,
            ),
            (
                Light => 1;
                pos: SpellData::Position,
            ),
            (
                Wayfinder => 2;
                pos: SpellData::Position,
                towards: SpellData::Direction,
            )
        }
    }
}

impl SpellPrototype {
    /// Return the number of input arguments this one wants.
    pub fn argc(&self) -> usize {
        match self {
            SpellPrototype::Starburst => 1,
            SpellPrototype::Shield => 1,
            SpellPrototype::Light => 1,
            SpellPrototype::Wayfinder => 2,
        }
    }

    /// Try to render this to a RenderedSpell.
    /// Give this the owned SpellContext, so we have a "snapshot" of when it was cast.
    pub fn try_render(
        proto: SpellPrototype,
        stack: Vec<SpellData>,
        ctx: SpellContext,
    ) -> Option<RenderedSpell> {
        RenderedSpellKind::try_render(proto, stack).map(|kind| RenderedSpell { context: ctx, kind })
    }
}

impl From<SpellPrototype> for SpellData {
    fn from(val: SpellPrototype) -> Self {
        SpellData::Function(Function::Spell(val))
    }
}
