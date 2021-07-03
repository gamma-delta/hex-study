use std::f32::consts::TAU;

use crate::modes::overworld::spells::SpellPrototype;

use super::{patterns::RawPattern, Function, RenderedSpell};

use hecs::Entity;
use hex2d::Angle;
use macroquad::{math::Vec2, prelude::warn};
use paste::paste;
use strum_macros::EnumDiscriminants;

/// Spell data that goes on the stack, either from a pattern
/// or calculated.
///
/// Note that spells themselves are SpellData.
///
/// After all the processing is done, all the `RenderedSpell`s left are executed.
#[derive(Debug, Clone, EnumDiscriminants)]
#[strum_discriminants(name(SpellDataKind))]
pub enum SpellData {
    /// A direction in radians
    Direction(f32),
    /// A position in world space
    Position(Vec2),
    /// Handle to an entity. NOTE: this should only be entities with the Animate component.
    Entity(Entity),
    /// Consumes some things off the stack and pushes others
    Function(Function),
    /// A spell that has its data filled in
    RenderedSpell(RenderedSpell),
    /// A pattern that couldn't be turned into data.
    Junk(RawPattern),
}

macro_rules! unwraps {
    ($variant:path, $taip:ty) => {
        paste! {
            pub fn [<unwrap_ $variant:snake:lower>](self) -> $taip {
                if let Self::$variant(it) = self {
                    it
                } else {
                    panic!(concat!("Could not unwrap {:?} to ", stringify!($variant)), self)
                }
            }
        }
    };
    ($both:ty) => {
        paste! {
            unwraps! {[< $both >], $both}
        }
    };
}

impl SpellData {
    unwraps! {Direction, f32}
    unwraps! {Position, Vec2}
    unwraps! {Entity}
    unwraps! {Function}
    unwraps! {RenderedSpell}
    unwraps! {Junk, RawPattern}
}
