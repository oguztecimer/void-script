use std::collections::HashMap;
use std::time::Duration;

use tiny_skia::{ColorU8, Pixmap, PremultipliedColorU8};

use crate::animation::AnimationPlayer;

// ---------------------------------------------------------------------------
// 3x5 bitmap font for digits 0-9
// ---------------------------------------------------------------------------

const DIGITS_3X5: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

fn set_px(canvas: &mut Pixmap, x: i32, y: i32, color: PremultipliedColorU8) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    if x >= 0 && x < w && y >= 0 && y < h {
        canvas.pixels_mut()[(y * w + x) as usize] = color;
    }
}

fn draw_digit(canvas: &mut Pixmap, digit: u8, x: i32, y: i32, sz: i32, color: PremultipliedColorU8) {
    if digit > 9 { return; }
    let glyph = &DIGITS_3X5[digit as usize];
    for (row, &bits) in glyph.iter().enumerate() {
        for col in 0..3i32 {
            if bits & (1 << (2 - col)) != 0 {
                for py in 0..sz {
                    for px in 0..sz {
                        set_px(canvas, x + col * sz + px, y + row as i32 * sz + py, color);
                    }
                }
            }
        }
    }
}

fn draw_number(canvas: &mut Pixmap, n: u32, center_x: i32, y: i32, sz: i32, color: PremultipliedColorU8) {
    let s = n.to_string();
    let digit_count = s.len() as i32;
    let char_w = 3 * sz + sz;
    let total_w = digit_count * char_w - sz;
    let start_x = center_x - total_w / 2;
    for (i, ch) in s.chars().enumerate() {
        let d = (ch as u8) - b'0';
        draw_digit(canvas, d, start_x + i as i32 * char_w, y, sz, color);
    }
}

pub const WORLD_WIDTH: u32 = 1000;

pub type UnitId = u64;

pub struct MovementState {
    pub target_x: f32,
    pub speed: f32,
}

pub struct Unit {
    pub id: UnitId,
    pub name: String,
    pub animation: AnimationPlayer,
    pub x: f32,
    pub y: f32,
    /// X pivot offset in frame pixels (0 = left edge, frame_width/2 = center).
    pub pivot_x: f32,
    /// Y pivot offset in frame pixels (0 = bottom edge, frame_height = top).
    pub pivot_y: f32,
    pub movement: Option<MovementState>,
    pub z_order: i32,
    pub visible: bool,
}

pub struct UnitManager {
    units: HashMap<UnitId, Unit>,
    next_id: UnitId,
    time: f32,
}

impl UnitManager {
    pub fn new() -> Self {
        Self {
            units: HashMap::new(),
            next_id: 0,
            time: 0.0,
        }
    }

    pub fn spawn(
        &mut self,
        name: &str,
        png_bytes: &[u8],
        json_str: &str,
        x: f32,
        pivot_x: f32,
        pivot_y: f32
    ) -> UnitId {
        let id = self.next_id;
        self.next_id += 1;

        let animation = AnimationPlayer::from_bytes(png_bytes, json_str);

        self.units.insert(id, Unit {
            id,
            name: name.to_string(),
            animation,
            x: x.clamp(0.0, WORLD_WIDTH as f32),
            y: 0.0,
            pivot_x,
            pivot_y,
            movement: None,
            z_order: 0,
            visible: true,
        });

        id
    }

    pub fn destroy(&mut self, id: UnitId) -> bool {
        self.units.remove(&id).is_some()
    }

    pub fn move_to(&mut self, id: UnitId, target_x: f32, speed: f32) {
        if let Some(unit) = self.units.get_mut(&id) {
            let clamped = target_x.clamp(0.0, WORLD_WIDTH as f32);
            unit.movement = Some(MovementState { target_x: clamped, speed });
        }
    }

    pub fn stop(&mut self, id: UnitId) {
        if let Some(unit) = self.units.get_mut(&id) {
            unit.movement = None;
        }
    }

    pub fn play_animation(&mut self, id: UnitId, name: &str) {
        if let Some(unit) = self.units.get_mut(&id) {
            unit.animation.play(name);
        }
    }

    pub fn set_facing(&mut self, id: UnitId, left: bool) {
        if let Some(unit) = self.units.get_mut(&id) {
            unit.animation.facing_left = left;
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        let dt = delta.as_secs_f32();
        self.time += dt;

        for unit in self.units.values_mut() {
            unit.animation.tick(delta);

            if let Some(movement) = &unit.movement {
                let dx = movement.target_x - unit.x;
                let step = movement.speed * dt;

                if dx.abs() <= step {
                    unit.x = movement.target_x;
                    unit.movement = None;
                    unit.animation.play("idle");
                } else if dx > 0.0 {
                    unit.x = (unit.x + step).min(WORLD_WIDTH as f32);
                    unit.animation.facing_left = false;
                    if unit.animation.current_animation() != "walk" {
                        unit.animation.play("walk");
                    }
                } else {
                    unit.x = (unit.x - step).max(0.0);
                    unit.animation.facing_left = true;
                    if unit.animation.current_animation() != "walk" {
                        unit.animation.play("walk");
                    }
                }
            }
        }
    }

    pub fn draw_all(&self, canvas: &mut Pixmap, strip_height: u32, pixel_scale: u32, dock_height: u32) {
        let scale = pixel_scale as f32;
        let screen_width = canvas.width();
        let world_px = WORLD_WIDTH * pixel_scale;
        let offset_x = (screen_width as i32 - world_px as i32) / 2;

        // Draw units.
        let mut sorted: Vec<&Unit> = self.units.values()
            .filter(|u| u.visible)
            .collect();
        sorted.sort_by_key(|u| (u.z_order, u.id));

        for unit in sorted {
            let us = scale * 2.0;
            // y is the bottom base in world coords; world 0 aligns with dock/taskbar top.
            let base_y = strip_height as i32 - dock_height as i32 - ((unit.y + 20.0) * scale) as i32;
            let sh = (unit.animation.frame_height() as f32 * us) as i32;
            let draw_y = base_y - sh + (unit.pivot_y * us) as i32;
            let x = offset_x + (unit.x * scale) as i32 - (unit.pivot_x * us) as i32;
            let reflection_y = base_y - sh;
            unit.animation.draw_reflection(canvas, x, reflection_y, 0.4, us);
            unit.animation.draw(canvas, x, draw_y, us);
        }

        // Collect unit x positions for proximity-based ruler opacity.
        let unit_xs: Vec<f32> = self.units.values()
            .filter(|u| u.visible)
            .map(|u| u.x)
            .collect();

        // Draw grid on top of everything.
        Self::draw_grid(canvas, strip_height, pixel_scale, offset_x, dock_height, &unit_xs);
    }

    fn draw_grid(canvas: &mut Pixmap, strip_height: u32, pixel_scale: u32, offset_x: i32, dock_height: u32, unit_xs: &[f32]) {
        let scale = pixel_scale as i32;
        let line_count = (WORLD_WIDTH / 20) as i32;

        // Font size scales with both pixel_scale and DPI (canvas width / logical world pixels).
        // This ensures numbers are visually the same size on all DPI displays.
        let canvas_scale = (canvas.width() as f32) / (WORLD_WIDTH as f32 * pixel_scale as f32);
        let sz = ((scale as f32 * canvas_scale) as i32).max(2);

        let ground_y = strip_height as i32 - (dock_height) as i32;
        let font_h = 5 * sz;
        let text_y = ground_y - font_h;
        let full_radius = 20.0_f32;
        let fade_radius = 60.0_f32;

        for i in 0..=line_count {
            let x = offset_x + i * 20 * scale;
            let world_x = i as f32 * 20.0;

            let nearest = unit_xs.iter()
                .map(|&ux| (ux - world_x).abs())
                .fold(f32::MAX, f32::min);
            let opacity = if nearest <= full_radius {
                1.0
            } else if nearest >= fade_radius {
                0.0
            } else {
                1.0 - (nearest - full_radius) / (fade_radius - full_radius)
            };

            let is_major = i % 2 == 0;
            let c = if is_major {
                ColorU8::from_rgba(220, 210, 190, (opacity * 255.0) as u8).premultiply()
            } else {
                ColorU8::from_rgba(220, 200, 80, (opacity * 200.0) as u8).premultiply()
            };
            let shadow = ColorU8::from_rgba(0, 0, 0, (opacity * 180.0) as u8).premultiply();

            draw_number(canvas, i as u32, x + 1, text_y + 1, sz, shadow);
            draw_number(canvas, i as u32, x, text_y, sz, c);
        }
    }

    pub fn get(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(&id)
    }

    pub fn get_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Unit> {
        self.units.values()
    }
}
