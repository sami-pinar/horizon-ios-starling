//! Starling's touch controls in the world, on iPhone and iPad: the Android port's touch-to-controller engine
//! (`horizon_client_render::touch::TouchControls`) turns fingers into a virtual controller and camera turns,
//! in place of the Pojav-style overlay of `touch.rs`, which keeps the menus. Fingers on a world screen belong
//! to this module; the pad pipeline (`pad.rs`) does what a controller's buttons do.
use super::Game;
use horizon_client::hud::HudBatch;
use horizon_client_render::touch::{Control, Settings, TouchControls, camera_delta, hotbar_slot};
use winit::{
    event::{Touch, TouchPhase},
    keyboard::KeyCode,
};

/// Degrees of camera turn per mouse-pixel unit `look_turn` takes: the controller's right stick turns the view
/// through the same `/ 0.15`.
const DEGREES_PER_UNIT: f64 = 0.15;

pub(super) struct StarlingTouch {
    pub(super) controls: TouchControls,
    /// Fingers that landed on the hotbar: they pick a slot and are nothing else.
    hotbar: Vec<u64>,
    /// The controls were fed to the pad last frame (so a frame without them lets go once).
    pub(super) fed: bool,
}

impl Default for StarlingTouch {
    fn default() -> Self {
        let mut controls = TouchControls::default();
        if std::path::Path::new("horizon-touch.json").exists() {
            // What the player moved and resized in Options > Controls > Touch Controls.
            controls.settings = Settings::load();
        } else {
            // The settings button at the top did nothing here (the editor is in Options instead).
            controls.settings.button_visibility[Control::Settings as usize] = 2; // 0 follows the scheme, 1 shows, 2 hides
            // Debug, chat, pause and camera sit together in the middle of the top row, with no hole where the
            // settings button was.
            for row in controls.settings.layouts.iter_mut() {
                for (control, x) in [
                    (Control::Debug, 0.44),
                    (Control::Chat, 0.48),
                    (Control::Pause, 0.52),
                    (Control::Perspective, 0.56),
                ] {
                    row[control as usize].center[0] = x;
                }
            }
        }
        Self {
            controls,
            hotbar: Vec::new(),
            fed: false,
        }
    }
}

impl Game<'_> {
    /// Whether Starling's controls play the world: on iOS unless `HORIZON_AMETHYST_TOUCH` brings back the
    /// Pojav-style overlay.
    pub(super) fn starling_on(&self) -> bool {
        cfg!(target_os = "ios") && std::env::var_os("HORIZON_AMETHYST_TOUCH").is_none()
    }

    /// A finger on the world. `size` is the surface in pixels.
    pub(super) fn starling_event(&mut self, touch: &Touch, size: (u32, u32)) {
        let (width, height) = (size.0.max(1), size.1.max(1));
        let point = [
            (touch.location.x as f32 / width as f32).clamp(0.0, 1.0),
            (touch.location.y as f32 / height as f32).clamp(0.0, 1.0),
        ];
        let started = touch.phase == TouchPhase::Started;
        let ended = matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled);
        self.touch.starling.controls.viewport(width, height);
        if started {
            // The hotbar is a row of GUI pixels the controls know nothing of.
            let scale = HudBatch::new([width, height]).scale;
            let gui = [
                touch.location.x as f32 / scale,
                touch.location.y as f32 / scale,
            ];
            if self.touch.starling.controls.control_at(point).is_none()
                && let Some(slot) = hotbar_slot(width as f32 / scale, height as f32 / scale, gui)
            {
                self.selected = slot;
                self.touch.starling.hotbar.push(touch.id);
                return;
            }
        }
        if self.touch.starling.hotbar.contains(&touch.id) {
            if ended {
                self.touch.starling.hotbar.retain(|id| *id != touch.id);
            }
            return;
        }
        let turn = self
            .touch
            .starling
            .controls
            .update(touch.id, point, started, ended);
        if turn != [0.0; 2] {
            let [yaw, pitch] = camera_delta(turn, width, height, &self.touch.starling.controls.settings);
            self.look_turn(
                f64::from(yaw) / DEGREES_PER_UNIT,
                f64::from(pitch) / DEGREES_PER_UNIT,
            );
        }
    }

    /// Once a frame in the world: the fingers' buttons and stick go to the pad, and the shortcut buttons press
    /// their keys.
    pub(super) fn starling_frame(&mut self) {
        let controls = &mut self.touch.starling.controls;
        let state = controls.poll_state();
        let chat = std::mem::take(&mut controls.chat_requested);
        let debug = std::mem::take(&mut controls.debug_requested);
        let perspective = std::mem::take(&mut controls.perspective_requested);
        controls.settings_requested = false;
        controls.clicked = false;
        let custom = controls.key_events();
        controls.take_tap();
        self.pad.feed_touch(state);
        self.touch.starling.fed = true;
        for (token, down) in custom {
            self.starling_token(&token, down);
        }
        for (requested, key) in [
            (chat, KeyCode::KeyT),
            (debug, KeyCode::F3),
            (perspective, KeyCode::F5),
        ] {
            if requested {
                let _ = self.touch_press_key(key);
            }
        }
    }

    /// A custom button's key or mouse button going down or up: the token names a keyboard key, a mouse button, a
    /// wheel step, or a game action (`action:key.pickItem`), which stands for the key it is bound to.
    fn starling_token(&mut self, token: &str, down: bool) {
        if let Some(action) = token.strip_prefix("action:") {
            let bound = horizon_client::keybinds::KeyBinds::default_of(action);
            if bound != "key.keyboard.unknown" {
                self.starling_token(bound, down);
            }
            return;
        }
        if token == "mouse.scroll.up" || token == "mouse.scroll.down" {
            if down {
                self.selected = if token == "mouse.scroll.up" {
                    (self.selected + 8) % 9
                } else {
                    (self.selected + 1) % 9
                };
            }
            return;
        }
        if let Some(button) = horizon_client::keybinds::mouse_button(token) {
            let _ = self.touch_custom_button(button, down);
        } else if let Some(key) = horizon_client::keybinds::key_code(token) {
            let _ = self.touch_custom_key(key, down);
        }
    }

    /// The editor of the touch controls is open (Options > Controls > Touch Controls): it owns every finger and
    /// draws over the screen it was opened from.
    pub(super) fn starling_editing(&self) -> bool {
        self.starling_on() && (self.touch.starling.controls.editor.is_some() || self.starling_panel_open())
    }

    /// Options > Controls > Touch Controls: opens the settings menu, which leads to the editors.
    pub(super) fn starling_open_editor(&mut self) {
        if !self.starling_on() {
            return;
        }
        self.starling_panel_show();
    }

    /// A finger while the settings or an editor is open. `size` is the surface in pixels; `at` and `gui` are the
    /// finger and the screen in GUI pixels.
    pub(super) fn starling_editor_event(
        &mut self,
        touch: &Touch,
        size: (u32, u32),
        at: (f32, f32),
        gui: (f32, f32),
    ) {
        if self.touch.starling.controls.editor.is_none() {
            self.starling_panel_event(touch, at, gui);
            return;
        }
        let (width, height) = (size.0.max(1), size.1.max(1));
        let point = [
            (touch.location.x as f32 / width as f32).clamp(0.0, 1.0),
            (touch.location.y as f32 / height as f32).clamp(0.0, 1.0),
        ];
        let started = touch.phase == TouchPhase::Started;
        let ended = matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled);
        let controls = &mut self.touch.starling.controls;
        controls.viewport(width, height);
        if controls.edit_touch(touch.id, point, started, ended) {
            // Done or Cancel closed it.
            controls.editor = None;
            controls.clear();
        }
    }

    /// The editor: the screen dimmed, and the controls as they sit, to move, resize, add and remove.
    pub(super) fn draw_starling_editor(&mut self, batch: &mut HudBatch) {
        if self.touch.starling.controls.editor.is_none() {
            self.draw_starling_panel(batch);
            return;
        }
        if let Some(size) = self.renderer.as_ref().map(|r| (r.config.width, r.config.height)) {
            self.touch.starling.controls.viewport(size.0.max(1), size.1.max(1));
        }
        self.touch.starling.controls.draw(batch, &self.gui.atlas);
    }

    /// Not in the world (a menu, loading): let go of everything the fingers held.
    pub(super) fn starling_release(&mut self) {
        if std::mem::take(&mut self.touch.starling.fed) {
            self.touch.starling.controls.clear();
            self.touch.starling.hotbar.clear();
            self.pad.feed_touch(Default::default());
            // Whatever a custom button held is let go of.
            for (token, down) in self.touch.starling.controls.key_events() {
                self.starling_token(&token, down);
            }
        }
    }

    /// The controls over the world.
    pub(super) fn draw_starling_controls(&self, batch: &mut HudBatch) {
        self.touch
            .starling
            .controls
            .draw(batch, &self.gui.atlas);
    }
}
