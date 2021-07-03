use crate::{ASPECT_RATIO, HEIGHT, WIDTH};

use macroquad::prelude::*;

/// Make a Color from an RRGGBBAA hex code.
pub fn hexcolor(code: u32) -> Color {
    let [r, g, b, a] = code.to_be_bytes();
    Color::from_rgba(r, g, b, a)
}

pub fn width_height_deficit() -> (f32, f32) {
    if (screen_width() / screen_height()) > ASPECT_RATIO {
        // it's too wide! put bars on the sides!
        // the height becomes the authority on how wide to draw
        let expected_width = screen_height() * ASPECT_RATIO;
        (screen_width() - expected_width, 0.0f32)
    } else {
        // it's too tall! put bars on the ends!
        // the width is the authority
        let expected_height = screen_width() / ASPECT_RATIO;
        (0.0f32, screen_height() - expected_height)
    }
}

/// Draw a 9patch of a 3x3 grid of tiles.
pub fn patch9(corner_x: f32, corner_y: f32, width: usize, height: usize, tex: Texture2D) {
    let tile_width = tex.width() / 3.0;
    let tile_height = tex.height() / 3.0;

    for x in 0..width {
        for y in 0..height {
            let px = corner_x + x as f32 * tile_width;
            let py = corner_y + y as f32 * tile_height;

            let sx = tile_width
                * if x == 0 {
                    0.0
                } else if x == width - 1 {
                    2.0
                } else {
                    1.0
                };
            let sy = tile_height
                * if y == 0 {
                    0.0
                } else if y == height - 1 {
                    2.0
                } else {
                    1.0
                };

            draw_texture_ex(
                tex,
                px,
                py,
                WHITE,
                DrawTextureParams {
                    source: Some(Rect::new(sx, sy, 16.0, 16.0)),
                    ..Default::default()
                },
            );
        }
    }
}
