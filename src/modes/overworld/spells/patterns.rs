use std::{f32::consts::TAU, mem};

use hecs::{Entity, World};
// haha DirectionN go brrrrr
use hex2d::{Angle, Coordinate as HexCoord, Direction as Direction6};
use macroquad::prelude::{info, warn, Vec2};

use crate::modes::overworld::spells::{Function, SpellPrototype};

use super::data::SpellData;

/// Pixel distance across a board hexagon horizontally.
/// Also, distance between hexagon centers horizontally.
pub const HEX_WIDTH: f32 = 48.0;
/// Distance from hex center to a corner
pub const HEX_SIZE: f32 = HEX_WIDTH / SQRT_3;
/// Height of a hex.
pub const HEX_HEIGHT: f32 = HEX_SIZE * 2.0;

const SQRT_3: f32 = 1.732051;

/// How far the mouse has to be from the current hex to draw to the next one
pub const NEW_DRAW_TOLERANCE: f32 = HEX_WIDTH * 0.9;

/// Pattern as drawn by the player with the mouse.
/// For most patterns, the orientation doesn't matter, just the delta-turns.
/// But for some the initial orientation is important.
///
/// We use POINTY-TOPPED, flat-sided hexagons.
///
/// No mating.
#[derive(Debug, Clone)]
pub struct RawPattern {
    /// The direction the first connection was drawn in
    pub first_direction: Direction6,
    /// Delta-connections after the first.
    ///
    /// For example, if the player drew:
    /// - Right-up
    /// - Right-up
    /// - Left
    /// - Right-down
    /// - Right-down
    ///
    /// the deltas would be:
    ///
    /// - `Forward`
    /// - `LeftBack`
    /// - `LeftBack`
    /// - `Forward`
    pub deltas: Vec<Angle>,
}

impl RawPattern {
    /// Draw this pattern with the given distance between hex centers,
    /// line width, and position for the center of the board.
    ///
    /// Also, you can draw a final line to the mouse if you provide the mouse origin and position.
    /// If you do it will also draw 5 dots around the "tip" so you know where to aim.
    ///
    /// If `this` is None, then there's no initial direction to draw. Just draw the mouse info if present.
    ///
    /// TODO this function is a giant mess. Will go over when i get to art i suppose.
    pub fn draw(
        this: Option<&Self>,
        center: Vec2,
        mouse_info: Option<(Vec2, Vec2)>,
        hex_dist: f32,
        line_width: f32,
        draw_nodes: bool,
    ) {
        use macroquad::prelude::*;

        let edge_length = hex_dist / SQRT_3;

        let draw_dot = |pos: Vec2| {
            draw_circle(pos.x, pos.y, 2.0, Color::new(0.7, 0.8, 0.9, 0.7));
        };
        let draw_line_fancy = |(x1, y1), (x2, y2)| {
            if draw_nodes {
                draw_dot(vec2(x1, y1));
            }
            draw_line(x1, y1, x2, y2, line_width, Color::new(0.2, 0.7, 0.9, 0.8));
        };
        let draw_hex_and_next = |hex: HexCoord, dir: Direction6| {
            let spacing = hex2d::Spacing::PointyTop(edge_length);
            let src = hex.to_pixel(spacing);

            let angle = dir.to_radians_pointy::<f32>() - TAU / 4.0;
            let (dy, dx) = angle.sin_cos();
            let dstx = src.0 + dx * hex_dist;
            let dsty = src.1 + dy * hex_dist;

            draw_line_fancy(
                (src.0 + center.x, src.1 + center.y),
                (dstx + center.x, dsty + center.y),
            );
        };

        let mut cursor = HexCoord::new(0, 0);
        let prev_dir = if let Some(this) = this {
            let mut prev_dir = this.first_direction;
            draw_hex_and_next(cursor, prev_dir);
            cursor = cursor + prev_dir;

            for angle in this.deltas.iter() {
                prev_dir = prev_dir + *angle;
                draw_hex_and_next(cursor, prev_dir);
                cursor = cursor + prev_dir;
            }
            Some(prev_dir)
        } else {
            None
        };

        if let Some((origin, pos)) = mouse_info {
            // draw_circle(origin.x, origin.y, 1.5, RED);

            // yes wet code bad
            let spacing = hex2d::Spacing::PointyTop(edge_length);
            let src = cursor.to_pixel(spacing);
            let src = vec2(src.0 + center.x, src.1 + center.y);
            let dmouse = pos - origin;
            let dst = src + dmouse;

            draw_line_fancy(src.into(), dst.into());

            // Draw dots in directions we didn't just come from
            for dir in Direction6::all().iter().filter(|d| {
                if let Some(prev_dir) = prev_dir {
                    **d != (prev_dir + Angle::Back)
                } else {
                    true
                }
            }) {
                let spacing = hex2d::Spacing::PointyTop(edge_length);
                let pos = (cursor + *dir).to_pixel(spacing);
                let pos = vec2(pos.0 + center.x, pos.1 + center.y);
                draw_dot(pos);
            }
        }
    }
}

impl RawPattern {
    /// Trace the pattern and figure out what kind of data it is
    pub fn to_data(self) -> SpellData {
        use Angle::*;
        match self.deltas.as_slice() {
            // Return a direction!
            [Forward, Forward, tail @ ..] => {
                // `to_radians` considers 0 to be *up* so we rotate it to being horizontal.
                let initial_angle: f32 =
                    self.first_direction.to_radians_pointy::<f32>() - TAU / 4.0;

                let narrowed = tail
                    .iter()
                    .enumerate()
                    .fold(initial_angle, |acc, (idx, angle)| {
                        let amt = match angle {
                            RightBack => 2.0,
                            Right => 1.0,
                            Forward => 0.0,
                            Left => -1.0,
                            LeftBack => -2.0,
                            Back => {
                                warn!("Had a Back in a direction");
                                0.0
                            }
                        } / 3.0;
                        let narrowness = (1f32 / 6.0).powi(idx as i32 + 1);
                        acc + amt * narrowness
                    });
                SpellData::Direction(narrowed)
            }

            // === Functions ===

            // Select caster with a diamond
            [Left, LeftBack, Left] | [Right, RightBack, Right] => {
                SpellData::Function(Function::GetCaster)
            }
            // Small triangle to get the entity's pos
            [LeftBack, LeftBack] | [RightBack, RightBack] => {
                SpellData::Function(Function::GetPosition)
            }

            // === Spells ===

            // Starburst!
            [Forward, LeftBack, LeftBack, Forward] => SpellPrototype::Starburst.into(),
            // Big hex for shield
            [Forward, Left, Forward, Left, Forward, Left, Forward, Left, Forward, Left, Forward] => {
                SpellPrototype::Shield.into()
            }

            // Otherwise it's junk
            _ => SpellData::Junk(self),
        }
    }
}
