//! Starling's touch controls in the world, on iPhone and iPad: the Android port's touch-to-controller engine
//! (`horizon_client_render::touch::TouchControls`) turns fingers into a virtual controller and camera turns,
//! in place of the Pojav-style overlay of `touch.rs`, which keeps the menus. Fingers on a world screen belong
//! to this module; the pad pipeline (`pad.rs`) does what a controller's buttons do.
use super::Game;
use horizon_client::hud::HudBatch;
use horizon_client_render::touch::{TouchControls, camera_delta, hotbar_slot};
use winit::{
    event::{Touch, TouchPhase},
    keyboard::KeyCode,
};

/// Degrees of camera turn per mouse-pixel unit `look_turn` takes: the controller's right stick turns the view
/// through the same `/ 0.15`.
const DEGREES_PER_UNIT: f64 = 0.15;

#[derive(Default)]
pub(super) struct StarlingTouch {
    pub(super) controls: TouchControls,
    /// Fingers that landed on the hotbar: they pick a slot and are nothing else.
    hotbar: Vec<u64>,
    /// The controls were fed to the pad last frame (so a frame without them lets go once).
    pub(super) fed: bool,
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
        controls.key_events();
        controls.take_tap();
        self.pad.feed_touch(state);
        self.touch.starling.fed = true;
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

    /// Not in the world (a menu, loading): let go of everything the fingers held.
    pub(super) fn starling_release(&mut self) {
        if std::mem::take(&mut self.touch.starling.fed) {
            self.touch.starling.controls.clear();
            self.touch.starling.hotbar.clear();
            self.pad.feed_touch(Default::default());
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
