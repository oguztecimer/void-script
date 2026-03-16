//! Ball engine for good-boi.
//!
//! Provides `FetchEngine` — ball physics simulation with gravity, bouncing, air resistance,
//! and rolling friction; a dog chase AI that runs directly toward the ball;
//! and a finite state machine (FetchState) that drives the full ball interaction.
//!
//! The ball lives in the strip (no fullscreen mode). BehaviorEngine drives the dog
//! when the ball is idle (WaitingForThrow); FetchEngine takes over during active states.

use std::time::Instant;

use crate::animation::AnimationPlayer;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Radius of the ball in logical pixels.
const BALL_RADIUS: f32 = 6.0;

/// Dog sprite frame height (used for dog_position() y calculation).
const DOG_FRAME_HEIGHT: f32 = 48.0;

/// Dog chase speed in pixels per second.
const DOG_CHASE_SPEED: f32 = 180.0;

/// Dog return (trot) speed in pixels per second.
const DOG_TROT_SPEED: f32 = 60.0;

/// Dog pickup walk speed (slow sad walk toward missed ball).
const DOG_PICKUP_SPEED: f32 = 40.0;

/// Radius within which the dog catches the ball.
const CATCH_RADIUS: f32 = 24.0;

/// Maximum throw speed (px/s).
const MAX_THROW_SPEED: f32 = 2000.0;

// ---------------------------------------------------------------------------
// Ball
// ---------------------------------------------------------------------------

/// Physical state of the fetch ball.
pub struct Ball {
    /// X position in logical pixels.
    pub x: f32,
    /// Y position in logical pixels.
    pub y: f32,
    /// X velocity in pixels per second.
    pub vx: f32,
    /// Y velocity in pixels per second.
    pub vy: f32,
    /// Number of times the ball has bounced off the ground (0 = still mid-air = perfect catch window).
    pub bounce_count: u32,
}

// ---------------------------------------------------------------------------
// FetchState FSM
// ---------------------------------------------------------------------------

/// State machine driving the ball interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum FetchState {
    /// Ball is resting, waiting for the user to grab and throw.
    WaitingForThrow,
    /// User has clicked and is dragging the ball. Ball position is set externally by App.
    BallGrabbed,
    /// Ball has been thrown and is in flight (or rolling on the ground).
    BallInFlight,
    /// Dog is actively running toward the ball.
    DogChasing,
    /// Dog has caught up to the ball — short celebration before trotting.
    DogCatching { timer: f32 },
    /// Dog is trotting around excitedly with the ball before dropping it.
    ProudTrot { leg_timer: f32, leg_duration: f32, legs_remaining: u8, going_right: bool },
    /// Dog reacts sadly to missing the ball.
    MissReaction { timer: f32 },
    /// Dog is walking slowly toward the missed ball to pick it up.
    MissPickup { timer: f32 },
    /// Dog caught the ball mid-air (bounce_count == 0) — extended celebration.
    Celebration { timer: f32 },
}

// ---------------------------------------------------------------------------
// FetchTickResult
// ---------------------------------------------------------------------------

/// Return value from `FetchEngine::tick()` telling the caller what happened this frame.
pub enum FetchTickResult {
    /// Nothing notable happened — continue rendering normally.
    Continue,
    /// The dog caught the ball — add the given happiness to the pet's stats.
    CatchHappiness(f32),
}

// ---------------------------------------------------------------------------
// FetchEngine
// ---------------------------------------------------------------------------

/// Ball engine for the strip.
///
/// Drives ball physics, chase AI, and the full FetchState FSM.
/// The ball lives in the strip — no fullscreen mode.
pub struct FetchEngine {
    /// The ball's current physical state.
    pub ball: Ball,
    /// Current FSM state.
    pub state: FetchState,

    // Dog
    /// Dog X position in logical pixels.
    pub dog_x: f32,
    /// Whether the dog sprite is facing left.
    pub dog_facing_left: bool,

    // Strip dimensions
    /// Width of the strip window in logical pixels.
    pub strip_width: f32,
    /// Height of the strip window in logical pixels.
    pub strip_height: f32,

    // Throw tracking
    /// True while the user is holding the ball.
    pub ball_grabbed: bool,
    /// Cursor position when the drag started.
    pub grab_start: (f32, f32),
    /// Time when the drag started (for velocity calculation).
    pub grab_start_time: Instant,
}

impl FetchEngine {
    /// Create a new FetchEngine.
    ///
    /// - `dog_x`: current dog X position (ball spawns near the dog's feet)
    /// - `strip_width`: strip window width in logical pixels
    /// - `strip_height`: strip window height in logical pixels
    pub fn new(dog_x: f32, strip_width: f32, strip_height: f32) -> Self {
        let ball_x = dog_x + 30.0;
        let ball_y = strip_height - BALL_RADIUS;

        Self {
            ball: Ball {
                x: ball_x,
                y: ball_y,
                vx: 0.0,
                vy: 0.0,
                bounce_count: 0,
            },
            state: FetchState::WaitingForThrow,
            dog_x,
            dog_facing_left: false,
            strip_width,
            strip_height,
            ball_grabbed: false,
            grab_start: (ball_x, ball_y),
            grab_start_time: Instant::now(),
        }
    }

    // -----------------------------------------------------------------------
    // Physics
    // -----------------------------------------------------------------------

    /// Advance ball physics by `dt` seconds.
    ///
    /// Only called when the state is `BallInFlight` or `DogChasing`.
    fn tick_physics(&mut self, dt: f32) {
        // Apply gravity (downward = +y).
        self.ball.vy += 980.0 * dt;

        // Apply air resistance while the ball is airborne (above ground line).
        let on_ground = self.ball.y >= self.strip_height - BALL_RADIUS - 1.0;
        if !on_ground {
            let air_factor = 0.995_f32.powf(dt * 30.0);
            self.ball.vx *= air_factor;
        }

        // Integrate position.
        self.ball.x += self.ball.vx * dt;
        self.ball.y += self.ball.vy * dt;

        // --- Ground bounce (bottom edge) ---
        let ground_y = self.strip_height - BALL_RADIUS;
        if self.ball.y >= ground_y {
            self.ball.y = ground_y;
            // Suppress micro-bounce jitter when velocity is small.
            if self.ball.vy.abs() < 20.0 {
                self.ball.vy = 0.0;
            } else {
                self.ball.vy = -(self.ball.vy * 0.65);
                self.ball.bounce_count += 1;
            }
            // Rolling friction reduces horizontal speed on each ground contact.
            self.ball.vx *= 0.92;
        }

        // --- Top wall bounce ---
        if self.ball.y <= BALL_RADIUS {
            self.ball.y = BALL_RADIUS;
            self.ball.vy = self.ball.vy.abs();
        }

        // --- Left wall bounce ---
        if self.ball.x <= BALL_RADIUS {
            self.ball.x = BALL_RADIUS;
            self.ball.vx = self.ball.vx.abs();
        }

        // --- Right wall bounce ---
        if self.ball.x >= self.strip_width - BALL_RADIUS {
            self.ball.x = self.strip_width - BALL_RADIUS;
            self.ball.vx = -self.ball.vx.abs();
        }
    }

    // -----------------------------------------------------------------------
    // Chase AI (direct — no prediction error)
    // -----------------------------------------------------------------------

    /// Move the dog directly toward `ball.x` at `DOG_CHASE_SPEED`.
    ///
    /// Returns `true` when the dog has reached the ball's x position.
    fn tick_chase(&mut self, dt: f32, player: &mut AnimationPlayer) -> bool {
        let dx = self.ball.x - self.dog_x;
        let step = DOG_CHASE_SPEED * dt;

        if dx.abs() <= step {
            self.dog_x = self.ball.x;
            return true;
        }

        if dx > 0.0 {
            self.dog_x += step;
            self.dog_facing_left = false;
            player.facing_left = false;
        } else {
            self.dog_x -= step;
            self.dog_facing_left = true;
            player.facing_left = true;
        }
        false
    }

    /// Create a new ProudTrot state with random leg count (3-5) and duration (1-3s).
    fn new_proud_trot(&self) -> FetchState {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let legs = 3 + (nanos % 3) as u8; // 3, 4, or 5 legs
        let duration = 1.0 + (nanos % 2001) as f32 / 1000.0; // 1.0–3.0s
        let going_right = nanos % 2 == 0;
        FetchState::ProudTrot { leg_timer: 0.0, leg_duration: duration, legs_remaining: legs, going_right }
    }

    /// Random leg duration between 1.0 and 3.0 seconds.
    fn random_leg_duration() -> f32 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        1.0 + (nanos % 2001) as f32 / 1000.0
    }

    // -----------------------------------------------------------------------
    // Main tick
    // -----------------------------------------------------------------------

    /// Advance the ball FSM by `dt` seconds.
    ///
    /// Must be called every frame when the ball is active.
    /// Returns `FetchTickResult` so the caller can react to game events.
    pub fn tick(&mut self, dt: f32, player: &mut AnimationPlayer) -> FetchTickResult {
        match self.state.clone() {
            // ------------------------------------------------------------------
            // WaitingForThrow — ball rests, dog idles (BehaviorEngine drives dog)
            // ------------------------------------------------------------------
            FetchState::WaitingForThrow => {
                // Nothing to do — BehaviorEngine handles dog in this state.
            }

            // ------------------------------------------------------------------
            // BallGrabbed — user is dragging the ball
            // ------------------------------------------------------------------
            FetchState::BallGrabbed => {
                // Nothing to do — position set externally.
            }

            // ------------------------------------------------------------------
            // BallInFlight — simulate physics, immediately start chasing
            // ------------------------------------------------------------------
            FetchState::BallInFlight => {
                self.tick_physics(dt);
                // Dog starts chasing immediately — no reaction delay.
                self.state = FetchState::DogChasing;
            }

            // ------------------------------------------------------------------
            // DogChasing — physics continues; dog runs directly toward ball
            // ------------------------------------------------------------------
            FetchState::DogChasing => {
                self.tick_physics(dt);
                player.play("run");

                let at_target = self.tick_chase(dt, player);

                // Check distance from dog to ball.
                let dist = (self.dog_x - self.ball.x).abs();

                // Dog can only catch when ball is near ground level.
                let ball_near_ground =
                    self.ball.y >= self.strip_height - BALL_RADIUS - DOG_FRAME_HEIGHT;
                if dist <= CATCH_RADIUS && ball_near_ground {
                    self.state = FetchState::DogCatching { timer: 0.0 };
                } else if self.is_ball_stopped() {
                    if at_target || dist <= CATCH_RADIUS * 2.0 {
                        // Close enough — missed catch, go pick it up.
                        self.state = FetchState::MissReaction { timer: 0.0 };
                    }
                }
            }

            // ------------------------------------------------------------------
            // DogCatching — short catch celebration (0.5 s), then branch
            // ------------------------------------------------------------------
            FetchState::DogCatching { timer } => {
                player.play("excited");
                let new_timer = timer + dt;

                if new_timer >= 0.5 {
                    if self.ball.bounce_count == 0 {
                        // Perfect mid-air catch!
                        self.state = FetchState::Celebration { timer: 0.0 };
                        return FetchTickResult::CatchHappiness(10.0);
                    } else {
                        // Normal catch after bounces.
                        self.state = self.new_proud_trot();
                        return FetchTickResult::CatchHappiness(7.0);
                    }
                } else {
                    self.state = FetchState::DogCatching { timer: new_timer };
                }
            }

            // ------------------------------------------------------------------
            // Celebration — extended excited animation (1.5 s) for perfect catch
            // ------------------------------------------------------------------
            FetchState::Celebration { timer } => {
                player.play("excited");
                let new_timer = timer + dt;

                if new_timer >= 1.5 {
                    self.state = self.new_proud_trot();
                } else {
                    self.state = FetchState::Celebration { timer: new_timer };
                }
            }

            // ------------------------------------------------------------------
            // ProudTrot — dog runs in one direction, switches, repeats, drops ball
            // ------------------------------------------------------------------
            FetchState::ProudTrot { leg_timer, leg_duration, legs_remaining, going_right } => {
                player.play("walk");

                let new_timer = leg_timer + dt;
                let step = DOG_TROT_SPEED * dt;

                // Move in current direction, bounce off strip edges.
                let mut dir_right = going_right;
                if dir_right {
                    self.dog_x += step;
                    if self.dog_x >= self.strip_width - DOG_FRAME_HEIGHT {
                        self.dog_x = self.strip_width - DOG_FRAME_HEIGHT;
                        dir_right = false;
                    }
                } else {
                    self.dog_x -= step;
                    if self.dog_x <= 0.0 {
                        self.dog_x = 0.0;
                        dir_right = true;
                    }
                }
                self.dog_facing_left = !dir_right;
                player.facing_left = !dir_right;

                if new_timer >= leg_duration || dir_right != going_right {
                    if legs_remaining <= 1 {
                        // Done — drop ball here.
                        self.ball.x = self.dog_x + if dir_right { 30.0 } else { -30.0 };
                        self.ball.x = self.ball.x.clamp(BALL_RADIUS, self.strip_width - BALL_RADIUS);
                        self.ball.y = self.strip_height - BALL_RADIUS;
                        self.ball.vx = 0.0;
                        self.ball.vy = 0.0;
                        self.ball.bounce_count = 0;
                        self.state = FetchState::WaitingForThrow;
                    } else {
                        self.state = FetchState::ProudTrot {
                            leg_timer: 0.0,
                            leg_duration: Self::random_leg_duration(),
                            legs_remaining: legs_remaining - 1,
                            going_right: !dir_right,
                        };
                    }
                } else {
                    self.state = FetchState::ProudTrot {
                        leg_timer: new_timer,
                        leg_duration,
                        legs_remaining,
                        going_right: dir_right,
                    };
                }
            }

            // ------------------------------------------------------------------
            // MissReaction — dog looks sad for 0.8 s, then heads toward ball
            // ------------------------------------------------------------------
            FetchState::MissReaction { timer } => {
                player.play("sad");
                let new_timer = timer + dt;

                if new_timer >= 0.8 {
                    self.state = FetchState::MissPickup { timer: 0.0 };
                } else {
                    self.state = FetchState::MissReaction { timer: new_timer };
                }
            }

            // ------------------------------------------------------------------
            // MissPickup — slow sad walk toward the ball; pick up and trot
            // ------------------------------------------------------------------
            FetchState::MissPickup { timer: _ } => {
                player.play("walk");

                let dx = self.ball.x - self.dog_x;
                let step = DOG_PICKUP_SPEED * dt;

                if dx.abs() <= CATCH_RADIUS {
                    self.state = self.new_proud_trot();
                } else if dx > 0.0 {
                    self.dog_x += step;
                    self.dog_facing_left = false;
                    player.facing_left = false;
                } else {
                    self.dog_x -= step;
                    self.dog_facing_left = true;
                    player.facing_left = true;
                }
            }
        }

        FetchTickResult::Continue
    }

    // -----------------------------------------------------------------------
    // Interaction: grab / release
    // -----------------------------------------------------------------------

    /// Called when the user clicks (pointer down). Returns true if ball was grabbed.
    ///
    /// Grab succeeds when cursor is within BALL_RADIUS + 10 px of ball center.
    /// Ball is re-grabbable in any state except when the dog has it.
    pub fn on_ball_grab(&mut self, cursor_x: f32, cursor_y: f32) -> bool {
        match self.state {
            FetchState::DogCatching { .. }
            | FetchState::ProudTrot { .. }
            | FetchState::Celebration { .. }
            | FetchState::BallGrabbed => return false,
            _ => {}
        }

        let dx = cursor_x - self.ball.x;
        let dy = cursor_y - self.ball.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= BALL_RADIUS + 10.0 {
            self.state = FetchState::BallGrabbed;
            self.ball_grabbed = true;
            self.grab_start = (cursor_x, cursor_y);
            self.grab_start_time = Instant::now();
            true
        } else {
            false
        }
    }

    /// Called when the user releases the pointer (pointer up).
    ///
    /// Computes throw velocity from drag displacement / duration.
    pub fn on_ball_release(&mut self, cursor_x: f32, cursor_y: f32) {
        if self.state != FetchState::BallGrabbed {
            return;
        }

        let elapsed = self.grab_start_time.elapsed().as_secs_f32().max(0.001);
        let dx = cursor_x - self.grab_start.0;
        let dy = cursor_y - self.grab_start.1;

        let mut vx = dx / elapsed;
        let mut vy = dy / elapsed;

        // Clamp to MAX_THROW_SPEED.
        let speed = (vx * vx + vy * vy).sqrt();
        if speed > MAX_THROW_SPEED {
            let scale = MAX_THROW_SPEED / speed;
            vx *= scale;
            vy *= scale;
        }

        self.ball.vx = vx;
        self.ball.vy = vy;
        self.ball.bounce_count = 0;
        self.ball_grabbed = false;

        self.state = FetchState::BallInFlight;
    }

    /// Update ball position while the user is dragging.
    pub fn update_ball_drag(&mut self, cursor_x: f32, cursor_y: f32) {
        if self.state == FetchState::BallGrabbed {
            self.ball.x = cursor_x;
            self.ball.y = cursor_y;
        }
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Returns ball center position as (x, y) in logical pixels.
    pub fn ball_position(&self) -> (f32, f32) {
        (self.ball.x, self.ball.y)
    }

    /// Returns `true` if the ball has effectively stopped moving.
    pub fn is_ball_stopped(&self) -> bool {
        let on_ground = self.ball.y >= self.strip_height - BALL_RADIUS - 1.0;
        let slow = self.ball.vx.abs() < 5.0 && self.ball.vy.abs() < 5.0;
        on_ground && slow
    }

    /// Returns true if FetchEngine is actively controlling the dog
    /// (i.e., not in WaitingForThrow or BallGrabbed where BehaviorEngine should drive).
    pub fn is_dog_active(&self) -> bool {
        !matches!(self.state, FetchState::WaitingForThrow | FetchState::BallGrabbed)
    }
}
