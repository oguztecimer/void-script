//! Sprite animation system.
//!
//! Provides `AnimationPlayer`, which:
//! - Decodes an atlas PNG once at construction (zero per-frame decode overhead)
//! - Pre-extracts all animation frames into `Vec<Vec<Pixmap>>` at load time
//! - Advances frames based on tick counts (each frame specifies how many ticks it lasts)
//! - Supports horizontal flip via Transform for left-facing rendering

use std::collections::HashMap;

use tiny_skia::{IntRect, Pixmap, PixmapPaint, PixmapRef, Transform};

// ---------------------------------------------------------------------------
// Embedded assets
// ---------------------------------------------------------------------------

/// Embedded skeleton sprite atlas PNG (compile-time).
pub const SKELETON_ATLAS_PNG: &[u8] = include_bytes!("assets/skeleton_atlas.png");

/// Embedded summoner sprite atlas PNG (compile-time).
pub const SUMMONER_ATLAS_PNG: &[u8] = include_bytes!("assets/summoner_atlas.png");

/// Load the skeleton atlas JSON metadata.
pub fn skeleton_atlas_json() -> String {
    #[cfg(debug_assertions)]
    {
        if let Ok(s) = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/assets/skeleton_atlas.json")
        ) {
            return s;
        }
    }
    include_str!("assets/skeleton_atlas.json").to_owned()
}

/// Load the summoner atlas JSON metadata.
pub fn summoner_atlas_json() -> String {
    #[cfg(debug_assertions)]
    {
        if let Ok(s) = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/assets/summoner_atlas.json")
        ) {
            return s;
        }
    }
    include_str!("assets/summoner_atlas.json").to_owned()
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize, Clone)]
pub struct AtlasMetadata {
    pub frame_width: u32,
    pub frame_height: u32,
    pub animations: Vec<AnimationDef>,
}

#[derive(serde::Deserialize, Clone)]
pub struct AnimationDef {
    pub name: String,
    pub row: u32,
    pub frames: Vec<FrameDef>,
    pub loop_mode: LoopMode,
}

#[derive(serde::Deserialize, Clone)]
pub struct FrameDef {
    pub col: u32,
    pub ticks: u64,
}

#[derive(serde::Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum LoopMode {
    Loop,
    PlayOnce,
}

// ---------------------------------------------------------------------------
// AnimationPlayer
// ---------------------------------------------------------------------------

/// Tick-driven animation player backed by a pre-extracted frame atlas.
///
/// All frames are extracted from the atlas PNG at construction time
/// (approximately 375 KB for 40 frames at 48×48×4 bytes). No allocations
/// occur during tick or draw.
pub struct AnimationPlayer {
    /// Pre-extracted frames: `frames[anim_idx][frame_idx]` → Pixmap.
    frames: Vec<Vec<Pixmap>>,
    /// Name → index into `frames` / `defs`.
    anim_index: HashMap<String, usize>,
    /// Full animation definitions (needed for duration and loop_mode).
    defs: Vec<AnimationDef>,
    /// Pixel dimensions of a single frame.
    frame_width: u32,
    frame_height: u32,
    /// Index of the currently playing animation.
    current_anim_idx: usize,
    /// Index of the current frame within that animation.
    current_frame_idx: usize,
    /// Accumulated sim ticks since the last frame advance.
    elapsed_ticks: u64,
    /// When `true`, the sprite is drawn mirrored horizontally (facing left).
    pub facing_left: bool,
    /// When `true`, PlayOnce animations hold on their last frame instead of
    /// transitioning back to idle.
    pub hold_on_finish: bool,
}

impl AnimationPlayer {
    /// Load the atlas PNG and JSON metadata, pre-extract all frames.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The PNG cannot be decoded
    /// - The JSON is malformed
    /// - Any animation frame rect falls outside the atlas bounds
    /// - The "idle" animation is not present in the metadata
    /// Load the embedded skeleton atlas (default).
    pub fn new() -> Self {
        Self::from_bytes(SKELETON_ATLAS_PNG, &skeleton_atlas_json())
    }

    /// Construct from arbitrary atlas PNG bytes and JSON metadata string.
    pub fn from_bytes(png_bytes: &[u8], json_str: &str) -> Self {
        let atlas =
            Pixmap::decode_png(png_bytes).expect("AnimationPlayer: failed to decode atlas PNG");

        let meta: AtlasMetadata =
            serde_json::from_str(json_str).expect("AnimationPlayer: failed to parse atlas JSON");

        let atlas_w = atlas.width();
        let atlas_h = atlas.height();

        // Pre-extract every frame from the atlas into its own Pixmap.
        let mut all_frames: Vec<Vec<Pixmap>> = Vec::with_capacity(meta.animations.len());

        for anim in &meta.animations {
            let mut anim_frames: Vec<Pixmap> = Vec::with_capacity(anim.frames.len());
            for (frame_idx, frame_def) in anim.frames.iter().enumerate() {
                let x = frame_def.col * meta.frame_width;
                let y = anim.row * meta.frame_height;
                let w = meta.frame_width;
                let h = meta.frame_height;

                let rect = IntRect::from_xywh(x as i32, y as i32, w, h).unwrap_or_else(|| {
                    panic!(
                        "Animation '{}' frame {}: invalid rect ({},{},{},{})",
                        anim.name, frame_idx, x, y, w, h
                    )
                });

                let frame_pixmap = atlas.as_ref().clone_rect(rect).unwrap_or_else(|| {
                    panic!(
                        "Animation '{}' frame {}: rect ({},{},{},{}) outside atlas bounds ({}x{})",
                        anim.name, frame_idx, x, y, w, h, atlas_w, atlas_h
                    )
                });

                anim_frames.push(frame_pixmap);
            }
            all_frames.push(anim_frames);
        }

        // Build name → index map.
        let mut anim_index: HashMap<String, usize> = HashMap::new();
        for (i, anim) in meta.animations.iter().enumerate() {
            anim_index.insert(anim.name.clone(), i);
        }

        // Start on "idle".
        let idle_idx = *anim_index
            .get("idle")
            .expect("AnimationPlayer: 'idle' animation not found in atlas metadata");

        Self {
            frames: all_frames,
            anim_index,
            defs: meta.animations,
            frame_width: meta.frame_width,
            frame_height: meta.frame_height,
            current_anim_idx: idle_idx,
            current_frame_idx: 0,
            elapsed_ticks: 0,
            facing_left: false,
            hold_on_finish: false,
        }
    }

    // -----------------------------------------------------------------------
    // Tick
    // -----------------------------------------------------------------------

    /// Advance the animation by one simulation tick.
    ///
    /// Frame durations are specified in ticks. Called once per sim tick
    /// for deterministic animation timing.
    /// On `LoopMode::PlayOnce`, auto-transitions back to "idle" when done.
    pub fn tick(&mut self) {
        self.elapsed_ticks += 1;

        let anim = &self.defs[self.current_anim_idx];
        let frame_ticks = anim.frames[self.current_frame_idx].ticks;

        if self.elapsed_ticks >= frame_ticks {
            self.elapsed_ticks = 0;
            let total_frames = anim.frames.len();

            match anim.loop_mode {
                LoopMode::Loop => {
                    self.current_frame_idx = (self.current_frame_idx + 1) % total_frames;
                }
                LoopMode::PlayOnce => {
                    if self.current_frame_idx + 1 < total_frames {
                        self.current_frame_idx += 1;
                    } else if self.hold_on_finish {
                        // Stay on last frame — don't transition to idle.
                    } else {
                        // Finished — transition back to idle.
                        let idle_idx = *self
                            .anim_index
                            .get("idle")
                            .expect("AnimationPlayer: 'idle' not found during PlayOnce transition");
                        self.current_anim_idx = idle_idx;
                        self.current_frame_idx = 0;
                        self.elapsed_ticks = 0;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Playback control
    // -----------------------------------------------------------------------

    /// Switch to the named animation, resetting to frame 0.
    ///
    /// If the animation is already playing, this is a no-op (prevents restart).
    /// If the animation name is not found, logs a warning and does nothing.
    pub fn play(&mut self, name: &str) {
        let Some(&idx) = self.anim_index.get(name) else {
            eprintln!("[animation] warning: '{}' not found, ignoring", name);
            return;
        };
        if idx == self.current_anim_idx {
            return; // Already playing — do nothing.
        }
        self.current_anim_idx = idx;
        self.current_frame_idx = 0;
        self.elapsed_ticks = 0;
    }

    /// Returns the name of the currently playing animation.
    pub fn current_animation(&self) -> &str {
        &self.defs[self.current_anim_idx].name
    }

    /// Returns true if a PlayOnce animation is currently playing (not yet finished).
    pub fn is_action_playing(&self) -> bool {
        let anim = &self.defs[self.current_anim_idx];
        anim.loop_mode == LoopMode::PlayOnce
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    /// Returns a reference to the current frame's pre-extracted `Pixmap`.
    ///
    /// No allocation — returns a reference into the pre-extracted frame vec.
    pub fn current_frame_pixmap(&self) -> &Pixmap {
        &self.frames[self.current_anim_idx][self.current_frame_idx]
    }

    /// Draw the current frame onto `canvas` at `(dst_x, dst_y)` with the given opacity (0.0–1.0).
    ///
    /// If `self.facing_left` is `true`, applies a horizontal mirror transform.
    pub fn draw(&self, canvas: &mut Pixmap, dst_x: i32, dst_y: i32, scale: f32, opacity: f32) {
        let frame = self.current_frame_pixmap();
        let sw = (self.frame_width as f32 * scale) as i32;

        let transform = if self.facing_left {
            Transform::from_scale(-scale, scale)
                .post_translate((dst_x + sw) as f32, dst_y as f32)
        } else {
            Transform::from_scale(scale, scale)
                .post_translate(dst_x as f32, dst_y as f32)
        };

        let mut paint = PixmapPaint::default();
        paint.opacity = opacity;

        canvas.draw_pixmap(
            0,
            0,
            PixmapRef::from_bytes(frame.data(), frame.width(), frame.height())
                .expect("AnimationPlayer::draw: invalid pixmap"),
            &paint,
            transform,
            None,
        );
    }

    /// Draw a vertically-flipped reflection with the given opacity and scale.
    pub fn draw_reflection(&self, canvas: &mut Pixmap, dst_x: i32, dst_y: i32, opacity: f32, scale: f32) {
        let frame = self.current_frame_pixmap();
        let sh = (self.frame_height as f32 * scale) as i32;
        let sw = (self.frame_width as f32 * scale) as i32;

        let transform = if self.facing_left {
            Transform::from_scale(-scale, -scale)
                .post_translate((dst_x + sw) as f32, (dst_y + sh * 2) as f32)
        } else {
            Transform::from_scale(scale, -scale)
                .post_translate(dst_x as f32, (dst_y + sh * 2) as f32)
        };

        let mut paint = PixmapPaint::default();
        paint.opacity = opacity;

        canvas.draw_pixmap(
            0,
            0,
            PixmapRef::from_bytes(frame.data(), frame.width(), frame.height())
                .expect("AnimationPlayer::draw_reflection: invalid pixmap"),
            &paint,
            transform,
            None,
        );
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Width of a single sprite frame in pixels.
    pub fn frame_width(&self) -> u32 {
        self.frame_width
    }

    /// Height of a single sprite frame in pixels.
    pub fn frame_height(&self) -> u32 {
        self.frame_height
    }

    /// Total ticks for a named animation, or 0 if not found.
    pub fn animation_duration_ticks(&self, name: &str) -> u64 {
        let Some(&idx) = self.anim_index.get(name) else { return 0 };
        self.defs[idx].frames.iter().map(|f| f.ticks).sum()
    }

    /// Whether the current animation is a resting state (idle/sleep).
    pub fn is_resting(&self) -> bool {
        let name = self.current_animation();
        name == "idle" || name == "sleep"
    }
}

/// Compute the total ticks of the "spawn" animation from atlas JSON metadata.
/// Returns 0 if no "spawn" animation exists.
pub fn spawn_animation_ticks(json_str: &str) -> i64 {
    let meta: AtlasMetadata = match serde_json::from_str(json_str) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    meta.animations
        .iter()
        .find(|a| a.name == "spawn")
        .map(|a| a.frames.iter().map(|f| f.ticks as i64).sum())
        .unwrap_or(0)
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self::new()
    }
}
