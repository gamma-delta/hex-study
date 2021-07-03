#![feature(try_blocks)]
#![feature(trace_macros)]

mod assets;
mod boilerplates;
mod controls;
mod modes;
mod utils;

// `getrandom` doesn't support WASM so we use quadrand's rng for it.
#[cfg(target_arch = "wasm32")]
mod wasm_random_impl;

use crate::{
    assets::Assets,
    boilerplates::{FrameInfo, Gamemode},
    controls::InputSubscriber,
    modes::ModeLogo,
    utils::draw::width_height_deficit,
};

use macroquad::prelude::*;

const WIDTH: f32 = 640.0;
const HEIGHT: f32 = 480.0;
const ASPECT_RATIO: f32 = WIDTH / HEIGHT;

const UPDATES_PER_DRAW: u64 = 1;
const UPDATE_DT: f32 = 1.0 / (30.0 * UPDATES_PER_DRAW as f32);

/// The `macroquad::main` macro uses this.
fn window_conf() -> Conf {
    Conf {
        window_title: if cfg!(debug_assertions) {
            concat!(env!("CARGO_CRATE_NAME"), " v", env!("CARGO_PKG_VERSION"))
        } else {
            "Omegaquad Game!"
        }
        .to_owned(),
        fullscreen: false,
        sample_count: 64,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = Assets::init().await;
    let assets = Box::leak(Box::new(assets)) as &'static Assets;
    let mut controls = InputSubscriber::new();

    let canvas = render_target(WIDTH as u32, HEIGHT as u32);
    canvas.texture.set_filter(FilterMode::Nearest);

    let mut mode_stack: Vec<Box<dyn Gamemode>> = vec![Box::new(ModeLogo::new())];

    let mut frame_info = FrameInfo {
        dt: UPDATE_DT,
        frames_ran: 0,
    };

    loop {
        // Update the current state.
        // To change state, return a non-None transition.
        for _ in 0..UPDATES_PER_DRAW {
            controls.update();

            let transition = mode_stack
                .last_mut()
                .unwrap()
                .update(&controls, frame_info, assets);
            transition.apply(&mut mode_stack, assets);
        }

        frame_info.dt = macroquad::time::get_frame_time();

        push_camera_state();
        // These divides and multiplies are required to get the camera in the center of the screen
        // and having it fill everything.
        set_camera(&Camera2D {
            render_target: Some(canvas),
            zoom: vec2((WIDTH as f32).recip() * 2.0, (HEIGHT as f32).recip() * 2.0),
            target: vec2(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0),
            ..Default::default()
        });
        clear_background(WHITE);
        // Draw the state.
        let drawer = mode_stack.last_mut().unwrap();
        drawer.draw(assets, frame_info, &controls);

        // Done rendering to the canvas; go back to our normal camera
        // to size the canvas
        pop_camera_state();
        clear_background(BLACK);

        // Figure out the drawbox.
        // these are how much wider/taller the window is than the content
        let (width_deficit, height_deficit) = width_height_deficit();
        draw_texture_ex(
            canvas.texture,
            width_deficit / 2.0,
            height_deficit / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(
                    screen_width() - width_deficit,
                    screen_height() - height_deficit,
                )),
                ..Default::default()
            },
        );

        frame_info.frames_ran += 1;
        next_frame().await
    }
}
