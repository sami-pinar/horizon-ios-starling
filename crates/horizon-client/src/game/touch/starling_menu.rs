//! Menu input on iPhone and iPad: two modes, switched by the cursor/finger button at the top right of every
//! interface and remembered between launches.
//!
//! * **Tap**: the finger is the pointer; a tap clicks where it lands, and no cursor is drawn.
//! * **Cursor**: a trackpad, as Windows' touch pointer works. The left quarter of the screen is the mouse
//!   buttons (upper half left click, lower half right click, held while touched), so one thumb can click while
//!   the other hand moves the cursor over the remaining three quarters. There, dragging moves the cursor, a tap
//!   clicks where it stands, a long press right-clicks, tap-then-press-and-hold holds the left button (drag
//!   items, sliders and scrollbars), and two fingers scroll.
use super::{Game, MenuTarget};
use crate::game::event_loop::EventLoopCtl;
use horizon_client::{
    hud::{HudAtlas, HudBatch, rgba},
    touch,
};
use std::time::{Duration, Instant};
use winit::event::{MouseButton, Touch, TouchPhase};
#[cfg(target_os = "ios")]
use winit::keyboard::KeyCode;

/// How far (GUI pixels) a finger travels before a tap becomes a drag.
const SLOP: f32 = 5.0;
/// A touch shorter than this is a tap.
const TAP: Duration = Duration::from_millis(260);
/// A press held this long without moving is a long press: a right click.
const LONG: Duration = Duration::from_millis(480);
/// A second tap this soon after a tap may become a held click.
const DOUBLE: Duration = Duration::from_millis(340);
/// A second touch held this long without lifting holds the left button.
const HOLD: Duration = Duration::from_millis(150);
/// The click zones' default places, as fractions of the screen (x, y, width, height): left click, right click and
/// the shift strip. They sit on the grid of `GRID` cells across and `GRID / 2` down.
pub(in crate::game) const DEFAULT_ZONES: [[f32; 4]; 3] = [
    [0.0, 0.0, 0.25, 0.5],
    [0.0, 0.5, 0.25, 0.5],
    [0.25, 0.0, 0.125, 1.0],
];
/// The editor's snap grid: this many cells across the screen, half as many down.
pub(in crate::game) const GRID: f32 = 32.0;

/// What the player set for cursor mode: how fast the cursor runs (GUI pixels of cursor per GUI pixel of finger),
/// where the three click zones are, and whether the zone editor snaps to its grid. Kept in
/// `Documents/starling-cursor.txt`.
#[derive(Clone)]
pub(in crate::game) struct CursorConfig {
    pub(in crate::game) speed: f32,
    pub(in crate::game) snap: bool,
    pub(in crate::game) zones: [[f32; 4]; 3],
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            speed: 1.5,
            snap: true,
            zones: DEFAULT_ZONES,
        }
    }
}

impl CursorConfig {
    fn path() -> Option<std::path::PathBuf> {
        Some(std::path::PathBuf::from(std::env::var_os("HOME")?).join("Documents/starling-cursor.txt"))
    }

    pub(in crate::game) fn load() -> Self {
        let mut config = Self::default();
        let Some(text) = Self::path().and_then(|path| std::fs::read_to_string(path).ok()) else {
            return config;
        };
        for line in text.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            let number = |i: usize| words.get(i).and_then(|w| w.parse::<f32>().ok()).filter(|v| v.is_finite());
            match words.first().copied() {
                Some("speed") => {
                    if let Some(v) = number(1) {
                        config.speed = v.clamp(0.5, 3.0);
                    }
                }
                Some("snap") => config.snap = number(1).is_none_or(|v| v != 0.0),
                Some("zone") => {
                    if let (Some(i), Some(x), Some(y), Some(w), Some(h)) =
                        (number(1), number(2), number(3), number(4), number(5))
                        && (0.0..3.0).contains(&i)
                        && w > 0.03
                        && h > 0.05
                        && x >= 0.0
                        && y >= 0.0
                        && x + w <= 1.001
                        && y + h <= 1.001
                    {
                        config.zones[i as usize] = [x, y, w, h];
                    }
                }
                _ => {}
            }
        }
        config
    }

    pub(in crate::game) fn save(&self) {
        if let Some(path) = Self::path() {
            let mut text = format!("speed {}\nsnap {}\n", self.speed, u8::from(self.snap));
            for (i, [x, y, w, h]) in self.zones.iter().enumerate() {
                text.push_str(&format!("zone {i} {x} {y} {w} {h}\n"));
            }
            let _ = std::fs::write(path, text);
        }
    }

    /// Zone `zone`'s rectangle on a screen `w` by `h` GUI pixels.
    pub(in crate::game) fn rect(&self, zone: usize, w: f32, h: f32) -> touch::Rect {
        let [x, y, zw, zh] = self.zones[zone];
        [(x * w).round(), (y * h).round(), (zw * w).round(), (zh * h).round()]
    }
}
/// Lines of scroll per GUI pixel of two-finger travel.
const SCROLL_LINE: f32 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    /// The mode button.
    Toggle,
    /// The ✕ and keyboard buttons, and every finger in tap mode: the direct menu gestures of `touch_menu`.
    Direct,
    /// A mouse button zone; `true` for the right button.
    Zone(bool),
    /// The long thin strip beside them: a tap is a shift-click, and Shift stays down while the finger does.
    Shift,
    /// Moves the cursor.
    Pad,
    /// One of two fingers that scroll.
    Scroll,
}

struct Finger {
    id: u64,
    role: Role,
    start: (f32, f32),
    last: (f32, f32),
    since: Instant,
    moved: bool,
    /// Landed soon after a tap: lifting at once double-clicks, holding holds the left button.
    again: bool,
    /// A long press already right-clicked.
    long: bool,
    /// Holds the left button down.
    holding: bool,
}

struct Edge {
    due: u64,
    button: MouseButton,
    down: bool,
}

#[derive(Default)]
pub(in crate::game) struct StarlingMenu {
    loaded: bool,
    cursor_mode: bool,
    /// The cursor, in GUI pixels, while in cursor mode.
    at: Option<(f32, f32)>,
    fingers: Vec<Finger>,
    edges: Vec<Edge>,
    /// Buttons logically down: left, right.
    down: [bool; 2],
    last_tap: Option<Instant>,
    /// Two-finger travel not yet turned into whole lines of scroll.
    scroll: f32,
    /// Shift is down for a shift-click: pressed by the strip, let go once its click has gone out.
    pub(super) shift: bool,
    /// The frame from which Shift comes up, once the finger on the strip has lifted.
    shift_off: Option<u64>,
    /// The eye button hid the click zones.
    zones_hidden: bool,
    /// The player's cursor settings.
    pub(in crate::game) config: CursorConfig,
}

fn mode_path() -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from(std::env::var_os("HOME")?).join("Documents/starling-ui-mode.txt"))
}

/// The saved state: the words `cursor` and `hidden`, in any order (`tap` and nothing else when neither).
fn save_state(cursor: bool, hidden: bool) {
    if let Some(path) = mode_path() {
        let text = match (cursor, hidden) {
            (true, true) => "cursor hidden",
            (true, false) => "cursor",
            (false, true) => "tap hidden",
            (false, false) => "tap",
        };
        let _ = std::fs::write(path, text);
    }
}

/// The mode button: the top right, kept inside a phone's rounded corner.
pub fn toggle_rect(w: f32, h: f32) -> touch::Rect {
    let s = (touch::button_size(h) * 0.75).round();
    [w - s - 24.0, 10.0, s, s]
}

/// The eye button under the mode button: shows or hides the click zones.
fn eye_rect(w: f32, h: f32) -> touch::Rect {
    let t = toggle_rect(w, h);
    [t[0], t[1] + t[3] + 6.0, t[2], t[3]]
}

impl Game<'_> {
    /// The zone under `at`, if any: 0 left click, 1 right click, 2 the shift strip.
    fn starling_zone_at(&self, at: (f32, f32), size: (f32, f32)) -> Option<usize> {
        (0..3).find(|&zone| touch::hit_rect(self.touch.starling_menu.config.rect(zone, size.0, size.1), at))
    }

    fn starling_menu_load(&mut self) {
        if self.touch.starling_menu.loaded {
            return;
        }
        self.touch.starling_menu.loaded = true;
        self.touch.starling_menu.config = CursorConfig::load();
        let saved = mode_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_default();
        self.touch.starling_menu.cursor_mode = saved.split_whitespace().any(|word| word == "cursor");
        self.touch.starling_menu.zones_hidden = saved.split_whitespace().any(|word| word == "hidden");
    }

    fn starling_menu_cursor(&self) -> bool {
        self.touch.starling_menu.cursor_mode
    }

    /// A button edge, two frames on and a frame after the last queued one, so the pointer is drawn where it
    /// stands (the widgets under it are found while drawing) before it presses, and a click has a frame between
    /// its press and release.
    fn starling_menu_edge(&mut self, button: MouseButton, down: bool) {
        let index = usize::from(button == MouseButton::Right);
        let frames = self.frames;
        let menu = &mut self.touch.starling_menu;
        let due = menu
            .edges
            .last()
            .map_or(frames + 2, |edge| (edge.due + 1).max(frames + 2));
        menu.edges.push(Edge { due, button, down });
        // Track the logical state at queue time, for `release` to know what to let go of.
        menu.down[index] = down;
    }

    /// Shift down or up, for a shift-click on the strip.
    fn starling_menu_shift(&mut self, down: bool) {
        self.touch.starling_menu.shift = down;
        #[cfg(target_os = "ios")]
        let _ = self.touch_raw_key(KeyCode::ShiftLeft, down);
    }

    fn starling_menu_click(&mut self, button: MouseButton) {
        self.starling_menu_edge(button, true);
        self.starling_menu_edge(button, false);
    }

    /// A finger on a screen that is not the world. `at` and `size` are in GUI pixels.
    pub(super) fn starling_menu_event(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        touch: &Touch,
        at: (f32, f32),
        size: (f32, f32),
    ) {
        self.starling_menu_load();
        let id = touch.id;
        match touch.phase {
            TouchPhase::Started => self.starling_menu_down(event_loop, id, at, size),
            TouchPhase::Moved => self.starling_menu_move(event_loop, id, at),
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.starling_menu_up(event_loop, id, at, touch.phase == TouchPhase::Cancelled)
            }
        }
    }

    fn starling_menu_down(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: u64,
        at: (f32, f32),
        size: (f32, f32),
    ) {
        let (w, h) = size;
        let mut finger = Finger {
            id,
            role: Role::Direct,
            start: at,
            last: at,
            since: Instant::now(),
            moved: false,
            again: false,
            long: false,
            holding: false,
        };
        if touch::hit_rect(toggle_rect(w, h), at) {
            finger.role = Role::Toggle;
            self.starling_menu_switch(event_loop);
        } else if self.starling_menu_cursor() && touch::hit_rect(eye_rect(w, h), at) {
            finger.role = Role::Toggle;
            self.starling_menu_eye();
        } else if touch::hit_rect(touch::close_rect(w, h), at) && self.touch_close_shown()
            || touch::hit_rect(touch::keyboard_rect(w, h), at) && self.field_focused()
            || !self.starling_menu_cursor()
        {
            finger.role = Role::Direct;
            self.touch_menu(event_loop, id, TouchPhase::Started, at);
        } else if let Some(zone) = self.starling_zone_at(at, size) {
            if zone == 2 {
                finger.role = Role::Shift;
                self.touch.starling_menu.shift_off = None;
                self.starling_menu_shift(true);
                // The strip clicks at once: Shift is down before the press goes out.
                self.starling_menu_click(MouseButton::Left);
            } else {
                let right = zone == 1;
                finger.role = Role::Zone(right);
                self.starling_menu_edge(
                    if right {
                        MouseButton::Right
                    } else {
                        MouseButton::Left
                    },
                    true,
                );
            }
        } else {
            let pads = self
                .touch
                .starling_menu
                .fingers
                .iter()
                .filter(|f| matches!(f.role, Role::Pad | Role::Scroll))
                .count();
            if pads >= 1 {
                // A second finger on the pad: both scroll, and neither is a tap any more.
                finger.role = Role::Scroll;
                for other in &mut self.touch.starling_menu.fingers {
                    if other.role == Role::Pad && !other.holding {
                        other.role = Role::Scroll;
                    }
                }
            } else {
                finger.role = Role::Pad;
                finger.again = self
                    .touch
                    .starling_menu
                    .last_tap
                    .is_some_and(|tap| tap.elapsed() < DOUBLE);
                let start = self.starling_menu_pointer(size);
                self.touch_pointer(event_loop, start);
            }
        }
        self.touch.starling_menu.fingers.push(finger);
    }

    /// Where the cursor is, putting it in the middle the first time.
    fn starling_menu_pointer(&mut self, size: (f32, f32)) -> (f32, f32) {
        *self
            .touch
            .starling_menu
            .at
            .get_or_insert((size.0 / 2.0, size.1 / 2.0))
    }

    fn starling_menu_move(&mut self, event_loop: &dyn EventLoopCtl, id: u64, at: (f32, f32)) {
        let Some(index) = self
            .touch
            .starling_menu
            .fingers
            .iter()
            .position(|f| f.id == id)
        else {
            return;
        };
        let (role, last) = {
            let f = &self.touch.starling_menu.fingers[index];
            (f.role, f.last)
        };
        let delta = (at.0 - last.0, at.1 - last.1);
        {
            let f = &mut self.touch.starling_menu.fingers[index];
            f.last = at;
            if !f.moved && (at.0 - f.start.0).hypot(at.1 - f.start.1) > SLOP {
                f.moved = true;
            }
        }
        match role {
            Role::Direct => self.touch_menu(event_loop, id, TouchPhase::Moved, at),
            Role::Pad => {
                let (moved, long) = {
                    let f = &self.touch.starling_menu.fingers[index];
                    (f.moved, f.long)
                };
                if moved && !long {
                    self.starling_menu_hold_if_again(index);
                    let gain = self.touch.starling_menu.config.speed;
                    if let Some(size) = self.touch_batch().map(|b| (b.width, b.height)) {
                        let (x, y) = self.starling_menu_pointer(size);
                        let next = (
                            (x + delta.0 * gain).clamp(0.0, size.0 - 1.0),
                            (y + delta.1 * gain).clamp(0.0, size.1 - 1.0),
                        );
                        self.touch.starling_menu.at = Some(next);
                        self.touch_pointer(event_loop, next);
                    }
                }
            }
            Role::Scroll => {
                // Whole lines only: the game's lists ignore a fraction of a line, so the remainder waits.
                let menu = &mut self.touch.starling_menu;
                menu.scroll += delta.1 / SCROLL_LINE;
                let lines = menu.scroll.trunc();
                if lines != 0.0 {
                    menu.scroll -= lines;
                    self.touch_wheel(event_loop, lines);
                }
            }
            Role::Toggle | Role::Zone(_) | Role::Shift => {}
        }
    }

    /// A finger that landed soon after a tap and then moves is a drag with the left button held.
    fn starling_menu_hold_if_again(&mut self, index: usize) {
        let f = &self.touch.starling_menu.fingers[index];
        if f.again && !f.holding {
            self.touch.starling_menu.fingers[index].holding = true;
            self.starling_menu_edge(MouseButton::Left, true);
        }
    }

    fn starling_menu_up(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: u64,
        at: (f32, f32),
        cancelled: bool,
    ) {
        let Some(index) = self
            .touch
            .starling_menu
            .fingers
            .iter()
            .position(|f| f.id == id)
        else {
            return;
        };
        let f = self.touch.starling_menu.fingers.remove(index);
        match f.role {
            Role::Direct => self.touch_menu(
                event_loop,
                id,
                if cancelled {
                    TouchPhase::Cancelled
                } else {
                    TouchPhase::Ended
                },
                at,
            ),
            Role::Zone(right) => self.starling_menu_edge(
                if right {
                    MouseButton::Right
                } else {
                    MouseButton::Left
                },
                false,
            ),
            Role::Pad => {
                if f.holding {
                    self.starling_menu_edge(MouseButton::Left, false);
                    self.touch.starling_menu.last_tap = None;
                } else if !cancelled && !f.moved && !f.long && f.since.elapsed() < TAP {
                    // A tap clicks where the cursor stands; a quick second tap clicks again.
                    self.starling_menu_click(MouseButton::Left);
                    self.touch.starling_menu.last_tap = if f.again { None } else { Some(Instant::now()) };
                } else {
                    self.touch.starling_menu.last_tap = None;
                }
            }
            Role::Shift => {
                // Shift stays until the queued click has gone out.
                let frames = self.frames;
                let menu = &mut self.touch.starling_menu;
                let last = menu.edges.last().map_or(frames, |edge| edge.due.max(frames));
                menu.shift_off = Some(last + 2);
            }
            Role::Scroll | Role::Toggle => {}
        }
    }

    /// Switch between tap and cursor mode, letting go of whatever the old one held.
    fn starling_menu_switch(&mut self, event_loop: &dyn EventLoopCtl) {
        self.starling_menu_release(event_loop);
        let menu = &mut self.touch.starling_menu;
        menu.cursor_mode = !menu.cursor_mode;
        if menu.cursor_mode {
            menu.at = Some(self.cursor);
        }
        save_state(menu.cursor_mode, menu.zones_hidden);
    }

    /// How far in from the left edge the click zones reach while they are drawn (0 in tap mode or hidden): what
    /// else is drawn over a screen, such as the chat typing bar, keeps right of it.
    pub(in crate::game) fn starling_zones_reach(&self, width: f32) -> f32 {
        if self.starling_on() && self.touch.starling_menu.cursor_mode && !self.starling_menu_zones_hidden() {
            // The zones on the left half of the screen: what is drawn over a screen keeps right of them.
            self.touch.starling_menu.config.zones.iter().filter(|z| z[0] + z[2] / 2.0 < 0.5).map(|z| (z[0] + z[2]) * width).fold(0.0, f32::max)
        } else {
            0.0
        }
    }

    /// Show or hide the click zones (they keep working hidden).
    fn starling_menu_eye(&mut self) {
        let menu = &mut self.touch.starling_menu;
        menu.zones_hidden = !menu.zones_hidden;
        save_state(menu.cursor_mode, menu.zones_hidden);
    }

    /// The zones are not drawn: the eye button hid them, or the launcher's setting does.
    fn starling_menu_zones_hidden(&self) -> bool {
        self.touch.starling_menu.zones_hidden || touch::mouse_zones_hidden()
    }

    /// Lets go of every finger and button: the screen changed, the world started, or the mode switched.
    pub(super) fn starling_menu_release(&mut self, event_loop: &dyn EventLoopCtl) {
        let fingers = std::mem::take(&mut self.touch.starling_menu.fingers);
        if fingers.iter().any(|f| f.role == Role::Direct) {
            self.touch_release_menu(event_loop);
        }
        if fingers.iter().any(|f| f.role == Role::Shift) || self.touch.starling_menu.shift {
            self.touch.starling_menu.shift_off = None;
            self.starling_menu_shift(false);
        }
        let menu = &mut self.touch.starling_menu;
        menu.edges.clear();
        menu.last_tap = None;
        let down = std::mem::take(&mut menu.down);
        for (held, button) in down.into_iter().zip([MouseButton::Left, MouseButton::Right]) {
            if held {
                self.touch_mouse_button(event_loop, button, false);
            }
        }
    }

    /// Once a frame on a menu: due button edges, long presses, and held clicks.
    pub(super) fn starling_menu_frame(&mut self, event_loop: &dyn EventLoopCtl) {
        self.starling_menu_load();
        let frames = self.frames;
        let mut due = Vec::new();
        self.touch.starling_menu.edges.retain(|edge| {
            if edge.due <= frames {
                due.push((edge.button, edge.down));
                false
            } else {
                true
            }
        });
        for (button, down) in due {
            self.touch_mouse_button(event_loop, button, down);
        }
        if let Some(off) = self.touch.starling_menu.shift_off
            && off <= frames
        {
            self.touch.starling_menu.shift_off = None;
            self.starling_menu_shift(false);
        }
        // Long presses and held second touches.
        let mut right_click = false;
        let mut hold = Vec::new();
        for (index, f) in self.touch.starling_menu.fingers.iter_mut().enumerate() {
            if f.role != Role::Pad || f.holding {
                continue;
            }
            let age = f.since.elapsed();
            if f.again && age >= HOLD {
                hold.push(index);
            } else if !f.again && !f.moved && !f.long && age >= LONG {
                f.long = true;
                right_click = true;
            }
        }
        for index in hold {
            self.starling_menu_hold_if_again(index);
        }
        if right_click {
            self.starling_menu_click(MouseButton::Right);
            self.touch.starling_menu.last_tap = None;
        }
    }

    /// The mode button, the mouse button zones and the cursor, in the style of the touch controls.
    pub(super) fn draw_starling_menu(&mut self, batch: &mut HudBatch) {
        let (w, h) = (batch.width, batch.height);
        let keyboard = self.touch_shown() && self.field_focused();
        let atlas = &self.gui.atlas;
        let cursor_mode = self.touch.starling_menu.cursor_mode;
        let zones_hidden = self.starling_menu_zones_hidden();
        if cursor_mode && !zones_hidden {
            for zone in 0..3 {
                let r = self.touch.starling_menu.config.rect(zone, w, h);
                let held = self.touch.starling_menu.fingers.iter().any(|f| match f.role {
                    Role::Zone(right) => zone == usize::from(right),
                    Role::Shift => zone == 2,
                    _ => false,
                });
                draw_zone(batch, atlas, zone, r, held);
            }
        }
        // The mode button shows the current mode: a reticle in cursor mode, a touch ripple in tap mode.
        let r = toggle_rect(w, h);
        let pressed = self.touch.starling_menu.fingers.iter().any(|f| f.role == Role::Toggle);
        plate(batch, atlas, r, pressed, 190);
        let centre = plate_centre(atlas, r);
        if cursor_mode {
            reticle_icon(batch, atlas, centre);
        } else {
            ripple_icon(batch, atlas, centre);
        }
        if cursor_mode {
            let r = eye_rect(w, h);
            plate(batch, atlas, r, false, 190);
            eye_icon(batch, atlas, plate_centre(atlas, r), zones_hidden);
        }
        if self.touch_close_shown() {
            let held = self
                .touch
                .menu
                .as_ref()
                .is_some_and(|finger| finger.target == MenuTarget::Close);
            let r = touch::close_rect(w, h);
            plate(batch, atlas, r, held, 190);
            cross(batch, atlas, r);
        }
        if keyboard {
            let held = self
                .touch
                .menu
                .as_ref()
                .is_some_and(|finger| finger.target == MenuTarget::Keyboard);
            let r = touch::keyboard_rect(w, h);
            plate(batch, atlas, r, held, 190);
            keyboard_icon(batch, atlas, r);
        }
        if cursor_mode {
            let at = self.touch.starling_menu.at.unwrap_or((w / 2.0, h / 2.0));
            crosshair(batch, atlas, [at.0, at.1], 1.0, self.touch.starling_menu.down[0]);
        }
    }
}

/// The dark bevelled plate of the touch controls (the Bedrock knob when it is installed).
fn plate(b: &mut HudBatch, a: &HudAtlas, r: touch::Rect, pressed: bool, alpha: u8) {
    let [x, y, w, h] = r;
    if let Some(sprite) = a.sprites.get("gui/bedrock/ui/joystick_knob").copied() {
        // Nine slices: the corners and edges keep their shape however long or tall the plate is.
        let shade = if pressed { 170 } else { 255 };
        let color = rgba(shade, shade, shade, alpha);
        let (sw, sh) = (sprite.w as f32, sprite.h as f32);
        let border = 3.0;
        let d = (border * w.min(h) / sw).min(w / 2.0).min(h / 2.0).round().max(1.0);
        let xs = [0.0, border, sw - border, sw];
        let ys = [0.0, border, sh - border, sh];
        let dx = [x, x + d, x + w - d, x + w];
        let dy = [y, y + d, y + h - d, y + h];
        for row in 0..3 {
            for col in 0..3 {
                let mut cell = sprite;
                cell.u0 = sprite.u0 + (sprite.u1 - sprite.u0) * xs[col] / sw;
                cell.u1 = sprite.u0 + (sprite.u1 - sprite.u0) * xs[col + 1] / sw;
                cell.v0 = sprite.v0 + (sprite.v1 - sprite.v0) * ys[row] / sh;
                cell.v1 = sprite.v0 + (sprite.v1 - sprite.v0) * ys[row + 1] / sh;
                b.rect_uv(dx[col], dy[row], dx[col + 1] - dx[col], dy[row + 1] - dy[row], cell, color);
            }
        }
        return;
    }
    let lift = if pressed { 18 } else { 0 };
    let fill = |b: &mut HudBatch, x: f32, y: f32, w: f32, h: f32, c: u32| b.fill(a, x, y, w, h, c);
    let bevel = (w.min(h) * 0.065).clamp(1.0, 3.0);
    let alpha = u16::from(alpha);
    fill(b, x, y, w, h, rgba(28, 28, 32, (alpha * 220 / 255) as u8));
    fill(
        b,
        x + bevel,
        y + bevel,
        w - bevel * 2.0,
        h - bevel * 2.0,
        rgba(58 + lift, 58 + lift, 62 + lift, (alpha * 190 / 255) as u8),
    );
    fill(b, x, y, w, bevel, rgba(215, 215, 222, (alpha * 210 / 255) as u8));
    fill(b, x, y, bevel, h, rgba(215, 215, 222, (alpha * 210 / 255) as u8));
}

/// A pixel picture from rows of characters: `#` white, `X` black, `o` light grey, anything else clear.
fn pixels(b: &mut HudBatch, a: &HudAtlas, at: [f32; 2], u: f32, rows: &[&str]) {
    for (row, line) in rows.iter().enumerate() {
        let cells: Vec<char> = line.chars().collect();
        let mut column = 0;
        while column < cells.len() {
            let color = match cells[column] {
                '#' => rgba(255, 255, 255, 255),
                'X' => rgba(16, 16, 20, 255),
                'o' => rgba(170, 170, 178, 255),
                'g' => rgba(255, 214, 82, 255),
                _ => {
                    column += 1;
                    continue;
                }
            };
            let start = column;
            while column < cells.len() && cells[column] == cells[start] {
                column += 1;
            }
            b.fill(
                a,
                at[0] + start as f32 * u,
                at[1] + row as f32 * u,
                (column - start) as f32 * u,
                u,
                color,
            );
        }
    }
}

/// The cursor: four short white arms around a gap, with a gold dot where the pointer is (red while the left
/// button is down). Each arm has a soft shadow below and to the right, the way Minecraft draws its text, so it
/// shows on bright ground and on dark, and it stays small enough not to hide what is under it.
fn crosshair(b: &mut HudBatch, a: &HudAtlas, at: [f32; 2], u: f32, pressed: bool) {
    // The middle pixel spans [c, c + u): its middle is the pointer, not its top-left corner.
    let (cx, cy) = ((at[0] - 0.5 * u).round(), (at[1] - 0.5 * u).round());
    let white = rgba(255, 255, 255, 255);
    let shade = rgba(0, 0, 0, 120);
    let dot = if pressed { rgba(255, 70, 60, 255) } else { rgba(255, 196, 40, 255) };
    // (x, y, width, height) of the four arms in cells around the middle one: a gap of two cells, arms of four.
    let arms = [(-6, 0, 4, 1), (3, 0, 4, 1), (0, -6, 1, 4), (0, 3, 1, 4)];
    for (ax, ay, aw, ah) in arms {
        b.fill(a, cx + (ax as f32 + 1.0) * u, cy + (ay as f32 + 1.0) * u, aw as f32 * u, ah as f32 * u, shade);
    }
    b.fill(a, cx + u, cy + u, u, u, shade);
    for (ax, ay, aw, ah) in arms {
        b.fill(a, cx + ax as f32 * u, cy + ay as f32 * u, aw as f32 * u, ah as f32 * u, white);
    }
    b.fill(a, cx, cy, u, u, dot);
}

/// One click zone: its plate and icon. 0 left click, 1 right click, 2 the shift strip.
pub(super) fn draw_zone(b: &mut HudBatch, a: &HudAtlas, zone: usize, r: touch::Rect, held: bool) {
    let inset = 5.0;
    let plate_r = [r[0] + inset, r[1] + inset, (r[2] - inset * 2.0).max(8.0), (r[3] - inset * 2.0).max(8.0)];
    plate(b, a, plate_r, held, 120);
    if zone == 2 {
        shift_icon(b, a, plate_r, held);
    } else {
        let side = (plate_r[2].min(plate_r[3]) * 0.42).floor().max(12.0);
        mouse_icon(
            b,
            a,
            [
                (plate_r[0] + (plate_r[2] - side * 0.75) / 2.0).round(),
                (plate_r[1] + (plate_r[3] - side) / 2.0).round(),
            ],
            side / 14.0,
            zone == 1,
        );
    }
}

/// Shift plus a click: the shift arrow over a plus over a mouse with its left button lit. Gold while held.
fn shift_icon(b: &mut HudBatch, a: &HudAtlas, r: touch::Rect, held: bool) {
    const ARROW: [&str; 10] = [
        "....#....",
        "...###...",
        "..#####..",
        ".#######.",
        "#########",
        "...###...",
        "...###...",
        "...###...",
        "...###...",
        "...###...",
    ];
    const PLUS: [&str; 5] = ["..#..", "..#..", "#####", "..#..", "..#.."];
    // Whole GUI pixels, so the cells stay crisp, and small enough for the strip however wide it is.
    // About 27.6 cells tall in all (arrow, plus and mouse with their gaps): the height caps it too.
    let u = (r[2] * 0.36 / 9.0).min(r[3] / 27.6).floor().max(1.0);
    let rows: Vec<String> = ARROW
        .iter()
        .map(|row| if held { row.replace('#', "g") } else { (*row).to_owned() })
        .collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let mouse_u = (u * 0.8).round().max(1.0);
    let gap = u * 1.6;
    let plus_u = (u * 0.8).round().max(1.0);
    let total = 10.0 * u + gap + 5.0 * plus_u + gap + 13.0 * mouse_u;
    let mut y = (r[1] + (r[3] - total) / 2.0).round();
    let cx = r[0] + r[2] / 2.0;
    pixels(b, a, [(cx - 4.5 * u).round(), y], u, &rows);
    y += 10.0 * u + gap;
    pixels(b, a, [(cx - 2.5 * plus_u).round(), y.round()], plus_u, &PLUS);
    y += 5.0 * plus_u + gap;
    mouse_icon(b, a, [(cx - 5.5 * mouse_u).round(), y.round()], mouse_u, false);
}

/// Where an icon's middle is on a plate: the Bedrock plate's face sits a little above and left of its box, so
/// the icon is moved with it.
fn plate_centre(a: &HudAtlas, r: touch::Rect) -> [f32; 2] {
    let (dx, dy) = if a.sprites.contains_key("gui/bedrock/ui/joystick_knob") {
        (-0.5 / 32.0, -1.5 / 32.0)
    } else {
        (0.0, 0.0)
    };
    [
        (r[0] + r[2] * (0.5 + dx)).round(),
        (r[1] + r[3] * (0.5 + dy)).round(),
    ]
}

/// A pixel picture of 1-GUI-pixel cells whose middle cell sits on `centre`.
fn glyph(b: &mut HudBatch, a: &HudAtlas, centre: [f32; 2], size: i32, cell: impl Fn(i32, i32) -> Option<u32>) {
    let half = size / 2;
    for j in 0..size {
        for i in 0..size {
            if let Some(color) = cell(i - half, j - half) {
                b.fill(a, centre[0] + (i - half) as f32, centre[1] + (j - half) as f32, 1.0, 1.0, color);
            }
        }
    }
}

/// Cursor mode: a reticle, a thin ring with four ticks and a dot, symmetrical about its middle pixel.
fn reticle_icon(b: &mut HudBatch, a: &HudAtlas, centre: [f32; 2]) {
    let white = rgba(255, 255, 255, 255);
    let soft = rgba(255, 255, 255, 150);
    glyph(b, a, centre, 17, |x, y| {
        let d = ((x * x + y * y) as f32).sqrt();
        if (6.4..=7.6).contains(&d) {
            Some(soft)
        } else if (x == 0 && (2..=4).contains(&y.abs())) || (y == 0 && (2..=4).contains(&x.abs())) {
            Some(white)
        } else if x == 0 && y == 0 {
            Some(white)
        } else {
            None
        }
    });
}

/// Tap mode: a touch ripple, an outer ring, an inner ring and a dot.
fn ripple_icon(b: &mut HudBatch, a: &HudAtlas, centre: [f32; 2]) {
    let white = rgba(255, 255, 255, 255);
    let soft = rgba(255, 255, 255, 120);
    glyph(b, a, centre, 17, |x, y| {
        let d = ((x * x + y * y) as f32).sqrt();
        if (6.4..=7.6).contains(&d) {
            Some(soft)
        } else if (3.4..=4.5).contains(&d) {
            Some(white)
        } else if d <= 1.2 {
            Some(white)
        } else {
            None
        }
    });
}

/// An eye for showing or hiding the click zones; struck through in red while they are hidden.
fn eye_icon(b: &mut HudBatch, a: &HudAtlas, centre: [f32; 2], hidden: bool) {
    const EYE: [&str; 9] = [
        "....#####....",
        "..##.....##..",
        ".#....#....#.",
        "#....###....#",
        "#...#####...#",
        "#....###....#",
        ".#....#....#.",
        "..##.....##..",
        "....#####....",
    ];
    let tone = if hidden { rgba(170, 170, 178, 255) } else { rgba(255, 255, 255, 255) };
    let strike = rgba(236, 84, 84, 255);
    glyph(b, a, centre, 15, |x, y| {
        let (col, row) = (x + 6, y + 4);
        if (0..13).contains(&col) && (0..9).contains(&row) && EYE[row as usize].as_bytes()[col as usize] == b'#' {
            return Some(tone);
        }
        // The strike runs from the top right to the bottom left, two cells thick.
        if hidden && (x + y).abs() <= 1 && x.abs() <= 6 {
            return Some(strike);
        }
        None
    });
}

/// A mouse with the left or right button lit.
fn mouse_icon(b: &mut HudBatch, a: &HudAtlas, at: [f32; 2], u: f32, right: bool) {
    let mut rows: Vec<String> = [
        ".XXXXXXXXX.",
        "X....X....X",
        "X....X....X",
        "X....X....X",
        "X....X....X",
        "XXXXXXXXXXX",
        "X.........X",
        "X.........X",
        "X.........X",
        "X.........X",
        "X.........X",
        "X.........X",
        ".XXXXXXXXX.",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    // Body in light grey, the pressed button white.
    for row in rows.iter_mut() {
        *row = row.replace('X', "o");
    }
    for (index, row) in rows.iter_mut().enumerate() {
        let mut cells: Vec<char> = row.chars().collect();
        if (1..=4).contains(&index) {
            let range = if right { 6..10 } else { 1..5 };
            for c in range {
                cells[c] = '#';
            }
        }
        *row = cells.into_iter().collect();
    }
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    pixels(b, a, at, u, &rows);
}

fn cross(b: &mut HudBatch, a: &HudAtlas, r: touch::Rect) {
    let u = (r[2] / 14.0).max(1.0);
    let (cx, cy) = ((r[0] + r[2] / 2.0).floor(), (r[1] + r[3] / 2.0).floor());
    for i in -3..=3 {
        let d = i as f32 * u;
        for (x, y) in [(cx + d, cy + d), (cx - d, cy + d)] {
            b.fill(a, x - u / 2.0, y - u / 2.0, u, u, rgba(255, 255, 255, 255));
        }
    }
}

fn keyboard_icon(b: &mut HudBatch, a: &HudAtlas, r: touch::Rect) {
    let u = (r[2] / 16.0).max(1.0);
    let (cx, cy) = ((r[0] + r[2] / 2.0).floor(), (r[1] + r[3] / 2.0).floor());
    for row in 0..2 {
        for key in 0..5 {
            b.fill(
                a,
                cx + (key as f32 - 2.5) * u * 1.8,
                cy + (row as f32 - 1.5) * u * 1.8,
                u,
                u,
                rgba(255, 255, 255, 255),
            );
        }
    }
    b.fill(a, cx - 3.0 * u, cy + 1.7 * u, 6.0 * u, u, rgba(255, 255, 255, 255));
}
