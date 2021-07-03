pub mod casting;
pub mod data;
pub mod patterns;

use std::convert::TryInto;

use data::{SpellData, SpellDataKind};

use hecs::{Entity, World};
use macroquad::prelude::Vec2;
use strum_macros::EnumDiscriminants;

use crate::modes::overworld::cs::physics::HasCollider;

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
    /// All the functions that are spells
    Spell(SpellPrototype),
}

impl Function {
    /// Return the number of arguments this wants.
    pub fn argc(&self) -> usize {
        match self {
            Self::GetCaster => 0,
            Self::GetPosition => 1,
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
    ) -> Option<SpellData> {
        // It is always OK to unwrap the conversion to the array here, because it's checked to be the right size
        // by the caller.
        fn arr<const N: usize>(stack: Vec<SpellData>) -> [SpellData; N] {
            stack.try_into().unwrap()
        }
        match self {
            Function::GetCaster => {
                // this always succeeds with its 0 argc
                Some(SpellData::Entity(ctx.caster))
            }
            Function::GetPosition => {
                if let [SpellData::Entity(target)] = arr::<1>(stack) {
                    let coll_handle = world.get::<HasCollider>(target).ok()?.0;
                    let collider = physics.colliders.get(coll_handle)?;
                    Some(SpellData::Position(collider.compute_aabb().center().into()))
                } else {
                    None
                }
            }
            Function::Spell(proto) => {
                // Pass this down to the spell prototype
                SpellPrototype::try_render(proto, stack, ctx.clone()).map(SpellData::RenderedSpell)
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
    /// Adds motion to the given entity.
    Yank { yankee: Entity, dir: f32 },
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
                Yank => 2;
                yankee: SpellData::Entity,
                dir: SpellData::Direction,
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
            SpellPrototype::Yank => 2,
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
