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

pub const DEFAULT_WORLD_WIDTH: u32 = 1000;

pub type UnitId = u64;

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
    /// Previous x position (for walk/idle animation transitions).
    pub prev_x: f32,
    pub z_order: i32,
    pub visible: bool,
    /// Marked for removal after death animation finishes.
    pub pending_destroy: bool,
    /// Current opacity (1.0 = fully visible, fades to 0.0 during death).
    pub opacity: f32,
    /// Sim ticks elapsed since kill() was called.
    pub death_timer: u32,
    /// Total ticks of the death animation (0 if no death anim).
    pub death_anim_ticks: u32,
}

pub struct UnitManager {
    units: HashMap<UnitId, Unit>,
    next_id: UnitId,
    time: f32,
    pub world_width: u32,
}

impl UnitManager {
    pub fn new() -> Self {
        Self {
            units: HashMap::new(),
            next_id: 0,
            time: 0.0,
            world_width: DEFAULT_WORLD_WIDTH,
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

        let mut animation = AnimationPlayer::from_bytes(png_bytes, json_str);
        animation.play("spawn");

        let clamped_x = x.clamp(0.0, self.world_width as f32);
        self.units.insert(id, Unit {
            id,
            name: name.to_string(),
            animation,
            x: clamped_x,
            y: 0.0,
            pivot_x,
            pivot_y,
            prev_x: clamped_x,
            z_order: 0,
            visible: true,
            pending_destroy: false,
            opacity: 1.0,
            death_timer: 0,
            death_anim_ticks: 0,
        });

        id
    }

    pub fn destroy(&mut self, id: UnitId) -> bool {
        self.units.remove(&id).is_some()
    }

    pub fn move_to(&mut self, id: UnitId, target_x: f32, _speed: f32) {
        if let Some(unit) = self.units.get_mut(&id) {
            unit.x = target_x.clamp(0.0, self.world_width as f32);
        }
    }

    pub fn play_animation(&mut self, id: UnitId, name: &str) {
        if let Some(unit) = self.units.get_mut(&id) {
            unit.animation.play(name);
        }
    }

    /// Play the death animation and mark the unit for removal.
    /// After the death animation finishes, the corpse lingers for 30 ticks,
    /// then fades out over 30 ticks before being removed.
    /// If no "death" animation exists, the unit is destroyed immediately.
    pub fn kill(&mut self, id: UnitId) {
        if let Some(unit) = self.units.get_mut(&id) {
            unit.pending_destroy = true;
            unit.animation.hold_on_finish = true;
            unit.animation.play("death");
            // If the animation didn't switch (no "death" anim), destroy now.
            if !unit.animation.is_action_playing() {
                self.units.remove(&id);
            } else {
                unit.death_anim_ticks = unit.animation.animation_duration_ticks("death") as u32;
            }
        }
    }

    /// Remove units that have fully faded out after death.
    pub fn reap_dead(&mut self) {
        self.units.retain(|_, unit| {
            !(unit.pending_destroy && unit.opacity <= 0.0)
        });
    }

    pub fn set_facing(&mut self, id: UnitId, left: bool) {
        if let Some(unit) = self.units.get_mut(&id) {
            unit.animation.facing_left = left;
        }
    }

    /// Per-frame update (render-driven).
    pub fn tick(&mut self, delta: Duration) {
        let dt = delta.as_secs_f32();
        self.time += dt;
    }

    /// Advance all animations by one sim tick (sim-driven, deterministic).
    pub fn tick_animations(&mut self) {
        const LINGER_TICKS: u32 = 30;
        const FADE_TICKS: u32 = 30;

        for unit in self.units.values_mut() {
            unit.animation.tick();

            // Walk/idle transitions based on position delta (sim-tick driven).
            if !unit.animation.is_action_playing() && !unit.pending_destroy {
                let dx = unit.x - unit.prev_x;
                let moved = dx.abs() > 0.01;

                if moved && unit.animation.current_animation() == "idle" {
                    unit.animation.play("walk");
                    unit.animation.facing_left = dx < 0.0;
                } else if !moved && unit.animation.current_animation() == "walk" {
                    unit.animation.play("idle");
                } else if moved {
                    unit.animation.facing_left = dx < 0.0;
                }
            }
            unit.prev_x = unit.x;

            if unit.pending_destroy {
                unit.death_timer += 1;
                let fade_start = unit.death_anim_ticks + LINGER_TICKS;
                if unit.death_timer >= fade_start {
                    let fade_elapsed = unit.death_timer - fade_start;
                    unit.opacity = 1.0 - fade_elapsed as f32 / FADE_TICKS as f32;
                    unit.opacity = unit.opacity.max(0.0);
                }
            }
        }
    }

    pub fn draw_all(&self, canvas: &mut Pixmap, strip_height: u32, pixel_scale: u32, dock_height: u32) {
        // Nudge everything up by 2px on Windows to align with taskbar edge.
        #[cfg(target_os = "windows")]
        let dock_height = dock_height + 2;
        let scale = pixel_scale as f32;
        let screen_width = canvas.width();
        let world_px = self.world_width * pixel_scale;
        let offset_x = (screen_width as i32 - world_px as i32) / 2;

        // Draw units.
        let mut sorted: Vec<&Unit> = self.units.values()
            .filter(|u| u.visible)
            .collect();
        sorted.sort_by_key(|u| (u.z_order, u.id));

        for unit in sorted {
            let us = scale * 2.0;
            // y is the bottom base in world coords; world 0 aligns with dock/taskbar top.
            let base_y = strip_height as i32 - dock_height as i32 - 20 - ((unit.y + 20.0) * scale) as i32;
            let sh = (unit.animation.frame_height() as f32 * us) as i32;
            let draw_y = base_y - sh + (unit.pivot_y * us) as i32;
            // Mirror pivot_x when facing left so the anchor stays at the entity's world position.
            let effective_pivot_x = if unit.animation.facing_left {
                unit.animation.frame_width() as f32 - unit.pivot_x
            } else {
                unit.pivot_x
            };
            let x = offset_x + (unit.x * scale) as i32 - (effective_pivot_x * us) as i32;
            let reflection_y = base_y - sh;
            unit.animation.draw_reflection(canvas, x, reflection_y, 0.4 * unit.opacity, us);
            unit.animation.draw(canvas, x, draw_y, us, unit.opacity);
        }

        // Collect unit x positions for proximity-based ruler opacity.
        let unit_xs: Vec<f32> = self.units.values()
            .filter(|u| u.visible)
            .map(|u| u.x)
            .collect();

        // Draw grid on top of everything.
        Self::draw_grid(canvas, strip_height, pixel_scale, offset_x, dock_height, &unit_xs, self.world_width);
    }

    fn draw_grid(canvas: &mut Pixmap, strip_height: u32, pixel_scale: u32, offset_x: i32, dock_height: u32, unit_xs: &[f32], world_width: u32) {
        let scale = pixel_scale as i32;
        let line_count = (world_width / 20) as i32;

        // Font size scales with both pixel_scale and DPI (canvas width / logical world pixels).
        // This ensures numbers are visually the same size on all DPI displays.
        let canvas_scale = (canvas.width() as f32) / (world_width as f32 * pixel_scale as f32);
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
