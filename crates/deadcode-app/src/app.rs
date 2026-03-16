use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use softbuffer::{Context, Surface};
use tray_icon::TrayIcon;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{Window, WindowId};

use deadcode_desktop::UserEvent;
use deadcode_desktop::animation::AnimationPlayer;
use deadcode_desktop::behavior::{BehaviorEngine, BehaviorState, EdgeEvent};
use deadcode_desktop::fetch::{FetchEngine, FetchTickResult};
use deadcode_desktop::fullscreen;
use deadcode_desktop::renderer::Renderer;
use deadcode_desktop::save;
use deadcode_desktop::save::{SaveData, Settings};
use deadcode_desktop::tray;
use deadcode_desktop::window::{StripInfo, enumerate_monitors};

use deadcode_editor::ipc::{JsToRust, RustToJs, WindowControlEvent};
use deadcode_editor::window::{WebViewManager, MaximizedState, open_editor, get_window_geometry};
use deadcode_editor::scripts::ScriptStore;
use deadcode_editor::tabs::EditorWindowState;
use deadcode_editor::execution::ScriptExecutionManager;
use deadcode_lang::DebugCommand;

// ---------------------------------------------------------------------------
// Particle system
// ---------------------------------------------------------------------------

const PARTICLE_LIFETIME: f32 = 1.0;
const PARTICLE_SPAWN_INTERVAL: f32 = 0.3;

struct Particle {
    x: f32,
    y: f32,
    vy: f32,
    alpha: f32,
    lifetime: f32,
    color: (u8, u8, u8),
}

// ---------------------------------------------------------------------------
// MonitorSlot
// ---------------------------------------------------------------------------

struct MonitorSlot {
    window: Arc<Window>,
    surface: Surface<Arc<Window>, Arc<Window>>,
    renderer: Renderer,
    info: StripInfo,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct App {
    // --- Desktop system ---
    monitor_slots: Vec<MonitorSlot>,
    active_monitor: usize,
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    renderer: Option<Renderer>,
    strip_info: Option<StripInfo>,
    first_frame: bool,
    tray_icon: Option<TrayIcon>,
    proxy: EventLoopProxy<UserEvent>,
    last_fullscreen_check: Instant,
    is_hidden_for_fullscreen: bool,
    hittest_disabled: bool,
    animation_player: Option<AnimationPlayer>,
    last_tick: Instant,
    active_until: Option<Instant>,
    behavior_engine: Option<BehaviorEngine>,
    context_menu: Option<tray_icon::menu::Menu>,
    cursor_position: winit::dpi::PhysicalPosition<f64>,
    is_petting: bool,
    pet_menu_timer: Option<Duration>,
    save_timer: Duration,
    particles: Vec<Particle>,
    particle_spawn_timer: Duration,
    fetch_engine: Option<FetchEngine>,

    // --- Editor system ---
    webview_manager: WebViewManager,
    ipc_sender: Sender<JsToRust>,
    ipc_receiver: Receiver<JsToRust>,
    script_store: Option<ScriptStore>,
    editor_state: EditorWindowState,
    execution_manager: ScriptExecutionManager,
    maximized_state: MaximizedState,
    settings: Settings,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let (ipc_sender, ipc_receiver) = unbounded::<JsToRust>();

        Self {
            // Desktop system
            monitor_slots: Vec::new(),
            active_monitor: 0,
            window: None,
            surface: None,
            renderer: None,
            strip_info: None,
            first_frame: true,
            tray_icon: None,
            proxy,
            last_fullscreen_check: Instant::now(),
            is_hidden_for_fullscreen: false,
            hittest_disabled: false,
            animation_player: None,
            last_tick: Instant::now(),
            active_until: None,
            behavior_engine: None,
            context_menu: None,
            cursor_position: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            is_petting: false,
            pet_menu_timer: None,
            save_timer: Duration::ZERO,
            particles: Vec::with_capacity(20),
            particle_spawn_timer: Duration::ZERO,
            fetch_engine: None,

            // Editor system
            webview_manager: WebViewManager::default(),
            ipc_sender,
            ipc_receiver,
            script_store: None,
            editor_state: EditorWindowState::default(),
            execution_manager: ScriptExecutionManager::default(),
            maximized_state: MaximizedState::default(),
            settings: Settings::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Active monitor helpers
// ---------------------------------------------------------------------------

impl App {
    fn active_renderer(&self) -> Option<&Renderer> {
        self.monitor_slots.get(self.active_monitor).map(|s| &s.renderer)
    }

    fn active_info(&self) -> Option<StripInfo> {
        self.monitor_slots.get(self.active_monitor).map(|s| s.info)
    }

    /// Build current settings with up-to-date editor window geometry.
    fn current_settings(&self) -> Settings {
        let editor_window = get_window_geometry(&self.webview_manager)
            .map(|(x, y, w, h)| save::WindowGeometry { x, y, width: w, height: h })
            .or_else(|| self.settings.editor_window.clone());
        Settings { editor_window }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Only create the windows once.
        if !self.monitor_slots.is_empty() {
            return;
        }

        // --- Desktop system init ---
        let monitor_infos = enumerate_monitors(event_loop);

        let slots: Vec<MonitorSlot> = monitor_infos
            .into_iter()
            .map(|info| {
                use winit::dpi::{LogicalPosition, LogicalSize};
                use winit::window::WindowLevel;

                let attrs = winit::window::Window::default_attributes()
                    .with_title("deadcode")
                    .with_transparent(true)
                    .with_decorations(false)
                    .with_window_level(WindowLevel::AlwaysOnTop)
                    .with_resizable(false)
                    .with_visible(false)
                    .with_inner_size(LogicalSize::new(
                        info.monitor_width as f64,
                        info.strip_height as f64,
                    ))
                    .with_position(LogicalPosition::new(
                        info.monitor_x as f64,
                        info.strip_y as f64,
                    ));

                let window = Arc::new(
                    event_loop
                        .create_window(attrs)
                        .expect("Failed to create strip window"),
                );

                let context = Context::new(window.clone())
                    .expect("Failed to create softbuffer context");
                let surface = Surface::new(&context, window.clone())
                    .expect("Failed to create softbuffer surface");

                let mut renderer = Renderer::new(info.monitor_width, info.strip_height);
                renderer.set_window(&window);

                MonitorSlot { window, surface, renderer, info }
            })
            .collect();

        // On Windows, invisible windows don't receive RedrawRequested events,
        // so we must make the active window visible before entering the event loop.
        // The window is transparent and frameless, so there's no white flash.
        // Also mark strip windows as tool windows so they don't minimize with the editor.
        #[cfg(target_os = "windows")]
        {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::Foundation::HWND;

            for slot in &slots {
                if let Ok(handle) = slot.window.window_handle() {
                    if let RawWindowHandle::Win32(h) = handle.as_raw() {
                        let hwnd = HWND(h.hwnd.get() as *mut _);
                        unsafe {
                            // Remove app-window style, add tool-window style so the strip
                            // won't appear in the taskbar and won't minimize with the editor.
                            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                            let new_style = (ex_style & !(WS_EX_APPWINDOW.0 as i32))
                                | WS_EX_TOOLWINDOW.0 as i32;
                            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
                            // Force Windows to apply the style change immediately.
                            let _ = SetWindowPos(
                                hwnd,
                                None,
                                0, 0, 0, 0,
                                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
                            );
                        }
                    }
                }
            }

            slots[0].window.set_visible(true);
            self.first_frame = false;

            // Re-apply tool-window style after set_visible since it resets the style.
            for slot in &slots {
                if let Ok(handle) = slot.window.window_handle() {
                    if let RawWindowHandle::Win32(h) = handle.as_raw() {
                        let hwnd = HWND(h.hwnd.get() as *mut _);
                        unsafe {
                            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                            let new_style = (ex_style & !(WS_EX_APPWINDOW.0 as i32))
                                | WS_EX_TOOLWINDOW.0 as i32;
                            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
                            let _ = SetWindowPos(
                                hwnd,
                                None,
                                0, 0, 0, 0,
                                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
                            );
                        }
                    }
                }
            }
        }

        let _ = slots[0].window.set_cursor_hittest(false);

        let mut animation_player = AnimationPlayer::new();
        let initial_dog_x = slots[0].renderer.dog_x() as f32;
        let mut behavior_engine = BehaviorEngine::new(initial_dog_x);

        if let Some(save_data) = save::load() {
            let elapsed_secs = save::elapsed_since(save_data.last_active_unix) as f32;
            behavior_engine.stats_mut().hunger =
                (save_data.hunger - 0.0070 * elapsed_secs).max(10.0);
            behavior_engine.stats_mut().cleanliness =
                (save_data.cleanliness - 0.0055 * elapsed_secs).max(10.0);
            behavior_engine.stats_mut().happiness =
                (save_data.happiness - 0.0042 * elapsed_secs).max(10.0);
            self.settings = save_data.settings;

        }

        behavior_engine.wake_up(&mut animation_player);

        let tray_icon = tray::create_tray(self.proxy.clone());
        let context_menu = tray::create_context_menu();

        self.window = Some(slots[0].window.clone());
        self.surface = None;
        self.renderer = None;
        self.strip_info = Some(slots[0].info);
        self.monitor_slots = slots;
        self.active_monitor = 0;
        self.first_frame = true;
        self.tray_icon = Some(tray_icon);
        self.context_menu = Some(context_menu);
        self.hittest_disabled = true;
        self.animation_player = Some(animation_player);
        self.last_tick = Instant::now();
        self.behavior_engine = Some(behavior_engine);

        // --- Editor system init ---
        let scripts_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("scripts");
        self.script_store = Some(ScriptStore::new(scripts_dir));

        // Open editor window immediately, restoring saved geometry if available
        open_editor(&mut self.webview_manager, &self.ipc_sender, self.settings.editor_window.as_ref().map(|g| (g.x, g.y, g.width, g.height)));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = position;
                let window = self.monitor_slots.get(self.active_monitor).map(|s| s.window.clone());
                if let (Some(fetch), Some(window)) = (&mut self.fetch_engine, window) {
                    if fetch.ball_grabbed {
                        let (lx, ly) = window_local_cursor(&window, self.cursor_position);
                        fetch.update_ball_drag(lx as f32, ly as f32);
                    }
                }
            }

            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Pressed,
                ..
            } => {
                let over_sprite = {
                    let window = self.monitor_slots.get(self.active_monitor).map(|s| s.window.clone());
                    let renderer = self.active_renderer();
                    let info = self.active_info();
                    match (window, renderer, info) {
                        (Some(w), Some(r), Some(i)) => cursor_over_sprite(&w, r, &i).unwrap_or(false),
                        _ => false,
                    }
                };

                if over_sprite {
                    if let (Some(engine), Some(player)) =
                        (&mut self.behavior_engine, &mut self.animation_player)
                    {
                        engine.wake_up(player);
                    }
                    self.show_context_menu();
                }
            }

            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                let ball_grabbed = if let Some(fetch) = &mut self.fetch_engine {
                    let window = self.monitor_slots.get(self.active_monitor).map(|s| s.window.clone());
                    if let Some(window) = window {
                        let (lx, ly) = window_local_cursor(&window, self.cursor_position);
                        fetch.on_ball_grab(lx as f32, ly as f32)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if ball_grabbed {
                    return;
                }

                let over_sprite = {
                    let window = self.monitor_slots.get(self.active_monitor).map(|s| s.window.clone());
                    let renderer = self.active_renderer();
                    let info = self.active_info();
                    match (window, renderer, info) {
                        (Some(w), Some(r), Some(i)) => cursor_over_sprite(&w, r, &i).unwrap_or(false),
                        _ => false,
                    }
                };

                if over_sprite {
                    self.is_petting = true;
                    self.pet_menu_timer = None;
                    if let (Some(engine), Some(player)) =
                        (&mut self.behavior_engine, &mut self.animation_player)
                    {
                        engine.start_petting(player);
                    }
                }
            }

            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Released,
                ..
            } => {
                if let Some(fetch) = &mut self.fetch_engine {
                    if fetch.ball_grabbed {
                        let window = self.monitor_slots.get(self.active_monitor).map(|s| s.window.clone());
                        if let Some(window) = window {
                            let (lx, ly) = window_local_cursor(&window, self.cursor_position);
                            fetch.on_ball_release(lx as f32, ly as f32);
                        }
                    }
                }

                if self.is_petting {
                    self.is_petting = false;
                    if let (Some(engine), Some(player)) =
                        (&mut self.behavior_engine, &mut self.animation_player)
                    {
                        engine.stop_petting(player);
                    }
                }
            }

            WindowEvent::Resized(new_size) => {
                let scale = self.monitor_slots
                    .get(self.active_monitor)
                    .map(|s| s.window.scale_factor())
                    .unwrap_or(1.0);
                let logical_w = (new_size.width as f64 / scale).round() as u32;
                let logical_h = (new_size.height as f64 / scale).round() as u32;

                if logical_w == 0 || logical_h == 0 {
                    return;
                }

                if let Some(slot) = self.monitor_slots.get_mut(self.active_monitor) {
                    slot.renderer.resize(logical_w, logical_h);
                }
            }

            WindowEvent::CloseRequested => {
                // Capture geometry before cleanup destroys the window
                if let Some((x, y, w, h)) = get_window_geometry(&self.webview_manager) {
                    self.settings.editor_window = Some(save::WindowGeometry { x, y, width: w, height: h });
                }
                if let Some(engine) = &self.behavior_engine {
                    save::save(&SaveData {
                        hunger: engine.stats().hunger,
                        cleanliness: engine.stats().cleanliness,
                        happiness: engine.stats().happiness,
                        last_active_unix: save::now_unix(),
                        settings: self.current_settings(),
                    });
                }
                self.webview_manager.cleanup();
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                let slot = match self.monitor_slots.get_mut(self.active_monitor) {
                    Some(s) => s,
                    None => return,
                };

                if let Some(player) = self.animation_player.as_ref() {
                    let info = slot.info;

                    slot.surface
                        .resize(
                            std::num::NonZeroU32::new(info.monitor_width).unwrap(),
                            std::num::NonZeroU32::new(info.strip_height).unwrap(),
                        )
                        .expect("Failed to resize surface");

                    let particle_data: Vec<(f32, f32, f32)> = self
                        .particles
                        .iter()
                        .filter(|p| p.color == (255, 100, 180))
                        .map(|p| (p.x, p.y, p.alpha))
                        .collect();

                    let colored_particle_data: Vec<(f32, f32, f32, u8, u8, u8)> = self
                        .particles
                        .iter()
                        .filter(|p| p.color != (255, 100, 180))
                        .map(|p| (p.x, p.y, p.alpha, p.color.0, p.color.1, p.color.2))
                        .collect();

                    let ball = self.fetch_engine.as_ref().map(|f| f.ball_position());

                    let overlay = if let Some(engine) = &self.behavior_engine {
                        match engine.current_state() {
                            BehaviorState::Eating { .. } => {
                                let dog_x = engine.dog_x();
                                let bowl_y = info.strip_height as i32 - 16;
                                Some((dog_x + 10, bowl_y, 20u32, 12u32, 139u8, 90u8, 43u8))
                            }
                            BehaviorState::Cleaning { .. } => {
                                let dog_x = engine.dog_x();
                                let bubble_y = info.strip_height as i32 - 40;
                                Some((dog_x - 5, bubble_y, 58u32, 35u32, 173u8, 216u8, 230u8))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };

                    slot.renderer.render(
                        &mut slot.surface,
                        info.monitor_width,
                        info.strip_height,
                        player,
                        &particle_data,
                        overlay,
                        ball,
                        &colored_particle_data,
                    );

                    if self.first_frame {
                        slot.window.set_visible(true);
                        self.first_frame = false;
                    }
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::MenuEvent(ref e) if e.id().0 == tray::quit_id() => {
                if let Some(engine) = &self.behavior_engine {
                    save::save(&SaveData {
                        hunger: engine.stats().hunger,
                        cleanliness: engine.stats().cleanliness,
                        happiness: engine.stats().happiness,
                        last_active_unix: save::now_unix(),
                        settings: self.current_settings(),
                    });
                }
                self.webview_manager.cleanup();
                event_loop.exit();
            }
            UserEvent::MenuEvent(ref e) if e.id().0 == tray::feed_id() => {
                if let (Some(engine), Some(player)) =
                    (&mut self.behavior_engine, &mut self.animation_player)
                {
                    engine.trigger_feed(player);
                }
            }
            UserEvent::MenuEvent(ref e) if e.id().0 == tray::clean_id() => {
                if let (Some(engine), Some(player)) =
                    (&mut self.behavior_engine, &mut self.animation_player)
                {
                    engine.trigger_clean(player);
                }
            }
            UserEvent::MenuEvent(ref e) if e.id().0 == "pet" => {
                if let (Some(engine), Some(player)) =
                    (&mut self.behavior_engine, &mut self.animation_player)
                {
                    engine.start_petting(player);
                    self.is_petting = true;
                    self.pet_menu_timer = Some(Duration::from_secs(3));
                }
            }
            UserEvent::MenuEvent(ref e) if e.id().0 == tray::editor_id() => {
                if self.webview_manager.is_visible() {
                    // Capture geometry before closing
                    if let Some((x, y, w, h)) = get_window_geometry(&self.webview_manager) {
                        self.settings.editor_window = Some(save::WindowGeometry { x, y, width: w, height: h });
                    }
                    self.webview_manager.close();
                } else {
                    // Not visible (either closed or hidden) — clean up stale state and reopen
                    if self.webview_manager.is_open() {
                        self.webview_manager.close();
                    }
                    open_editor(&mut self.webview_manager, &self.ipc_sender, self.settings.editor_window.as_ref().map(|g| (g.x, g.y, g.width, g.height)));
                }
            }
            UserEvent::MenuEvent(ref e) if e.id().0 == tray::play_id() => {
                if self.fetch_engine.is_some() {
                    self.fetch_engine = None;
                } else {
                    let dog_x = self.behavior_engine.as_ref()
                        .map(|e| e.dog_x() as f32)
                        .unwrap_or(100.0);
                    let info = self.active_info().unwrap_or(StripInfo {
                        monitor_x: 0, monitor_width: 800, monitor_height: 600,
                        monitor_index: 0, strip_y: 0, strip_height: 96,
                    });
                    self.fetch_engine = Some(FetchEngine::new(
                        dog_x,
                        info.monitor_width as f32,
                        info.strip_height as f32,
                    ));
                }
            }
            UserEvent::MenuEvent(_) => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_tick);
        self.last_tick = now;

        // --- Desktop system tick ---
        if let Some(player) = &mut self.animation_player {
            player.tick(delta);
        }

        // Ball engine tick
        let fetch_driving_dog = if let Some(fetch) = &self.fetch_engine {
            fetch.is_dog_active()
        } else {
            false
        };

        if fetch_driving_dog {
            let (catch_event, fetch_dog_x) =
                if let (Some(fetch), Some(player)) =
                    (&mut self.fetch_engine, &mut self.animation_player)
                {
                    let dt = delta.as_secs_f32();
                    let result = fetch.tick(dt, player);
                    let dog_x = fetch.dog_x;
                    let strip_h = fetch.strip_height;
                    let catch = match result {
                        FetchTickResult::CatchHappiness(amount) => Some((amount, strip_h)),
                        FetchTickResult::Continue => None,
                    };
                    (catch, dog_x)
                } else {
                    (None, 0.0)
                };

            if let Some((amount, strip_h)) = catch_event {
                if let Some(engine) = &mut self.behavior_engine {
                    engine.stats_mut().add_happiness(amount);
                }
                let dog_y = strip_h - 48.0;
                self.spawn_celebration_particles(fetch_dog_x, dog_y);
            }

            if let Some(slot) = self.monitor_slots.get_mut(self.active_monitor) {
                slot.renderer.set_dog_x(fetch_dog_x as i32);
            }
            if let Some(engine) = &mut self.behavior_engine {
                engine.set_dog_x(fetch_dog_x as i32);
            }

            self.active_until = Some(Instant::now() + Duration::from_secs(1));
        } else {
            if let (Some(fetch), Some(player)) =
                (&mut self.fetch_engine, &mut self.animation_player)
            {
                let dt = delta.as_secs_f32();
                fetch.tick(dt, player);
            }
            if let (Some(fetch), Some(engine)) = (&mut self.fetch_engine, &self.behavior_engine) {
                fetch.dog_x = engine.dog_x() as f32;
            }
        }

        // Behavior engine tick
        if !fetch_driving_dog {
            let strip_width = self.monitor_slots
                .get(self.active_monitor)
                .map(|s| s.info.monitor_width)
                .unwrap_or(800);

            let (is_active, edge_event) =
                if let (Some(player), Some(engine)) = (
                    self.animation_player.as_mut(),
                    self.behavior_engine.as_mut(),
                ) {
                    let frame_width = player.frame_width();
                    engine.tick(delta, player, strip_width, frame_width)
                } else {
                    (false, EdgeEvent::None)
                };

            let dog_x = self.behavior_engine.as_ref().map(|e| e.dog_x()).unwrap_or(0);
            if let Some(slot) = self.monitor_slots.get_mut(self.active_monitor) {
                slot.renderer.set_dog_x(dog_x);
            }

            match edge_event {
                EdgeEvent::WalkOffLeft => {
                    if self.active_monitor > 0 {
                        let new_idx = self.active_monitor - 1;
                        self.switch_monitor(new_idx, true);
                    } else if let (Some(engine), Some(player)) =
                        (&mut self.behavior_engine, &mut self.animation_player)
                    {
                        engine.bounce_at_edge(player);
                        let dog_x = engine.dog_x();
                        if let Some(slot) = self.monitor_slots.get_mut(self.active_monitor) {
                            slot.renderer.set_dog_x(dog_x);
                        }
                    }
                }
                EdgeEvent::WalkOffRight => {
                    let num_slots = self.monitor_slots.len();
                    if self.active_monitor + 1 < num_slots {
                        let new_idx = self.active_monitor + 1;
                        self.switch_monitor(new_idx, false);
                    } else if let (Some(engine), Some(player)) =
                        (&mut self.behavior_engine, &mut self.animation_player)
                    {
                        engine.bounce_at_edge(player);
                        let dog_x = engine.dog_x();
                        if let Some(slot) = self.monitor_slots.get_mut(self.active_monitor) {
                            slot.renderer.set_dog_x(dog_x);
                        }
                    }
                }
                EdgeEvent::None => {}
            }

            if is_active {
                self.active_until = Some(Instant::now() + Duration::from_secs(1));
            }
        }

        // Auto-save timer
        self.save_timer += delta;
        if self.save_timer >= Duration::from_secs(60) {
            self.save_timer = Duration::ZERO;
            if let Some(engine) = &self.behavior_engine {
                save::save(&SaveData {
                    hunger: engine.stats().hunger,
                    cleanliness: engine.stats().cleanliness,
                    happiness: engine.stats().happiness,
                    last_active_unix: save::now_unix(),
                    settings: self.current_settings(),
                });
            }
        }

        // Pet menu timer
        if let Some(remaining) = &mut self.pet_menu_timer {
            if *remaining <= delta {
                self.pet_menu_timer = None;
                self.is_petting = false;
                if let (Some(engine), Some(player)) =
                    (&mut self.behavior_engine, &mut self.animation_player)
                {
                    engine.stop_petting(player);
                }
            } else {
                *remaining -= delta;
            }
        }

        // Petting tick
        if self.is_petting {
            if let Some(engine) = &mut self.behavior_engine {
                engine.tick_petting(delta);
            }

            self.particle_spawn_timer += delta;
            if self.particle_spawn_timer >= Duration::from_secs_f32(PARTICLE_SPAWN_INTERVAL) {
                self.particle_spawn_timer = Duration::ZERO;

                let dog_x = self.behavior_engine.as_ref().map(|e| e.dog_x()).unwrap_or(0) as f32;
                let frame_w = self
                    .animation_player
                    .as_ref()
                    .map(|p| p.frame_width())
                    .unwrap_or(48) as f32;

                let base_nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();

                for i in 0..2u32 {
                    let seed = base_nanos.wrapping_add(i.wrapping_mul(997));
                    let offset_x = (seed % 30) as f32 - 15.0 + frame_w / 2.0;
                    let vy = 30.0 + (seed % 20) as f32;
                    self.particles.push(Particle {
                        x: dog_x + offset_x,
                        y: 40.0,
                        vy,
                        alpha: 1.0,
                        lifetime: PARTICLE_LIFETIME,
                        color: (255, 100, 180),
                    });
                }
            }
        }

        // Tick and cull particles
        let delta_secs = delta.as_secs_f32();
        self.particles.retain_mut(|p| {
            p.y -= p.vy * delta_secs;
            p.lifetime -= delta_secs;
            p.alpha = (p.lifetime / PARTICLE_LIFETIME).max(0.0);
            p.lifetime > 0.0
        });

        // Fullscreen polling
        if self.last_fullscreen_check.elapsed() >= Duration::from_millis(500) {
            let fs = fullscreen::is_any_fullscreen();
            if fs && !self.is_hidden_for_fullscreen {
                if let Some(slot) = self.monitor_slots.get(self.active_monitor) {
                    slot.window.set_visible(false);
                }
                self.is_hidden_for_fullscreen = true;
            } else if !fs && self.is_hidden_for_fullscreen {
                if let Some(slot) = self.monitor_slots.get(self.active_monitor) {
                    slot.window.set_visible(true);
                }
                self.is_hidden_for_fullscreen = false;
            }
            self.last_fullscreen_check = Instant::now();
        }

        // Per-pixel hit testing
        if let Some(slot) = self.monitor_slots.get(self.active_monitor) {
            let should_hittest = cursor_over_sprite(&slot.window, &slot.renderer, &slot.info)
                .unwrap_or(false);
            let w = slot.window.clone();
            if should_hittest && self.hittest_disabled {
                let _ = w.set_cursor_hittest(true);
                self.hittest_disabled = false;
            } else if !should_hittest && !self.hittest_disabled {
                let _ = w.set_cursor_hittest(false);
                self.hittest_disabled = true;
            }
        }

        // --- Editor IPC polling ---
        self.poll_editor_ipc();

        // --- Script execution polling ---
        self.execution_manager.poll_script_events(&self.webview_manager);

        // --- Detect editor native close ---
        if let Some((x, y, w, h)) = self.webview_manager.detect_native_close() {
            self.settings.editor_window = Some(save::WindowGeometry { x, y, width: w, height: h });

            if let Some(engine) = &self.behavior_engine {
                save::save(&SaveData {
                    hunger: engine.stats().hunger,
                    cleanliness: engine.stats().cleanliness,
                    happiness: engine.stats().happiness,
                    last_active_unix: save::now_unix(),
                    settings: self.current_settings(),
                });
            }
        }

        // --- Dynamic FPS ---
        let interval = if let Some(player) = &self.animation_player {
            let desired = player.desired_frame_interval();
            if desired > Duration::from_millis(33) {
                if self.active_until.map(|t| Instant::now() < t).unwrap_or(false) {
                    Duration::from_millis(33)
                } else {
                    desired
                }
            } else {
                self.active_until = Some(Instant::now() + Duration::from_secs(1));
                desired
            }
        } else {
            Duration::from_millis(100)
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + interval));

        if !self.is_hidden_for_fullscreen {
            if let Some(slot) = self.monitor_slots.get(self.active_monitor) {
                slot.window.request_redraw();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Editor IPC dispatch
// ---------------------------------------------------------------------------

impl App {
    fn poll_editor_ipc(&mut self) {
        while let Ok(msg) = self.ipc_receiver.try_recv() {
            match msg {
                JsToRust::EditorReady => {
                    self.webview_manager.show();
                    if let Some(store) = &self.script_store {
                        let infos = store.get_script_infos();
                        let msg = RustToJs::ScriptList { scripts: infos };
                        self.webview_manager.send_to_all(&msg);
                    }
                }
                JsToRust::ScriptSave { script_id, content } => {
                    if let Some(store) = &mut self.script_store {
                        store.save_script(&script_id, content);
                    }
                    self.editor_state.set_modified(&script_id, false);
                }
                JsToRust::ScriptRequest { script_id } => {
                    if let Some(store) = &self.script_store {
                        if let Some(script) = store.scripts.get(&script_id) {
                            self.editor_state.open_tab(script.id.clone(), script.name.clone());
                            let msg = RustToJs::ScriptLoad {
                                script_id: script.id.clone(),
                                name: script.name.clone(),
                                content: script.content.clone(),
                                script_type: script.script_type.as_str().to_string(),
                            };
                            self.webview_manager.send_to_all(&msg);
                        }
                    }
                }
                JsToRust::ScriptListRequest => {
                    if let Some(store) = &self.script_store {
                        let infos = store.get_script_infos();
                        let msg = RustToJs::ScriptList { scripts: infos };
                        self.webview_manager.send_to_all(&msg);
                    }
                }
                JsToRust::TabChanged { .. } => {
                    // Tab tracking handled by JS side
                }
                JsToRust::RunScript { script_id } => {
                    if let Some(store) = &self.script_store {
                        self.execution_manager.handle_run_script(&script_id, store, &self.webview_manager);
                    }
                }
                JsToRust::StopScript { .. } => {
                    self.execution_manager.handle_stop_script();
                }
                JsToRust::DebugStart { script_id } => {
                    if let Some(store) = &self.script_store {
                        self.execution_manager.handle_debug_start(&script_id, store, &self.webview_manager);
                    }
                }
                JsToRust::DebugContinue { .. } => {
                    self.execution_manager.handle_debug_command(DebugCommand::Continue, &self.webview_manager);
                }
                JsToRust::DebugStepOver { .. } => {
                    self.execution_manager.handle_debug_command(DebugCommand::StepOver, &self.webview_manager);
                }
                JsToRust::DebugStepInto { .. } => {
                    self.execution_manager.handle_debug_command(DebugCommand::StepInto, &self.webview_manager);
                }
                JsToRust::DebugStepOut { .. } => {
                    self.execution_manager.handle_debug_command(DebugCommand::StepOut, &self.webview_manager);
                }
                JsToRust::ToggleBreakpoint { script_id, line } => {
                    self.execution_manager.handle_toggle_breakpoint(&script_id, line);
                }
                JsToRust::WindowMinimize => {
                    self.webview_manager.handle_window_control(
                        WindowControlEvent::Minimize,
                        &mut self.maximized_state.maximized,
                    );
                }
                JsToRust::WindowMaximize => {
                    self.webview_manager.handle_window_control(
                        WindowControlEvent::Maximize,
                        &mut self.maximized_state.maximized,
                    );
                }
                JsToRust::WindowClose => {
                    // Capture geometry before the window is destroyed
                    if let Some((x, y, w, h)) = get_window_geometry(&self.webview_manager) {
                        self.settings.editor_window = Some(save::WindowGeometry { x, y, width: w, height: h });
                    }
                    self.webview_manager.handle_window_control(
                        WindowControlEvent::Close,
                        &mut self.maximized_state.maximized,
                    );
                }
                JsToRust::WindowDragStart => {
                    // Handled directly in the IPC handler for native drag
                }
                JsToRust::WindowResizeStart { .. } => {
                    // Handled directly in the IPC handler for native resize
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Monitor switching and helpers
// ---------------------------------------------------------------------------

impl App {
    fn switch_monitor(&mut self, new_index: usize, entering_from_right: bool) {
        if let Some(old_slot) = self.monitor_slots.get(self.active_monitor) {
            old_slot.window.set_visible(false);
        }

        if let Some(new_slot) = self.monitor_slots.get(new_index) {
            new_slot.window.set_visible(true);
            let _ = new_slot.window.set_cursor_hittest(false);
        }

        let new_width = self.monitor_slots.get(new_index).map(|s| s.info.monitor_width).unwrap_or(800);
        let new_x = if entering_from_right {
            new_width as f32 - 48.0
        } else {
            0.0
        };

        if let Some(engine) = &mut self.behavior_engine {
            engine.set_monitor_entry(new_x, new_width);
        }

        self.active_monitor = new_index;

        if let Some(slot) = self.monitor_slots.get(new_index) {
            self.window = Some(slot.window.clone());
            self.strip_info = Some(slot.info);
        }

        let dog_x = self.behavior_engine.as_ref().map(|e| e.dog_x()).unwrap_or(0);
        if let Some(slot) = self.monitor_slots.get_mut(new_index) {
            slot.renderer.set_dog_x(dog_x);
        }

        self.hittest_disabled = true;
    }

    fn spawn_celebration_particles(&mut self, dog_x: f32, dog_y: f32) {
        let base_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();

        for i in 0..8u32 {
            let seed = base_nanos.wrapping_add(i.wrapping_mul(1097));
            let offset_x = (seed % 40) as f32 - 20.0;
            let offset_y = (seed % 20) as f32;
            let vy = 40.0 + (seed % 30) as f32;
            self.particles.push(Particle {
                x: dog_x + offset_x,
                y: dog_y - offset_y,
                vy,
                alpha: 1.0,
                lifetime: PARTICLE_LIFETIME,
                color: (255, 215, 0),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Context menu display
// ---------------------------------------------------------------------------

impl App {
    #[cfg(target_os = "macos")]
    fn show_context_menu(&self) {
        use tray_icon::menu::ContextMenu;
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        if let (Some(w), Some(menu)) = (&self.window, &self.context_menu) {
            if let Ok(handle) = w.window_handle() {
                if let RawWindowHandle::AppKit(h) = handle.as_raw() {
                    unsafe {
                        menu.show_context_menu_for_nsview(h.ns_view.as_ptr() as _, None);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn show_context_menu(&self) {
        use tray_icon::menu::ContextMenu;
        use tray_icon::dpi::{PhysicalPosition, Position};
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        if let (Some(w), Some(menu)) = (&self.window, &self.context_menu) {
            if let Ok(handle) = w.window_handle() {
                if let RawWindowHandle::Win32(h) = handle.as_raw() {
                    let pos = Position::Physical(PhysicalPosition::new(
                        self.cursor_position.x as i32,
                        self.cursor_position.y as i32,
                    ));
                    unsafe {
                        menu.show_context_menu_for_hwnd(h.hwnd.get() as _, Some(pos));
                    }
                }
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn show_context_menu(&self) {}
}

// ---------------------------------------------------------------------------
// Cursor / hit test helpers
// ---------------------------------------------------------------------------

fn cursor_over_sprite(
    window: &Window,
    renderer: &Renderer,
    info: &StripInfo,
) -> Option<bool> {
    let (cx, cy) = get_cursor_position()?;

    let win_pos = window.outer_position().ok()?;
    let scale = window.scale_factor();

    let win_x = win_pos.x as f64 / scale;
    let win_y = win_pos.y as f64 / scale;

    let local_x = cx - win_x;
    let local_y = cy - win_y;

    if local_x < 0.0 || local_y < 0.0
        || local_x >= info.monitor_width as f64
        || local_y >= info.strip_height as f64
    {
        return Some(false);
    }

    Some(renderer.hit_test_at(local_x, local_y))
}

fn window_local_cursor(
    window: &Window,
    physical_pos: winit::dpi::PhysicalPosition<f64>,
) -> (f64, f64) {
    let scale = window.scale_factor();
    (physical_pos.x / scale, physical_pos.y / scale)
}

#[cfg(target_os = "macos")]
fn get_cursor_position() -> Option<(f64, f64)> {
    use objc2::{class, msg_send};
    use objc2::runtime::AnyObject;
    use objc2_foundation::NSPoint;

    unsafe {
        let ns_event_class = class!(NSEvent);
        let pos: NSPoint = msg_send![ns_event_class, mouseLocation];

        let ns_screen_class = class!(NSScreen);
        let main_screen: *mut AnyObject = msg_send![ns_screen_class, mainScreen];
        if main_screen.is_null() {
            return None;
        }
        let frame: objc2_foundation::NSRect = msg_send![main_screen, frame];
        let screen_height = frame.size.height;

        Some((pos.x, screen_height - pos.y))
    }
}

#[cfg(target_os = "windows")]
fn get_cursor_position() -> Option<(f64, f64)> {
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    use windows::Win32::Foundation::POINT;

    unsafe {
        let mut point = POINT::default();
        GetCursorPos(&mut point).ok()?;
        Some((point.x as f64, point.y as f64))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn get_cursor_position() -> Option<(f64, f64)> {
    None
}
