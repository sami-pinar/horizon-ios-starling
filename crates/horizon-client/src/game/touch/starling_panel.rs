//! The Touch Controls settings (Options > Controls > Touch Controls), drawn over the screen it was opened from.
//!
//! * **Menu**: camera sensitivity, cursor speed, button size, left-handed layout, the control scheme and layout
//!   preset to switch between, and the two editors.
//! * **Edit Layout** opens Starling's editor for the buttons in the world (`TouchControls::open_editor`).
//! * **Edit Cursor Buttons** (`Panel::Zones`): the two mouse buttons and the shift strip of cursor mode can be
//!   moved and resized, snapping to a grid so nothing is left with odd gaps.
use super::{Game, starling_menu::DEFAULT_ZONES, starling_menu::GRID};
use horizon_client::{
    hud::{HudBatch, rgba},
    text::Text,
    touch, ui,
};
use horizon_client_render::touch::{Mode, Settings};
use std::time::{Duration, Instant};
use winit::event::{Touch, TouchPhase};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(in crate::game) enum Panel {
    #[default]
    Closed,
    Menu,
    Zones,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Id {
    Sensitivity,
    CursorSpeed,
    ButtonSize,
    LeftHanded,
    Scheme,
    Preset,
    SavePreset,
    EditLayout,
    EditZones,
    Done,
    ZoneDone,
    ZoneReset,
    ZoneSnap,
}

/// A zone being dragged: where it started (fractions) and where the finger did (GUI pixels).
#[derive(Clone, Copy)]
struct ZoneDrag {
    zone: usize,
    resize: bool,
    start: [f32; 4],
    from: (f32, f32),
}

#[derive(Default)]
pub(in crate::game) struct StarlingPanel {
    panel: Panel,
    /// The finger on a button or slider.
    finger: Option<(u64, Id)>,
    drag: Option<(u64, ZoneDrag)>,
    /// Layout presets: id and name.
    presets: Vec<(String, String)>,
    notice: Option<(String, Instant)>,
}

const BUTTON_W: f32 = 150.0;
const BUTTON_H: f32 = 20.0;
const COLUMN_GAP: f32 = 10.0;
const ROW: f32 = 24.0;
/// The corner a zone is resized by.
const HANDLE: f32 = 16.0;
const SENSITIVITY: (f32, f32) = (0.1, 3.0);
const SPEED: (f32, f32) = (0.5, 3.0);
const SIZE: (f32, f32) = (0.5, 2.0);

/// Where the header ends and the footer begins, as on the vanilla options screens.
fn list_bounds(h: f32) -> (f32, f32) {
    (33.0, h - 33.0)
}

fn menu_items(w: f32, h: f32) -> Vec<(Id, [f32; 4])> {
    let left = ((w - (BUTTON_W * 2.0 + COLUMN_GAP)) / 2.0).floor();
    let right = left + BUTTON_W + COLUMN_GAP;
    let (list_top, list_bottom) = list_bounds(h);
    // The five rows, centred between the separators.
    let top = (list_top + ((list_bottom - list_top) - (5.0 * ROW - 4.0)) / 2.0).floor();
    let row = |i: f32| (top + i * ROW).floor();
    let wide = BUTTON_W * 2.0 + COLUMN_GAP;
    vec![
        (Id::Sensitivity, [left, row(0.0), BUTTON_W, BUTTON_H]),
        (Id::CursorSpeed, [right, row(0.0), BUTTON_W, BUTTON_H]),
        (Id::ButtonSize, [left, row(1.0), BUTTON_W, BUTTON_H]),
        (Id::LeftHanded, [right, row(1.0), BUTTON_W, BUTTON_H]),
        (Id::Scheme, [left, row(2.0), wide, BUTTON_H]),
        (Id::Preset, [left, row(3.0), BUTTON_W, BUTTON_H]),
        (Id::SavePreset, [right, row(3.0), BUTTON_W, BUTTON_H]),
        (Id::EditLayout, [left, row(4.0), BUTTON_W, BUTTON_H]),
        (Id::EditZones, [right, row(4.0), BUTTON_W, BUTTON_H]),
        // Done sits in the footer, 200 wide like vanilla's.
        (Id::Done, [((w - 200.0) / 2.0).floor(), h - 27.0, 200.0, BUTTON_H]),
    ]
}

fn zone_bar(w: f32) -> [(Id, [f32; 4]); 3] {
    let bw = 70.0;
    let x = ((w - (bw * 3.0 + 8.0)) / 2.0).floor();
    [
        (Id::ZoneDone, [x, 6.0, bw, BUTTON_H]),
        (Id::ZoneReset, [x + bw + 4.0, 6.0, bw, BUTTON_H]),
        (Id::ZoneSnap, [x + (bw + 4.0) * 2.0, 6.0, bw, BUTTON_H]),
    ]
}

fn slider_ids() -> [Id; 3] {
    [Id::Sensitivity, Id::CursorSpeed, Id::ButtonSize]
}

fn percent(v: f32) -> String {
    format!("{}%", (v * 100.0).round() as i32)
}

impl Game<'_> {
    pub(in crate::game) fn starling_panel_open(&self) -> bool {
        self.touch.starling_panel.panel != Panel::Closed
    }

    /// Opens the settings menu.
    pub(in crate::game) fn starling_panel_show(&mut self) {
        self.touch.starling_panel.panel = Panel::Menu;
        horizon_client_render::touch::set_settings_cover(true);
        self.touch.starling_panel.finger = None;
        self.touch.starling_panel.drag = None;
        self.starling_panel_presets();
    }

    fn starling_panel_presets(&mut self) {
        let style = self.touch.starling.controls.settings.control_style;
        self.touch.starling_panel.presets = Settings::list_presets(std::path::Path::new("."), style)
            .into_iter()
            .map(|preset| (preset.id, preset.name))
            .collect();
    }

    fn starling_panel_slider(&self, id: Id) -> (f32, String, f32) {
        let s = &self.touch.starling.controls.settings;
        match id {
            Id::Sensitivity => (
                (s.sensitivity - SENSITIVITY.0) / (SENSITIVITY.1 - SENSITIVITY.0),
                format!("Camera Sensitivity: {}", percent(s.sensitivity)),
                s.sensitivity,
            ),
            Id::CursorSpeed => {
                let v = self.touch.starling_menu.config.speed;
                (
                    (v - SPEED.0) / (SPEED.1 - SPEED.0),
                    format!("Cursor Speed: {}", percent(v)),
                    v,
                )
            }
            _ => (
                (s.button_scale - SIZE.0) / (SIZE.1 - SIZE.0),
                format!("Button Size: {}", percent(s.button_scale)),
                s.button_scale,
            ),
        }
    }

    fn starling_panel_set(&mut self, id: Id, fraction: f32) {
        let f = fraction.clamp(0.0, 1.0);
        let step = |v: f32| (v * 20.0).round() / 20.0;
        match id {
            Id::Sensitivity => {
                self.touch.starling.controls.settings.sensitivity =
                    step(SENSITIVITY.0 + f * (SENSITIVITY.1 - SENSITIVITY.0)).clamp(SENSITIVITY.0, SENSITIVITY.1);
            }
            Id::CursorSpeed => {
                self.touch.starling_menu.config.speed = step(SPEED.0 + f * (SPEED.1 - SPEED.0)).clamp(SPEED.0, SPEED.1);
            }
            Id::ButtonSize => {
                self.touch.starling.controls.settings.button_scale =
                    step(SIZE.0 + f * (SIZE.1 - SIZE.0)).clamp(SIZE.0, SIZE.1);
            }
            _ => {}
        }
    }

    /// A finger on the panel. `at` and `size` are in GUI pixels.
    pub(in crate::game) fn starling_panel_event(&mut self, touch: &Touch, at: (f32, f32), size: (f32, f32)) {
        match self.touch.starling_panel.panel {
            Panel::Closed => {}
            Panel::Menu => self.starling_panel_menu_event(touch, at, size),
            Panel::Zones => self.starling_panel_zones_event(touch, at, size),
        }
    }

    fn starling_panel_menu_event(&mut self, touch: &Touch, at: (f32, f32), size: (f32, f32)) {
        let items = menu_items(size.0, size.1);
        let hit = |at: (f32, f32)| items.iter().find(|(_, r)| touch::hit_rect(*r, at)).copied();
        match touch.phase {
            TouchPhase::Started => {
                if self.touch.starling_panel.finger.is_some() {
                    return;
                }
                if let Some((id, rect)) = hit(at) {
                    self.touch.starling_panel.finger = Some((touch.id, id));
                    if slider_ids().contains(&id) {
                        self.starling_panel_set(id, ui::slider_value(rect[0], rect[2], at.0));
                    }
                }
            }
            TouchPhase::Moved => {
                if let Some((owner, id)) = self.touch.starling_panel.finger
                    && owner == touch.id
                    && slider_ids().contains(&id)
                    && let Some((_, rect)) = items.iter().find(|(i, _)| *i == id)
                {
                    self.starling_panel_set(id, ui::slider_value(rect[0], rect[2], at.0));
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let Some((owner, id)) = self.touch.starling_panel.finger else {
                    return;
                };
                if owner != touch.id {
                    return;
                }
                self.touch.starling_panel.finger = None;
                let released_on = hit(at).map(|(i, _)| i);
                if touch.phase == TouchPhase::Ended && released_on == Some(id) && !slider_ids().contains(&id) {
                    self.starling_panel_press(id);
                }
            }
        }
    }

    fn starling_panel_press(&mut self, id: Id) {
        match id {
            Id::LeftHanded => {
                let s = &mut self.touch.starling.controls.settings;
                s.left_handed = !s.left_handed;
            }
            Id::Scheme => {
                let s = &mut self.touch.starling.controls.settings;
                let current = Mode::SCHEMES
                    .iter()
                    .position(|(m, split, _)| *m == s.mode && *split == s.split_controls)
                    .unwrap_or(0);
                let (mode, split, _) = Mode::SCHEMES[(current + 1) % Mode::SCHEMES.len()];
                s.mode = mode;
                s.split_controls = split;
            }
            Id::Preset => {
                let presets = self.touch.starling_panel.presets.clone();
                if presets.len() > 1 {
                    let style = self.touch.starling.controls.settings.control_style;
                    let current = presets
                        .iter()
                        .position(|(id, _)| *id == self.touch.starling.controls.settings.preset_id)
                        .unwrap_or(0);
                    let next = &presets[(current + 1) % presets.len()].0;
                    self.touch
                        .starling
                        .controls
                        .settings
                        .select_preset(std::path::Path::new("."), style, next);
                    self.starling_panel_presets();
                }
            }
            Id::SavePreset => {
                let name = format!("Layout {}", self.touch.starling_panel.presets.len());
                let saved = self
                    .touch
                    .starling
                    .controls
                    .settings
                    .save_new_preset(std::path::Path::new("."), &name, "")
                    .is_ok();
                self.touch.starling_panel.notice = Some((
                    if saved {
                        format!("Saved as {name}")
                    } else {
                        "Could not save".to_owned()
                    },
                    Instant::now(),
                ));
                self.starling_panel_presets();
            }
            Id::EditLayout => {
                if let Some(size) = self.renderer.as_ref().map(|r| (r.config.width, r.config.height)) {
                    self.touch.starling.controls.viewport(size.0.max(1), size.1.max(1));
                }
                self.touch.starling.controls.open_editor();
            }
            Id::EditZones => self.touch.starling_panel.panel = Panel::Zones,
            Id::Done => {
                let _ = self.touch.starling.controls.settings.save();
                self.touch.starling_menu.config.save();
                self.touch.starling_panel.panel = Panel::Closed;
                horizon_client_render::touch::set_settings_cover(false);
            }
            Id::Sensitivity | Id::CursorSpeed | Id::ButtonSize | Id::ZoneDone | Id::ZoneReset | Id::ZoneSnap => {}
        }
    }

    fn starling_panel_zones_event(&mut self, touch: &Touch, at: (f32, f32), size: (f32, f32)) {
        let (w, h) = size;
        match touch.phase {
            TouchPhase::Started => {
                if self.touch.starling_panel.finger.is_some() || self.touch.starling_panel.drag.is_some() {
                    return;
                }
                if let Some((id, _)) = zone_bar(w).into_iter().find(|(_, r)| touch::hit_rect(*r, at)) {
                    self.touch.starling_panel.finger = Some((touch.id, id));
                    return;
                }
                for zone in [2, 1, 0] {
                    let r = self.touch.starling_menu.config.rect(zone, w, h);
                    let handle = [r[0] + r[2] - HANDLE, r[1] + r[3] - HANDLE, HANDLE, HANDLE];
                    let resize = touch::hit_rect(handle, at);
                    if resize || touch::hit_rect(r, at) {
                        self.touch.starling_panel.drag = Some((
                            touch.id,
                            ZoneDrag {
                                zone,
                                resize,
                                start: self.touch.starling_menu.config.zones[zone],
                                from: at,
                            },
                        ));
                        return;
                    }
                }
            }
            TouchPhase::Moved => {
                if let Some((owner, drag)) = self.touch.starling_panel.drag
                    && owner == touch.id
                {
                    let moved = (at.0 - drag.from.0, at.1 - drag.from.1);
                    let snap = self.touch.starling_menu.config.snap;
                    let next = zone_after_drag(drag, moved, (w, h), snap);
                    self.touch.starling_menu.config.zones[drag.zone] = next;
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if let Some((owner, _)) = self.touch.starling_panel.drag
                    && owner == touch.id
                {
                    self.touch.starling_panel.drag = None;
                    self.touch.starling_menu.config.save();
                    return;
                }
                let Some((owner, id)) = self.touch.starling_panel.finger else {
                    return;
                };
                if owner != touch.id {
                    return;
                }
                self.touch.starling_panel.finger = None;
                let on_it = zone_bar(w).into_iter().find(|(i, _)| *i == id).is_some_and(|(_, r)| touch::hit_rect(r, at));
                if touch.phase == TouchPhase::Ended && on_it {
                    let config = &mut self.touch.starling_menu.config;
                    match id {
                        Id::ZoneDone => {
                            config.save();
                            self.touch.starling_panel.panel = Panel::Menu;
                        }
                        Id::ZoneReset => {
                            config.zones = DEFAULT_ZONES;
                            config.save();
                        }
                        Id::ZoneSnap => {
                            config.snap = !config.snap;
                            if config.snap {
                                for zone in &mut config.zones {
                                    *zone = snapped(*zone);
                                }
                            }
                            config.save();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// The panel, over a dimmed screen.
    pub(in crate::game) fn draw_starling_panel(&mut self, batch: &mut HudBatch) {
        let (w, h) = (batch.width, batch.height);
        let atlas = &self.gui.atlas;
        match self.touch.starling_panel.panel {
            Panel::Closed => {}
            Panel::Menu => {
                // The vanilla options look: the dirt background, a list area between two separators, the title
                // above and Done in the footer.
                let (top, bottom) = list_bounds(h);
                let in_world = self.player.is_some() && self.title.is_none();
                ui::list_background(batch, atlas, top, bottom, in_world);
                ui::separators(batch, atlas, top, bottom, in_world);
                let title = Text::translate("horizon.pages.open.touch_controls", &[]);
                let title = Text::literal(title.plain().trim_end_matches('.').to_owned());
                let tw = atlas.font.width_text(&title);
                batch.text_styled(atlas, &title, ((w - tw) / 2.0).floor(), 12.0, u32::MAX, true);
                let held = self.touch.starling_panel.finger.map(|(_, id)| id);
                let settings = &self.touch.starling.controls.settings;
                for (id, r) in menu_items(w, h) {
                    let pressed = held == Some(id);
                    if slider_ids().contains(&id) {
                        let (value, label, _) = self.starling_panel_slider(id);
                        ui::slider(batch, atlas, r[0], r[1], r[2], &Text::literal(label), value.clamp(0.0, 1.0), pressed, true, pressed);
                        continue;
                    }
                    let label = match id {
                        Id::LeftHanded => format!("Left-Handed: {}", if settings.left_handed { "ON" } else { "OFF" }),
                        Id::Scheme => {
                            let name = Mode::SCHEMES
                                .iter()
                                .find(|(m, split, _)| *m == settings.mode && *split == settings.split_controls)
                                .map_or("Custom", |s| s.2);
                            format!("Control Scheme: {name}")
                        }
                        Id::Preset => {
                            let name = self
                                .touch
                                .starling_panel
                                .presets
                                .iter()
                                .find(|(id, _)| *id == settings.preset_id)
                                .map_or("Default", |p| p.1.as_str());
                            format!("Layout: {name}")
                        }
                        Id::SavePreset => match &self.touch.starling_panel.notice {
                            Some((text, at)) if at.elapsed() < Duration::from_millis(2500) => text.clone(),
                            _ => "Save Layout as New".to_owned(),
                        },
                        Id::EditLayout => "Edit Layout...".to_owned(),
                        Id::EditZones => "Edit Cursor Buttons...".to_owned(),
                        _ => String::new(),
                    };
                    let text = if id == Id::Done {
                        Text::translate("gui.done", &[])
                    } else {
                        Text::literal(label)
                    };
                    ui::button(batch, atlas, r[0], r[1], r[2], r[3], &text, true, pressed);
                }
            }
            Panel::Zones => {
                let config = &self.touch.starling_menu.config;
                if config.snap {
                    let line = rgba(255, 255, 255, 22);
                    for k in 1..GRID as i32 {
                        batch.fill(atlas, (w * k as f32 / GRID).round(), 0.0, 1.0, h, line);
                    }
                    for k in 1..(GRID / 2.0) as i32 {
                        batch.fill(atlas, 0.0, (h * k as f32 / (GRID / 2.0)).round(), w, 1.0, line);
                    }
                }
                let dragged = self.touch.starling_panel.drag.map(|(_, d)| d.zone);
                for zone in 0..3 {
                    let r = config.rect(zone, w, h);
                    super::starling_menu::draw_zone(batch, atlas, zone, r, dragged == Some(zone));
                    // The resize corner.
                    let corner = [r[0] + r[2] - 11.0, r[1] + r[3] - 11.0, 8.0, 8.0];
                    batch.fill(atlas, corner[0] - 1.0, corner[1] - 1.0, 10.0, 10.0, rgba(20, 20, 24, 230));
                    batch.fill(atlas, corner[0], corner[1], 8.0, 8.0, rgba(255, 200, 60, 255));
                }
                let held = self.touch.starling_panel.finger.map(|(_, id)| id);
                for (id, r) in zone_bar(w) {
                    let text = match id {
                        Id::ZoneDone => Text::translate("gui.done", &[]),
                        Id::ZoneReset => Text::literal("Reset"),
                        _ => Text::literal(if config.snap { "Snap: ON" } else { "Snap: OFF" }),
                    };
                    ui::button(batch, atlas, r[0], r[1], r[2], r[3], &text, true, held == Some(id));
                }
                let hint = Text::literal("Drag to move, drag the corner to resize");
                let hw = atlas.font.width_text(&hint);
                batch.text_styled(atlas, &hint, ((w - hw) / 2.0).floor(), h - 14.0, u32::MAX, true);
            }
        }
    }
}

/// A zone on the grid: position and size in whole cells.
fn snapped([x, y, w, h]: [f32; 4]) -> [f32; 4] {
    let (gx, gy) = (GRID, GRID / 2.0);
    let (w, h) = (((w * gx).round() / gx).max(2.0 / gx), ((h * gy).round() / gy).max(2.0 / gy));
    let x = ((x * gx).round() / gx).clamp(0.0, 1.0 - w);
    let y = ((y * gy).round() / gy).clamp(0.0, 1.0 - h);
    [x, y, w, h]
}

/// Where a dragged zone ends up after the finger moved by `moved` GUI pixels on a screen of `size`.
fn zone_after_drag(drag: ZoneDrag, moved: (f32, f32), size: (f32, f32), snap: bool) -> [f32; 4] {
    let [x, y, w, h] = drag.start;
    let (dx, dy) = (moved.0 / size.0.max(1.0), moved.1 / size.1.max(1.0));
    let raw = if drag.resize {
        let w = (w + dx).clamp(0.05, 1.0 - x);
        let h = (h + dy).clamp(0.08, 1.0 - y);
        [x, y, w, h]
    } else {
        [(x + dx).clamp(0.0, 1.0 - w), (y + dy).clamp(0.0, 1.0 - h), w, h]
    };
    if snap { snapped(raw) } else { raw }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drag(resize: bool) -> ZoneDrag {
        ZoneDrag {
            zone: 0,
            resize,
            start: [0.0, 0.0, 0.25, 0.5],
            from: (0.0, 0.0),
        }
    }

    #[test]
    fn a_moved_zone_snaps_to_whole_cells_and_stays_on_the_screen() {
        let size = (400.0, 200.0);
        let moved = zone_after_drag(drag(false), (53.0, 41.0), size, true);
        // 53/400 = 0.1325 of the width: 4 cells of 1/32 = 0.125; 41/200 = 0.205: 3 cells of 1/16 = 0.1875.
        assert_eq!(moved, [0.125, 0.1875, 0.25, 0.5]);
        let off = zone_after_drag(drag(false), (900.0, 900.0), size, true);
        assert_eq!(off, [0.75, 0.5, 0.25, 0.5]);
    }

    #[test]
    fn a_resized_zone_keeps_its_corner_and_a_two_cell_minimum() {
        let size = (400.0, 200.0);
        let grown = zone_after_drag(drag(true), (40.0, 30.0), size, true);
        assert_eq!(grown, [0.0, 0.0, 0.34375, 0.625]);
        let tiny = zone_after_drag(drag(true), (-399.0, -199.0), size, true);
        assert_eq!(tiny, [0.0, 0.0, 0.0625, 0.125]);
    }
}
