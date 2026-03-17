use std::collections::HashMap;
use std::time::Duration;

use tiny_skia::Pixmap;

use crate::animation::AnimationPlayer;

pub const WORLD_WIDTH: u32 = 500;

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
    ) -> UnitId {
        let id = self.next_id;
        self.next_id += 1;

        let animation = AnimationPlayer::from_bytes(png_bytes, json_str);

        self.units.insert(id, Unit {
            id,
            name: name.to_string(),
            animation,
            x: x.clamp(0.0, WORLD_WIDTH as f32),
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

    pub fn draw_all(&self, canvas: &mut Pixmap, strip_height: u32, pixel_scale: u32) {
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
            let sh = (unit.animation.frame_height() as f32 * scale) as i32;
            let y = strip_height as i32 - sh * 2;
            let x = offset_x + (unit.x * scale) as i32;
            unit.animation.draw_reflection(canvas, x, y, 0.4, scale);
            unit.animation.draw(canvas, x, y, scale);
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
