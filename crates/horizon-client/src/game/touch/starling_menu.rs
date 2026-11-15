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
/// GUI pixels of cursor travel per GUI pixel of finger travel, slow and fast.
const GAIN: (f32, f32) = (1.25, 2.6);
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
}

fn mode_path() -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from(std::env::var_os("HOME")?).join("Documents/starling-ui-mode.txt"))
}

/// The mode button: the top right, kept inside a phone's rounded corner.
pub fn toggle_rect(w: f32, h: f32) -> touch::Rect {
    let s = (touch::button_size(h) * 0.75).round();
    [w - s - 24.0, 10.0, s, s]
}

fn zone_rect(w: f32, h: f32, right: bool) -> touch::Rect {
    let top = if right { h / 2.0 } else { 0.0 };
    [0.0, top, w / 4.0, h / 2.0]
}

impl Game<'_> {
    fn starling_menu_load(&mut self) {
        if self.touch.starling_menu.loaded {
            return;
        }
        self.touch.starling_menu.loaded = true;
        self.touch.starling_menu.cursor_mode = mode_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .is_some_and(|text| text.trim() == "cursor");
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
        } else if touch::hit_rect(touch::close_rect(w, h), at) && self.touch_close_shown()
            || touch::hit_rect(touch::keyboard_rect(w, h), at) && self.field_focused()
            || !self.starling_menu_cursor()
        {
            finger.role = Role::Direct;
            self.touch_menu(event_loop, id, TouchPhase::Started, at);
        } else if at.0 < w / 4.0 {
            let right = at.1 >= h / 2.0;
            finger.role = Role::Zone(right);
            self.starling_menu_edge(
                if right {
                    MouseButton::Right
                } else {
                    MouseButton::Left
                },
                true,
            );
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
                    let speed = delta.0.hypot(delta.1);
                    let gain = (GAIN.0 + speed / 5.0 * (GAIN.1 - GAIN.0)).min(GAIN.1);
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
                let lines = (delta.1 / SCROLL_LINE) * 2.0;
                if lines != 0.0 {
                    self.touch_wheel(event_loop, lines);
                }
            }
            Role::Toggle | Role::Zone(_) => {}
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
        if let Some(path) = mode_path() {
            let _ = std::fs::write(path, if menu.cursor_mode { "cursor" } else { "tap" });
        }
    }

    /// Lets go of every finger and button: the screen changed, the world started, or the mode switched.
    pub(super) fn starling_menu_release(&mut self, event_loop: &dyn EventLoopCtl) {
        let fingers = std::mem::take(&mut self.touch.starling_menu.fingers);
        if fingers.iter().any(|f| f.role == Role::Direct) {
            self.touch_release_menu(event_loop);
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
        if cursor_mode {
            for right in [false, true] {
                let r = zone_rect(w, h, right);
                let held = self.touch.starling_menu.fingers.iter().any(|f| f.role == Role::Zone(right));
                let inset = 6.0;
                let plate_r = [r[0] + inset, r[1] + inset, r[2] - inset * 1.5, r[3] - inset * 2.0];
                plate(batch, atlas, plate_r, held, 120);
                let side = (plate_r[2].min(plate_r[3]) * 0.42).floor().max(12.0);
                mouse_icon(
                    batch,
                    atlas,
                    [
                        plate_r[0] + (plate_r[2] - side * 0.75) / 2.0,
                        plate_r[1] + (plate_r[3] - side) / 2.0,
                    ],
                    side / 14.0,
                    right,
                );
            }
        }
        // The mode button shows the current mode: an arrow in cursor mode, a touch ring in tap mode.
        let r = toggle_rect(w, h);
        let pressed = self.touch.starling_menu.fingers.iter().any(|f| f.role == Role::Toggle);
        plate(batch, atlas, r, pressed, 190);
        if cursor_mode {
            arrow(batch, atlas, [r[0] + r[2] * 0.3, r[1] + r[3] * 0.18], r[3] * 0.64 / 17.0);
        } else {
            ring_icon(batch, atlas, [r[0] + r[2] / 2.0, r[1] + r[3] / 2.0], r[3] * 0.62 / 13.0);
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
            arrow(batch, atlas, [at.0, at.1], 1.0);
        }
    }
}

/// The dark bevelled plate of the touch controls (the Bedrock knob when it is installed).
fn plate(b: &mut HudBatch, a: &HudAtlas, r: touch::Rect, pressed: bool, alpha: u8) {
    let [x, y, w, h] = r;
    if let Some(sprite) = a.sprites.get("gui/bedrock/ui/joystick_knob").copied() {
        let shade = if pressed { 170 } else { 255 };
        b.rect_uv(x, y, w, h, sprite, rgba(shade, shade, shade, alpha));
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

const ARROW: [&str; 17] = [
    "X..........",
    "XX.........",
    "X#X........",
    "X##X.......",
    "X###X......",
    "X####X.....",
    "X#####X....",
    "X######X...",
    "X#######X..",
    "X########X.",
    "X#####XXXXX",
    "X##X##X....",
    "X#X.X##X...",
    "XX..X##X...",
    "X....X##X..",
    ".....X##X..",
    "......XX...",
];

/// The mouse pointer, its tip at `at`.
fn arrow(b: &mut HudBatch, a: &HudAtlas, at: [f32; 2], u: f32) {
    pixels(b, a, at, u, &ARROW);
}

/// A touch ring with a dot: tap mode.
fn ring_icon(b: &mut HudBatch, a: &HudAtlas, center: [f32; 2], u: f32) {
    for row in 0..13 {
        for col in 0..13 {
            let d = ((col as f32 - 6.0).powi(2) + (row as f32 - 6.0).powi(2)).sqrt();
            let color = if d <= 1.7 {
                Some(rgba(255, 255, 255, 255))
            } else if (4.4..=6.3).contains(&d) {
                Some(rgba(255, 255, 255, 255))
            } else {
                None
            };
            if let Some(color) = color {
                b.fill(a, center[0] + (col as f32 - 6.5) * u, center[1] + (row as f32 - 6.5) * u, u, u, color);
            }
        }
    }
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
