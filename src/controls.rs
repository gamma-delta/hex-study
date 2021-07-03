use cogs_gamedev::controls::{EventInputHandler, InputHandler};
use enum_map::Enum;
use macroquad::{
    miniquad::{self, Context, KeyMods},
    prelude::{
        mouse_position, screen_height, screen_width,
        utils::{register_input_subscriber, repeat_all_miniquad_input},
        vec2, DVec2, KeyCode, MouseButton, Vec2,
    },
};

use std::collections::HashMap;

use crate::{utils::draw::width_height_deficit, HEIGHT, WIDTH};

/// The controls
#[derive(Enum, Copy, Clone)]
pub enum Control {
    Click,
    Up,
    Down,
    Left,
    Right,

    Submit,
    Debug,
}

/// Combo keycode and mouse button code
#[derive(Hash, PartialEq, Eq, Copy, Clone)]
pub enum InputCode {
    Key(KeyCode),
    Mouse(MouseButton),
}

/// Event handler to hook into miniquad and get inputs
#[derive(Clone)]
pub struct InputSubscriber {
    controls: EventInputHandler<InputCode, Control>,
    subscriber_id: usize,
}

impl InputSubscriber {
    pub fn new() -> Self {
        // the science kid
        let sid = register_input_subscriber();

        InputSubscriber {
            controls: EventInputHandler::new(Self::default_controls()),
            subscriber_id: sid,
        }
    }

    pub fn default_controls() -> HashMap<InputCode, Control> {
        use KeyCode::*;

        let mut controls = HashMap::new();
        controls.insert(InputCode::Mouse(MouseButton::Left), Control::Click);

        for (key, ctrl) in [
            (W, Control::Up),
            (A, Control::Left),
            (S, Control::Down),
            (D, Control::Right),
            //
            (Enter, Control::Submit),
            (Backslash, Control::Debug),
        ] {
            controls.insert(InputCode::Key(key), ctrl);
        }

        controls
    }

    pub fn update(&mut self) {
        repeat_all_miniquad_input(self, self.subscriber_id);
        self.controls.update();
    }

    /// Normalized vector indicating the direction the player is inputting
    pub fn pressed_vec(&self) -> Vec2 {
        let mut out = Vec2::ZERO;

        if self.pressed(Control::Up) {
            out.y -= 1.0;
        }
        if self.pressed(Control::Down) {
            out.y += 1.0;
        }
        if self.pressed(Control::Left) {
            out.x -= 1.0;
        }
        if self.pressed(Control::Right) {
            out.x += 1.0;
        }

        out.normalize_or_zero()
    }

    /// Returns where the mouse is in pixel coordinates.
    ///
    /// Although this can totally be gotten without the input handler,
    /// it just feels wrong to have it be a free-floating function...
    pub fn mouse_pos(&self) -> Vec2 {
        let (mx, my) = mouse_position();
        let (wd, hd) = width_height_deficit();
        let mx = (mx - wd / 2.0) / ((screen_width() - wd) / WIDTH);
        let my = (my - hd / 2.0) / ((screen_height() - hd) / HEIGHT);
        vec2(mx, my)
    }
}

impl std::ops::Deref for InputSubscriber {
    type Target = EventInputHandler<InputCode, Control>;

    fn deref(&self) -> &Self::Target {
        &self.controls
    }
}

impl miniquad::EventHandler for InputSubscriber {
    fn update(&mut self, _ctx: &mut Context) {}

    fn draw(&mut self, _ctx: &mut Context) {}

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        repeat: bool,
    ) {
        if !repeat {
            self.controls.input_down(InputCode::Key(keycode));
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods) {
        self.controls.input_up(InputCode::Key(keycode));
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.controls.input_down(InputCode::Mouse(button));
    }
    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        self.controls.input_up(InputCode::Mouse(button));
    }
}
