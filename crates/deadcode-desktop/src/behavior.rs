//! Behavior engine for good-boi.
//!
//! Provides `BehaviorEngine` — a finite state machine that reads three internal
//! stats (`PetStats`), applies time-based decay, and drives `AnimationPlayer`
//! through weighted random state transitions.
//!
//! Design principles:
//! - No rand crate: random selection uses SystemTime subsec_nanos seeding.
//! - Stats floor at 10.0 regardless of decay (PET-06).
//! - PlayOnce animations (Beg, Scratch, Sad, Excited) are NOT exited until
//!   `player.current_animation()` returns "idle" — prevents animation desync.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::animation::AnimationPlayer;

// ---------------------------------------------------------------------------
// EdgeEvent
// ---------------------------------------------------------------------------

/// Emitted by `BehaviorEngine::tick()` when the dog walks off a strip edge.
///
/// The App uses this to either switch to an adjacent monitor or call
/// `bounce_at_edge()` if no adjacent monitor exists.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeEvent {
    /// No edge was hit this tick.
    None,
    /// Dog walked past x < 0 (walked off the left edge).
    WalkOffLeft,
    /// Dog walked past x >= strip_width - frame_width (walked off the right edge).
    WalkOffRight,
}

// ---------------------------------------------------------------------------
// PetStats
// ---------------------------------------------------------------------------

/// Internal state of the pet. All values range 0.0–100.0, floor 10.0.
#[derive(Debug, Clone)]
pub struct PetStats {
    /// How hungry the dog is (lower = more hungry, triggers beg).
    pub hunger: f32,
    /// How clean the dog is (lower = dirtier, triggers scratch).
    pub cleanliness: f32,
    /// How happy the dog is (lower = sadder, triggers sad).
    pub happiness: f32,
}

impl Default for PetStats {
    fn default() -> Self {
        Self {
            hunger: 100.0,
            cleanliness: 100.0,
            happiness: 100.0,
        }
    }
}

impl PetStats {
    /// Apply per-second decay for the given number of elapsed seconds.
    ///
    /// Decay rates:
    /// - hunger:      0.0070/sec (~3.6 hours full-to-critical, fastest)
    /// - cleanliness: 0.0055/sec (~4.5 hours, medium)
    /// - happiness:   0.0042/sec (~6 hours, slowest)
    ///
    /// All stats are clamped to a floor of 10.0 after decay (PET-06).
    pub fn apply_decay_secs(&mut self, secs: f32) {
        self.hunger = (self.hunger - 0.0070 * secs).max(10.0);
        self.cleanliness = (self.cleanliness - 0.0055 * secs).max(10.0);
        self.happiness = (self.happiness - 0.0042 * secs).max(10.0);
    }

    /// Restore hunger by 70 points (clamped to 100.0).
    pub fn restore_hunger(&mut self) {
        self.hunger = (self.hunger + 70.0).min(100.0);
    }

    /// Restore cleanliness by 70 points (clamped to 100.0).
    pub fn restore_cleanliness(&mut self) {
        self.cleanliness = (self.cleanliness + 70.0).min(100.0);
    }

    /// Add `amount` to happiness (clamped to 100.0).
    pub fn add_happiness(&mut self, amount: f32) {
        self.happiness = (self.happiness + amount).min(100.0);
    }
}

// ---------------------------------------------------------------------------
// BehaviorState
// ---------------------------------------------------------------------------

/// The FSM state of the dog's behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum BehaviorState {
    /// Dog is standing/sitting idly. Accumulates time for nap/walk transitions.
    Idle { timer: Duration },
    /// Dog is walking across the strip. `direction` is +1.0 (right) or -1.0 (left).
    Walk { direction: f32 },
    /// Dog is sitting. Holds for 3–8 seconds, then transitions.
    Sit { timer: Duration },
    /// Dog is napping (short sleep). Woken by interaction or after 2–3 min.
    Nap { timer: Duration },
    /// Dog is in deep night sleep (11 PM – 6 AM). Stays until dawn or interaction.
    NightSleep,
    /// PlayOnce — dog begs. Triggered by low hunger. Wait for "idle" return.
    Beg,
    /// PlayOnce — dog scratches. Triggered by low cleanliness. Wait for "idle" return.
    Scratch,
    /// PlayOnce — dog looks sad. Triggered by low happiness. Wait for "idle" return.
    Sad,
    /// Dog is excited. Holds for ~1 second, then transitions to `return_to` or chooses next state.
    Excited { timer: Duration, return_to: Option<Box<BehaviorState>> },
    /// Dog is eating. Holds for ~2 seconds, then transitions to Idle.
    Eating { timer: Duration },
    /// Dog is being cleaned. Holds for ~2 seconds, then transitions to Idle.
    Cleaning { timer: Duration },
    /// Dog is being petted externally. Exits when `stop_petting()` is called.
    Petting,
}

// ---------------------------------------------------------------------------
// BehaviorEngine
// ---------------------------------------------------------------------------

/// Core behavior FSM. Drives the AnimationPlayer through autonomous state
/// transitions based on internal stats and random weighted selection.
pub struct BehaviorEngine {
    /// Current FSM state.
    state: BehaviorState,
    /// Internal pet stats (hunger, cleanliness, happiness).
    stats: PetStats,
    /// Accumulated consecutive idle time for nap transition (~5 min).
    idle_accumulator: Duration,
    /// Sub-pixel dog position accumulator (f32 for precision at high tick rates).
    dog_x: f32,
    /// Whether the dog is currently moving left.
    dog_moving_left: bool,
    /// Current movement speed in pixels per second (60 for walk, 0 for stationary).
    dog_speed: f32,
    /// Time until the next walk consideration (randomized 30–60 seconds).
    walk_timer: Duration,
    /// Stored strip width for edge detection (set on first tick).
    strip_width: u32,
}

impl BehaviorEngine {
    /// Create a new BehaviorEngine starting with an excited greeting.
    pub fn new(initial_dog_x: f32) -> Self {
        Self {
            state: BehaviorState::Excited { timer: Duration::ZERO, return_to: None },
            stats: PetStats::default(),
            idle_accumulator: Duration::ZERO,
            dog_x: initial_dog_x,
            dog_moving_left: false,
            dog_speed: 0.0,
            walk_timer: randomized_walk_timer(),
            strip_width: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Core tick
    // -----------------------------------------------------------------------

    /// Advance the behavior engine by `delta`.
    ///
    /// Returns `(is_active, edge_event)`:
    /// - `is_active`: `true` if the dog is in an active (non-idle) state, so the caller can
    ///   maintain elevated FPS.
    /// - `edge_event`: `EdgeEvent::WalkOffLeft` or `EdgeEvent::WalkOffRight` when the dog walks
    ///   past a strip edge; `EdgeEvent::None` otherwise. The App must respond by either calling
    ///   `switch_monitor()` or `bounce_at_edge()`.
    ///
    /// Must be called once per frame from `App::about_to_wait()`.
    pub fn tick(
        &mut self,
        delta: Duration,
        player: &mut AnimationPlayer,
        strip_width: u32,
        frame_width: u32,
    ) -> (bool, EdgeEvent) {
        // Cache strip width for edge detection.
        self.strip_width = strip_width;

        // Apply stat decay every tick.
        self.stats.apply_decay_secs(delta.as_secs_f32());

        // Handle nighttime override: force NightSleep if in Idle/Sit/Nap and it's night.
        match &self.state {
            BehaviorState::Idle { .. }
            | BehaviorState::Sit { .. }
            | BehaviorState::Nap { .. } => {
                if is_nighttime() {
                    self.state = BehaviorState::NightSleep;
                    self.dog_speed = 0.0;
                    player.play("sleep");
                    return (false, EdgeEvent::None);
                }
            }
            _ => {}
        }

        let mut edge_event = EdgeEvent::None;
        let next_state: Option<BehaviorState> = match &mut self.state {
            // ---------------------------------------------------------------
            // Idle
            // ---------------------------------------------------------------
            BehaviorState::Idle { timer } => {
                *timer += delta;
                self.idle_accumulator += delta;

                // Nighttime override already handled above.

                // After ~5 min consecutive idle → nap.
                if self.idle_accumulator >= Duration::from_secs(300) {
                    self.idle_accumulator = Duration::ZERO;
                    Some(BehaviorState::Nap { timer: Duration::ZERO })
                } else if *timer >= self.walk_timer {
                    // Walk timer expired — choose what to do next.
                    self.walk_timer = randomized_walk_timer();
                    Some(self.choose_next_state())
                } else {
                    player.play("idle");
                    None
                }
            }

            // ---------------------------------------------------------------
            // Walk
            // ---------------------------------------------------------------
            BehaviorState::Walk { direction } => {
                let dir = *direction;
                self.dog_speed = 60.0;
                player.facing_left = dir < 0.0;
                player.play("walk");

                // Move dog.
                let delta_secs = delta.as_secs_f32();
                let max_x = strip_width as f32 - frame_width as f32;

                self.dog_x += self.dog_speed * delta_secs * dir;

                // Edge detection: emit walk-off events instead of bouncing.
                // The App will either switch to an adjacent monitor or call bounce_at_edge().
                if self.dog_x < 0.0 {
                    edge_event = EdgeEvent::WalkOffLeft;
                } else if self.dog_x >= max_x {
                    edge_event = EdgeEvent::WalkOffRight;
                } else {
                    self.dog_moving_left = dir < 0.0;
                }

                // Walk for 2–4 seconds then transition.
                let walk_duration = randomized_walk_duration();
                // Use idle_accumulator as walk timer (reset it on state entry).
                self.idle_accumulator += delta;
                if self.idle_accumulator >= walk_duration {
                    self.idle_accumulator = Duration::ZERO;
                    self.dog_speed = 0.0;
                    Some(self.choose_next_state())
                } else {
                    None
                }
            }

            // ---------------------------------------------------------------
            // Sit
            // ---------------------------------------------------------------
            BehaviorState::Sit { timer } => {
                *timer += delta;
                player.play("sit");

                let hold = randomized_sit_duration();
                if *timer >= hold {
                    Some(self.choose_next_state())
                } else {
                    None
                }
            }

            // ---------------------------------------------------------------
            // Nap
            // ---------------------------------------------------------------
            BehaviorState::Nap { timer } => {
                *timer += delta;
                player.play("sleep");

                // Wake after 2–3 min.
                let nap_duration = Duration::from_secs(150); // ~2.5 min
                if *timer >= nap_duration {
                    Some(BehaviorState::Idle { timer: Duration::ZERO })
                } else {
                    None
                }
            }

            // ---------------------------------------------------------------
            // NightSleep
            // ---------------------------------------------------------------
            BehaviorState::NightSleep => {
                player.play("sleep");

                if !is_nighttime() {
                    // Dawn — wake up with excitement.
                    Some(BehaviorState::Excited {
                        timer: Duration::ZERO,
                        return_to: Some(Box::new(BehaviorState::Idle { timer: Duration::ZERO })),
                    })
                } else {
                    None
                }
            }

            // ---------------------------------------------------------------
            // Beg — PlayOnce: wait until animation returns to "idle"
            // ---------------------------------------------------------------
            BehaviorState::Beg => {
                if player.current_animation() == "idle" {
                    // PlayOnce completed.
                    Some(self.choose_next_state())
                } else {
                    player.play("beg");
                    None
                }
            }

            // ---------------------------------------------------------------
            // Scratch — PlayOnce
            // ---------------------------------------------------------------
            BehaviorState::Scratch => {
                if player.current_animation() == "idle" {
                    Some(self.choose_next_state())
                } else {
                    player.play("scratch");
                    None
                }
            }

            // ---------------------------------------------------------------
            // Sad — PlayOnce
            // ---------------------------------------------------------------
            BehaviorState::Sad => {
                if player.current_animation() == "idle" {
                    Some(self.choose_next_state())
                } else {
                    player.play("sad");
                    None
                }
            }

            // ---------------------------------------------------------------
            // Excited — PlayOnce
            // ---------------------------------------------------------------
            BehaviorState::Excited { timer, return_to } => {
                *timer += delta;
                player.play("excited");
                if *timer >= Duration::from_millis(960) {
                    // ~2 loops of the 480ms excited animation, then transition.
                    let next = return_to
                        .take()
                        .map(|b| *b)
                        .unwrap_or_else(|| self.choose_next_state());
                    Some(next)
                } else {
                    None
                }
            }

            // ---------------------------------------------------------------
            // Eating
            // ---------------------------------------------------------------
            BehaviorState::Eating { timer } => {
                *timer += delta;
                player.play("sit"); // Dog sits while eating from the food bowl.
                if *timer >= Duration::from_secs(2) {
                    Some(BehaviorState::Idle { timer: Duration::ZERO })
                } else {
                    None
                }
            }

            // ---------------------------------------------------------------
            // Cleaning
            // ---------------------------------------------------------------
            BehaviorState::Cleaning { timer } => {
                *timer += delta;
                player.play("excited"); // Dog wiggles excitedly during wash.
                if *timer >= Duration::from_secs(2) {
                    Some(BehaviorState::Idle { timer: Duration::ZERO })
                } else {
                    None
                }
            }

            // ---------------------------------------------------------------
            // Petting — externally controlled, exits via stop_petting()
            // ---------------------------------------------------------------
            BehaviorState::Petting => {
                // Stay until stop_petting() is called.
                player.play("sit"); // Sit while being petted.
                None
            }
        };

        // Apply state transition if one was computed.
        if let Some(new_state) = next_state {
            self.enter_state(new_state, player);
        }

        (self.is_active(), edge_event)
    }

    /// Enter a new state, resetting relevant timers and playing the appropriate animation.
    fn enter_state(&mut self, new_state: BehaviorState, player: &mut AnimationPlayer) {
        match &new_state {
            BehaviorState::Idle { .. } => {
                self.dog_speed = 0.0;
                player.play("idle");
            }
            BehaviorState::Walk { direction } => {
                let dir = *direction;
                self.dog_speed = 60.0;
                self.idle_accumulator = Duration::ZERO; // Reset walk timer.
                player.facing_left = dir < 0.0;
                player.play("walk");
            }
            BehaviorState::Sit { .. } => {
                self.dog_speed = 0.0;
                player.play("sit");
            }
            BehaviorState::Nap { .. } => {
                self.dog_speed = 0.0;
                player.play("sleep");
            }
            BehaviorState::NightSleep => {
                self.dog_speed = 0.0;
                player.play("sleep");
            }
            BehaviorState::Beg => {
                self.dog_speed = 0.0;
                player.play("beg");
            }
            BehaviorState::Scratch => {
                self.dog_speed = 0.0;
                player.play("scratch");
            }
            BehaviorState::Sad => {
                self.dog_speed = 0.0;
                player.play("sad");
            }
            BehaviorState::Excited { .. } => {
                self.dog_speed = 0.0;
                player.play("excited");
            }
            BehaviorState::Eating { .. } => {
                self.dog_speed = 0.0;
                player.play("sit"); // Dog sits while eating from the food bowl.
            }
            BehaviorState::Cleaning { .. } => {
                self.dog_speed = 0.0;
                player.play("excited"); // Dog wiggles excitedly during wash.
            }
            BehaviorState::Petting => {
                self.dog_speed = 0.0;
                player.play("sit");
            }
        }
        self.state = new_state;
    }

    // -----------------------------------------------------------------------
    // Weighted random next state
    // -----------------------------------------------------------------------

    /// Choose the next state using weighted random selection.
    ///
    /// Distress weights (hunger/cleanliness/happiness < 30.0) elevate the
    /// corresponding animation. Walk and Sit/Idle fill the rest of the weight.
    fn choose_next_state(&self) -> BehaviorState {
        let beg_weight: u64 = if self.stats.hunger < 30.0 { 40 } else { 5 };
        let scratch_weight: u64 = if self.stats.cleanliness < 30.0 { 40 } else { 5 };
        let sad_weight: u64 = if self.stats.happiness < 30.0 { 40 } else { 5 };
        let walk_weight: u64 = 30;
        let sit_weight: u64 = 20;
        let idle_weight: u64 = 15;

        let total = beg_weight + scratch_weight + sad_weight + walk_weight + sit_weight + idle_weight;

        // Use SystemTime subsec_nanos as cheap random seed (no rand crate).
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .subsec_nanos() as u64;
        let pick = nanos % total;

        let mut acc = 0u64;

        acc += beg_weight;
        if pick < acc {
            return BehaviorState::Beg;
        }
        acc += scratch_weight;
        if pick < acc {
            return BehaviorState::Scratch;
        }
        acc += sad_weight;
        if pick < acc {
            return BehaviorState::Sad;
        }
        acc += walk_weight;
        if pick < acc {
            // Pick a direction based on current dog_moving_left for continuity.
            let direction = if self.dog_moving_left { -1.0 } else { 1.0 };
            return BehaviorState::Walk { direction };
        }
        acc += sit_weight;
        if pick < acc {
            return BehaviorState::Sit { timer: Duration::ZERO };
        }
        let _ = acc; // idle_weight covers the remainder.
        BehaviorState::Idle { timer: Duration::ZERO }
    }

    // -----------------------------------------------------------------------
    // Public interaction triggers
    // -----------------------------------------------------------------------

    /// Trigger a feeding interaction: restore hunger, enter Excited then Eating.
    pub fn trigger_feed(&mut self, player: &mut AnimationPlayer) {
        self.stats.restore_hunger();
        let eating = BehaviorState::Eating { timer: Duration::ZERO };
        let excited = BehaviorState::Excited {
            timer: Duration::ZERO,
            return_to: Some(Box::new(eating)),
        };
        self.enter_state(excited, player);
    }

    /// Trigger a cleaning interaction: restore cleanliness, enter Excited then Cleaning.
    pub fn trigger_clean(&mut self, player: &mut AnimationPlayer) {
        self.stats.restore_cleanliness();
        let cleaning = BehaviorState::Cleaning { timer: Duration::ZERO };
        let excited = BehaviorState::Excited {
            timer: Duration::ZERO,
            return_to: Some(Box::new(cleaning)),
        };
        self.enter_state(excited, player);
    }

    /// Begin a petting interaction.
    pub fn start_petting(&mut self, player: &mut AnimationPlayer) {
        self.enter_state(BehaviorState::Petting, player);
    }

    /// End petting, returning to Idle.
    pub fn stop_petting(&mut self, player: &mut AnimationPlayer) {
        self.enter_state(BehaviorState::Idle { timer: Duration::ZERO }, player);
    }

    /// Add happiness while petting (~5.0 points per second).
    pub fn tick_petting(&mut self, delta: Duration) {
        self.stats.add_happiness(5.0 * delta.as_secs_f32());
    }

    /// Wake up from Nap or NightSleep (called on any interaction).
    pub fn wake_up(&mut self, player: &mut AnimationPlayer) {
        match &self.state {
            BehaviorState::Nap { .. } | BehaviorState::NightSleep => {
                let excited = BehaviorState::Excited {
                    timer: Duration::ZERO,
                    return_to: Some(Box::new(BehaviorState::Idle { timer: Duration::ZERO })),
                };
                self.enter_state(excited, player);
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Returns dog X position as integer pixels.
    pub fn dog_x(&self) -> i32 {
        self.dog_x as i32
    }

    /// Set the dog X position directly (used when returning from fetch mode).
    pub fn set_dog_x(&mut self, x: i32) {
        self.dog_x = x as f32;
    }

    /// Reposition the dog at the entry point of a new monitor.
    ///
    /// Called by App after a successful `EdgeEvent::WalkOff*` monitor switch.
    /// The dog appears at the corresponding edge of the new monitor and continues
    /// walking in the same direction.
    pub fn set_monitor_entry(&mut self, new_x: f32, new_strip_width: u32) {
        self.dog_x = new_x;
        self.strip_width = new_strip_width;
    }

    /// Bounce the dog at the current edge (called when no adjacent monitor exists).
    ///
    /// Reverses the walk direction and clamps the dog position so it stays on screen.
    /// This restores the pre-multi-monitor edge-bounce behavior for single-monitor setups.
    pub fn bounce_at_edge(&mut self, player: &mut AnimationPlayer) {
        if self.dog_x < 0.0 {
            self.dog_x = 0.0;
            if let BehaviorState::Walk { direction } = &mut self.state {
                *direction = 1.0;
            }
            self.dog_moving_left = false;
            player.facing_left = false;
        } else {
            let max_x = self.strip_width as f32 - 48.0; // frame_width
            if self.dog_x >= max_x {
                self.dog_x = max_x;
                if let BehaviorState::Walk { direction } = &mut self.state {
                    *direction = -1.0;
                }
                self.dog_moving_left = true;
                player.facing_left = true;
            }
        }
    }

    /// Returns whether the dog is currently moving left.
    pub fn dog_moving_left(&self) -> bool {
        self.dog_moving_left
    }

    /// Immutable access to pet stats.
    pub fn stats(&self) -> &PetStats {
        &self.stats
    }

    /// Mutable access to pet stats (used by save/load in Plan 03-03).
    pub fn stats_mut(&mut self) -> &mut PetStats {
        &mut self.stats
    }

    /// Returns a reference to the current behavior state.
    ///
    /// Used by App to determine visual overlays (food bowl, soap bubbles, etc.).
    pub fn current_state(&self) -> &BehaviorState {
        &self.state
    }

    /// Returns `true` if the dog is in an active (non-resting) state.
    ///
    /// Used by App to maintain elevated FPS during action sequences.
    pub fn is_active(&self) -> bool {
        !matches!(
            self.state,
            BehaviorState::Idle { .. }
                | BehaviorState::Sit { .. }
                | BehaviorState::Nap { .. }
                | BehaviorState::NightSleep
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the current system hour is in the nighttime range (23:00–05:59).
fn is_nighttime() -> bool {
    // Use Instant-based approach isn't possible for wall clock. Use SystemTime.
    let secs_since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    // Seconds within the current day (UTC). Production apps should use local
    // time, but this avoids a chrono/time dependency (consistent with project pattern).
    let secs_of_day = secs_since_epoch % 86_400;
    let hour = (secs_of_day / 3600) as u32;

    hour >= 23 || hour < 6
}

/// Returns a randomized idle timer before the next action (3–10 seconds).
///
/// Reduced from the original 30–60 s so the dog feels alive and interactive
/// rather than standing still for half a minute between actions.
fn randomized_walk_timer() -> Duration {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .subsec_nanos() as u64;
    // Range: 3–10 seconds
    let ms = 3000 + (nanos % 7001);
    Duration::from_millis(ms)
}

/// Returns a randomized walk duration between 2 and 4 seconds.
fn randomized_walk_duration() -> Duration {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .subsec_nanos() as u64;
    // Range: 2000–4000ms
    let ms = 2000 + (nanos % 2001);
    Duration::from_millis(ms)
}

/// Returns a randomized sit duration between 3 and 8 seconds.
fn randomized_sit_duration() -> Duration {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .subsec_nanos() as u64;
    // Range: 3000–8000ms
    let ms = 3000 + (nanos % 5001);
    Duration::from_millis(ms)
}
