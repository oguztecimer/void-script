//! Sprite animation system for good-boi.
//!
//! Provides `AnimationPlayer`, which:
//! - Decodes the dog atlas PNG once at startup (zero per-frame decode overhead)
//! - Pre-extracts all animation frames into `Vec<Vec<Pixmap>>` at load time
//! - Advances frames based on wall-clock time (`Duration`) — animation speed is
//!   independent of render frame rate
//! - Supports horizontal flip via Transform for left-facing rendering
//!
//! The module is wired into `App` and `Renderer` as of Plan 02-02.

use std::collections::HashMap;
use std::time::Duration;

use tiny_skia::{IntRect, Pixmap, PixmapPaint, PixmapRef, Transform};

// ---------------------------------------------------------------------------
// Embedded assets
// ---------------------------------------------------------------------------

const ATLAS_PNG: &[u8] = include_bytes!("assets/dog_atlas.png");

/// Load the JSON metadata. In debug builds, prefer a local file override so
/// that the JSON can be edited without recompiling. In release, use the
/// embedded string baked in at compile time.
fn load_json_str() -> String {
    #[cfg(debug_assertions)]
    {
        // Try to load from the workspace-relative path first (dev override).
        if let Ok(s) = std::fs::read_to_string("src/assets/dog_atlas.json") {
            return s;
        }
    }
    include_str!("assets/dog_atlas.json").to_owned()
}

// ---------------------------------------------------------------------------
// Data structures (Serde-deserializable from dog_atlas.json)
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
    pub duration_ms: u64,
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
    /// Accumulated time since the last frame advance.
    elapsed: Duration,
    /// When `true`, the sprite is drawn mirrored horizontally (facing left).
    pub facing_left: bool,
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
    pub fn new() -> Self {
        // Decode atlas PNG once.
        let atlas =
            Pixmap::decode_png(ATLAS_PNG).expect("AnimationPlayer: failed to decode dog_atlas.png");

        // Parse JSON metadata.
        let json = load_json_str();
        let meta: AtlasMetadata =
            serde_json::from_str(&json).expect("AnimationPlayer: failed to parse dog_atlas.json");

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
            elapsed: Duration::ZERO,
            facing_left: false,
        }
    }

    // -----------------------------------------------------------------------
    // Tick
    // -----------------------------------------------------------------------

    /// Advance the animation by `delta` wall-clock time.
    ///
    /// Uses `Duration` arithmetic so animation speed is independent of render
    /// frame rate. On `LoopMode::PlayOnce`, holds the last frame then
    /// auto-transitions back to "idle".
    pub fn tick(&mut self, delta: Duration) {
        self.elapsed += delta;

        let anim = &self.defs[self.current_anim_idx];
        let frame_duration =
            Duration::from_millis(anim.frames[self.current_frame_idx].duration_ms);

        if self.elapsed >= frame_duration {
            self.elapsed -= frame_duration;
            let total_frames = anim.frames.len();

            match anim.loop_mode {
                LoopMode::Loop => {
                    self.current_frame_idx = (self.current_frame_idx + 1) % total_frames;
                }
                LoopMode::PlayOnce => {
                    if self.current_frame_idx + 1 < total_frames {
                        self.current_frame_idx += 1;
                    } else {
                        // Finished — transition back to idle.
                        let idle_idx = *self
                            .anim_index
                            .get("idle")
                            .expect("AnimationPlayer: 'idle' not found during PlayOnce transition");
                        self.current_anim_idx = idle_idx;
                        self.current_frame_idx = 0;
                        self.elapsed = Duration::ZERO;
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
    ///
    /// # Panics
    ///
    /// Panics if `name` is not present in the atlas metadata.
    pub fn play(&mut self, name: &str) {
        let idx = *self.anim_index.get(name).unwrap_or_else(|| {
            panic!("AnimationPlayer::play: animation '{}' not found", name)
        });
        if idx == self.current_anim_idx {
            return; // Already playing — do nothing.
        }
        self.current_anim_idx = idx;
        self.current_frame_idx = 0;
        self.elapsed = Duration::ZERO;
    }

    /// Returns the name of the currently playing animation.
    pub fn current_animation(&self) -> &str {
        &self.defs[self.current_anim_idx].name
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

    /// Draw the current frame onto `canvas` at `(dst_x, dst_y)`.
    ///
    /// If `self.facing_left` is `true`, applies a horizontal mirror transform
    /// so the dog faces left without needing separate left-facing art.
    pub fn draw(&self, canvas: &mut Pixmap, dst_x: i32, dst_y: i32) {
        let frame = self.current_frame_pixmap();
        let transform = if self.facing_left {
            // Mirror around the frame's vertical centre, then translate to destination.
            // scale(-1, 1) flips horizontally; post_translate shifts it into place.
            Transform::from_scale(-1.0, 1.0)
                .post_translate(
                    (dst_x + self.frame_width as i32) as f32,
                    dst_y as f32,
                )
        } else {
            Transform::from_translate(dst_x as f32, dst_y as f32)
        };

        canvas.draw_pixmap(
            0,
            0,
            PixmapRef::from_bytes(frame.data(), frame.width(), frame.height())
                .expect("AnimationPlayer::draw: invalid pixmap"),
            &PixmapPaint::default(),
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

    /// Suggested render interval for the current animation.
    ///
    /// Returns 30 FPS (33 ms) for active animations, 10 FPS (100 ms) for
    /// `idle` and `sleep` (per user decision — lower rate for resting states).
    pub fn desired_frame_interval(&self) -> Duration {
        let name = self.current_animation();
        if name == "idle" || name == "sleep" {
            Duration::from_millis(100) // 10 FPS
        } else {
            Duration::from_millis(33) // ~30 FPS
        }
    }
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self::new()
    }
}
