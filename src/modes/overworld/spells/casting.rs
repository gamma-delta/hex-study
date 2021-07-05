use std::{f32::consts::TAU, mem};

use hecs::{Entity, World};
// haha DirectionN go brrrrr
use cogs_gamedev::controls::InputHandler;
use hex2d::{Angle, Coordinate as HexCoord, Direction as Direction6};
use macroquad::prelude::{info, Vec2};

use crate::{
    controls::{Control, InputSubscriber},
    modes::overworld::physics::PhysicsWorld,
};

use super::{
    data::SpellData,
    patterns::{RawPattern, NEW_DRAW_TOLERANCE},
    RenderedSpell, RenderedSpellKind,
};

/// Struct attached as a field on Player when we're drawing spells.
#[derive(Debug)]
pub struct SpellCaster {
    /// Patterns that have been drawn so far
    patterns: Vec<RawPattern>,
    /// The stack of data we're working on
    stack: Vec<SpellData>,
    /// Current spell context
    context: SpellContext,
    /// How we're drawing
    state: PatternDrawState,
}

impl SpellCaster {
    /// Create a new spellcaster when the player first clicks
    pub fn new(player: Entity, controls: &InputSubscriber) -> Self {
        Self {
            patterns: Vec::new(),
            stack: Vec::new(),
            context: SpellContext { caster: player },
            state: PatternDrawState::new_drawing(controls),
        }
    }

    /// Update this based on the player's controls.
    ///
    /// When this stops returning `NotDone`, we're done editing the spell.
    /// Remove the caster and start putting the things in the world.
    /// The caster may be in an unusable state afterwards.
    pub fn update(
        &mut self,
        controls: &InputSubscriber,
        world: &World,
        physics: &PhysicsWorld,
    ) -> CastResult {
        match &mut self.state {
            PatternDrawState::Waiting => {
                if controls.clicked_down(Control::Click) {
                    // start a new spell!
                    self.state = PatternDrawState::new_drawing(controls);
                }
                CastResult::NotDone
            }
            PatternDrawState::Drawing {
                wip_pattern,
                mouse_origin,
            } => {
                if controls.pressed(Control::Click) {
                    let mouse_pos = controls.mouse_pos();
                    let dmouse = mouse_pos - *mouse_origin;
                    if dmouse.length_squared() >= NEW_DRAW_TOLERANCE.powi(2) {
                        // oh boy here I go adding new directions again
                        let angle = Vec2::X.angle_between(dmouse);
                        // We need to divide by tau/6, but 0 angle is halfway between two sides...
                        // so we "rotate" it down 1/12 of a circle so 0 lies directly on the bottom
                        // of sector #0.
                        let tilted = angle - TAU / 12.0;
                        // invert because graphics convention vs trig convention
                        let sector_idx = tilted / (TAU / 6.0);
                        let dir = Direction6::from_int(sector_idx.floor() as i32);

                        let dir = dir + Angle::Back;

                        let success = if let Some((pattern, prev_dir)) = wip_pattern {
                            // push a new angle to the pattern
                            // make sure we aren't going backwards
                            let angle = dir - *prev_dir;
                            if angle != Angle::Back {
                                pattern.deltas.push(angle);
                                *prev_dir = dir;
                                true
                            } else {
                                false
                            }
                        } else {
                            // oooh time to start a brand new one!
                            *wip_pattern = Some((
                                RawPattern {
                                    deltas: Vec::new(),
                                    first_direction: dir,
                                },
                                dir,
                            ));
                            true
                        };

                        if success {
                            *mouse_origin = mouse_pos;
                        }
                    }
                    // Still working on drawing
                    CastResult::NotDone
                } else {
                    // we stopped drawing the spell
                    // do this long-form unwrap...
                    let wip_pattern = match mem::replace(&mut self.state, PatternDrawState::Waiting)
                    {
                        PatternDrawState::Drawing { wip_pattern, .. } => wip_pattern,
                        _ => unreachable!(),
                    };
                    if let Some((pattern, _)) = wip_pattern {
                        info!("{:#?}", &pattern);
                        self.add_pattern(pattern, world, physics)
                    } else {
                        CastResult::NotDone
                    }
                }
            }
        }
    }

    /// Add a new freshly drawn pattern to the stack.
    fn add_pattern(
        &mut self,
        pattern: RawPattern,
        world: &World,
        physics: &PhysicsWorld,
    ) -> CastResult {
        // Clone the pattern to put it in the display
        self.patterns.push(pattern.clone());

        let data = pattern.into_data();
        match data {
            SpellData::Junk(_) => {
                return CastResult::Mistake;
            }
            SpellData::Function(func) => {
                // Try and execute it?
                let argc = func.argc();
                if argc > self.stack.len() {
                    // oh no, we tried to pop too many things.
                    return CastResult::Mistake;
                }
                let splitpos = self.stack.len() - argc;
                let data = self.stack.split_off(splitpos);

                if let Some(res) = func.try_execute(data, &mut self.context, world, physics) {
                    // it went ok!
                    self.stack.push(res);
                } else {
                    // oh no, bad things happened
                    return CastResult::Mistake;
                }
            }
            _ => {
                // Just push it
                self.stack.push(data);
            }
        }

        info!("New stack: {:?}", &self.stack);

        if self.stack.len() == 1 && matches!(self.stack[0], SpellData::RenderedSpell(_)) {
            // nice!
            let spell = self.stack[0].clone().unwrap_rendered_spell();
            CastResult::Success(spell)
        } else {
            CastResult::NotDone
        }
    }

    /// Get a reference to the spellcaster's patterns.
    pub fn patterns(&self) -> &[RawPattern] {
        self.patterns.as_slice()
    }

    /// Get a reference to the spell caster's state.
    pub fn state(&self) -> &PatternDrawState {
        &self.state
    }
}

#[derive(Debug)]
pub enum PatternDrawState {
    /// We're thinking about drawing a new pattern if we want
    Waiting,
    /// We're in the middle of drawing a new pattern.
    Drawing {
        /// Pattern we're in progress drawing,
        /// or None if we've only clicked and haven't actually drawn the first line yet.
        ///
        /// Also includes the absolute direction of the previously drawn line, to avoid having
        /// to do the O(n) lookup every time a new line is added.
        wip_pattern: Option<(RawPattern, Direction6)>,
        /// Where the mouse was when we started drawing from this hex.
        mouse_origin: Vec2,
    },
}

impl PatternDrawState {
    pub fn new_drawing(controls: &InputSubscriber) -> Self {
        Self::Drawing {
            wip_pattern: None,
            mouse_origin: controls.mouse_pos(),
        }
    }
}

/// Context in which a spell is cast.
///
/// Certain very powerful spells can alter this.
#[derive(Debug, Clone)]
pub struct SpellContext {
    /// Entity casting this spell
    pub caster: Entity,
}

/// How did casting our spell go?
#[derive(Debug)]
pub enum CastResult {
    /// We haven't actually finished yet
    NotDone,
    /// We cast successfully!
    Success(RenderedSpell),
    /// Something went wrong, oh no, make an explosion
    Mistake,
}
