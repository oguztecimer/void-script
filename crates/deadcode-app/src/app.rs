use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use softbuffer::{Context, Surface};
use tray_icon::TrayIcon;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::window::{Window, WindowId};

use deadcode_desktop::UserEvent;
use deadcode_desktop::fullscreen;
use deadcode_desktop::renderer::Renderer;
use deadcode_desktop::save;
use deadcode_desktop::save::Settings;
use deadcode_desktop::tray;
use deadcode_desktop::unit::{UnitManager, WORLD_WIDTH};
use deadcode_desktop::window::{StripInfo, enumerate_monitors};

use deadcode_editor::ipc::{CommandInfo, JsToRust, RustToJs, WindowControlEvent};
use deadcode_sim::SimWorld;
use deadcode_sim::action::CommandDef;
use deadcode_editor::window::{WebViewManager, MaximizedState, open_editor, get_window_geometry};
use deadcode_editor::scripts::ScriptStore;
use deadcode_editor::tabs::EditorWindowState;
use deadcode_editor::execution::ScriptExecutionManager;
use grimscript_lang::DebugCommand;

use crate::modding::{self, SpriteData};

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
    last_tick: Instant,
    active_until: Option<Instant>,
    context_menu: Option<tray_icon::menu::Menu>,
    cursor_position: winit::dpi::PhysicalPosition<f64>,
    save_timer: Duration,
    unit_manager: Option<UnitManager>,

    // --- Modding system ---
    /// Entity type → sprite data (PNG bytes + JSON metadata).
    sprite_registry: HashMap<String, SpriteData>,
    /// Entity type → pivot [x, y].
    pivot_registry: HashMap<String, [f32; 2]>,
    /// Entity type → stat overrides.
    entity_configs: HashMap<String, deadcode_sim::entity::EntityConfig>,

    // --- Simulation system ---
    sim_world: Option<SimWorld>,

    // --- Editor system ---
    webview_manager: WebViewManager,
    ipc_sender: Sender<JsToRust>,
    ipc_receiver: Receiver<JsToRust>,
    script_store: Option<ScriptStore>,
    editor_state: EditorWindowState,
    execution_manager: ScriptExecutionManager,
    maximized_state: MaximizedState,
    settings: Settings,
    available_commands: HashSet<String>,
    /// Custom command definitions from mods.
    command_defs: HashMap<String, CommandDef>,

    /// Whether the background tick thread has been spawned (Windows only).
    #[cfg(target_os = "windows")]
    tick_thread_started: bool,
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
            last_tick: Instant::now(),
            active_until: None,
            context_menu: None,
            cursor_position: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            save_timer: Duration::ZERO,
            unit_manager: None,

            // Modding system
            sprite_registry: HashMap::new(),
            pivot_registry: HashMap::new(),
            entity_configs: HashMap::new(),

            // Simulation system
            sim_world: None,

            // Editor system
            webview_manager: WebViewManager::default(),
            ipc_sender,
            ipc_receiver,
            script_store: None,
            editor_state: EditorWindowState::default(),
            execution_manager: ScriptExecutionManager::default(),
            maximized_state: MaximizedState::default(),
            settings: Settings::default(),
            available_commands: HashSet::new(),
            command_defs: HashMap::new(),

            #[cfg(target_os = "windows")]
            tick_thread_started: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Active monitor helpers
// ---------------------------------------------------------------------------

impl App {
    /// Build current settings with up-to-date editor window geometry.
    fn current_settings(&self) -> Settings {
        let editor_window = get_window_geometry(&self.webview_manager)
            .map(|(x, y, w, h)| save::WindowGeometry { x, y, width: w, height: h })
            .or_else(|| self.settings.editor_window.clone());
        Settings { editor_window }
    }

    /// Run one game tick: advance units, poll IPC, check fullscreen, etc.
    fn do_tick(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_tick);
        self.last_tick = now;

        // --- Unit system tick ---
        if let Some(um) = &mut self.unit_manager {
            um.tick(delta);
        }

        // --- Simulation tick ---
        if let Some(sim) = &mut self.sim_world {
            if sim.is_running() {
                sim.tick();

                // Sync sim entity positions to UnitManager for rendering.
                // Maps i64 sim position → f32 render position.
                let snapshot = sim.snapshot();
                if let Some(um) = &mut self.unit_manager {
                    for es in &snapshot.entities {
                        let render_x = es.position as f32;
                        // Collect matching unit IDs first to avoid borrow conflict.
                        let matching_id = um.iter()
                            .find(|unit| unit.name == es.name)
                            .map(|unit| unit.id);
                        if let Some(uid) = matching_id {
                            um.move_to(uid, render_x, 100.0);
                        }
                    }
                }

                // Forward script output events to editor console and
                // spawn render units for newly-spawned sim entities.
                let events = sim.take_events();
                for event in &events {
                    match event {
                        deadcode_sim::SimEvent::EntitySpawned { entity_type, position, .. } => {
                            // Look up sprite in registry and spawn a render unit.
                            if let Some(sprite) = self.sprite_registry.get(entity_type) {
                                if let Some(um) = &mut self.unit_manager {
                                    let [px, py] = self.pivot_registry
                                        .get(entity_type)
                                        .copied()
                                        .unwrap_or([24.0, 0.0]);
                                    // Use entity_type as the render unit name for new spawns.
                                    let name = format!("{}_{}", entity_type, position);
                                    um.spawn(
                                        &name,
                                        &sprite.png,
                                        &sprite.json,
                                        *position as f32,
                                        px,
                                        py,
                                    );
                                }
                            }
                        }
                        deadcode_sim::SimEvent::ScriptOutput { text, .. } => {
                            let msg = RustToJs::ConsoleOutput {
                                text: text.clone(),
                                level: "info".to_string(),
                            };
                            self.webview_manager.send_to_all(&msg);
                        }
                        deadcode_sim::SimEvent::ScriptError { error, .. } => {
                            let msg = RustToJs::ConsoleOutput {
                                text: format!("[sim error] {error}"),
                                level: "error".to_string(),
                            };
                            self.webview_manager.send_to_all(&msg);
                        }
                        deadcode_sim::SimEvent::ScriptFinished { success, error, .. } => {
                            // Find the script_id associated with this entity.
                            // For now, use a placeholder — the editor tracks running state.
                            let msg = RustToJs::ScriptFinished {
                                script_id: String::new(), // The editor uses isRunning state.
                                success: *success,
                                error: error.clone(),
                            };
                            self.webview_manager.send_to_all(&msg);
                        }
                        _ => {}
                    }
                }

                // Send tick number to editor.
                let msg = RustToJs::SimulationTick { tick: snapshot.tick };
                self.webview_manager.send_to_all(&msg);

                self.active_until = Some(Instant::now() + Duration::from_secs(1));
            }
        }

        // Auto-save timer
        self.save_timer += delta;
        if self.save_timer >= Duration::from_secs(60) {
            self.save_timer = Duration::ZERO;
            save::save(&save::SaveData {
                last_active_unix: save::now_unix(),
                settings: self.current_settings(),
            });
        }

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

        // --- Window shake animation ---
        self.webview_manager.tick_shake();

        // --- Script execution polling ---
        self.execution_manager.poll_script_events(&self.webview_manager);
        self.execution_manager.poll_terminal_events(&self.webview_manager);

        // --- Detect editor native close ---
        if let Some((x, y, w, h)) = self.webview_manager.detect_native_close() {
            self.settings.editor_window = Some(save::WindowGeometry { x, y, width: w, height: h });
            save::save(&save::SaveData {
                last_active_unix: save::now_unix(),
                settings: self.current_settings(),
            });
        }
    }

    /// Render the current frame (request redraw on the active monitor).
    fn do_redraw(&mut self) {
        if !self.is_hidden_for_fullscreen {
            if let Some(slot) = self.monitor_slots.get(self.active_monitor) {
                slot.window.request_redraw();
            }
        }
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
                use winit::window::WindowLevel;

                // macOS: use logical coords (Cocoa handles DPI natively).
                // Windows: use physical coords to avoid DPI mismatch on multi-monitor.
                #[cfg(target_os = "macos")]
                let attrs = {
                    use winit::dpi::{LogicalPosition, LogicalSize};
                    winit::window::Window::default_attributes()
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
                        ))
                };
                #[cfg(not(target_os = "macos"))]
                let attrs = {
                    use winit::dpi::{LogicalSize, PhysicalPosition};
                    winit::window::Window::default_attributes()
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
                        .with_position(PhysicalPosition::new(
                            info.phys_x,
                            info.phys_y,
                        ))
                };

                let window = Arc::new(
                    event_loop
                        .create_window(attrs)
                        .expect("Failed to create strip window"),
                );

                // Disable macOS window shadow so sprites don't get a dark outline.
                #[cfg(target_os = "macos")]
                {
                    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
                    if let Ok(handle) = window.window_handle() {
                        if let RawWindowHandle::AppKit(h) = handle.as_raw() {
                            use objc2::msg_send;
                            let ns_window: *mut objc2::runtime::AnyObject = unsafe {
                                msg_send![h.ns_view.cast::<objc2::runtime::AnyObject>().as_ref(), window]
                            };
                            if !ns_window.is_null() {
                                let _: () = unsafe { msg_send![ns_window, setHasShadow: false] };
                            }
                        }
                    }
                }

                let context = Context::new(window.clone())
                    .expect("Failed to create softbuffer context");
                let surface = Surface::new(&context, window.clone())
                    .expect("Failed to create softbuffer surface");

                // Canvas must match physical window size (softbuffer blits 1:1 on Windows).
                // But pixel_scale stays logical-based so sprite size matches macOS.
                #[cfg(target_os = "macos")]
                let (canvas_w, canvas_h) = (info.monitor_width, info.strip_height);
                #[cfg(not(target_os = "macos"))]
                let (canvas_w, canvas_h) = (info.phys_width, info.phys_height);

                let mut renderer = Renderer::new(canvas_w, canvas_h);
                renderer.pixel_scale = (info.monitor_width / WORLD_WIDTH).max(1);
                renderer.set_window(&window);

                MonitorSlot { window, surface, renderer, info }
            })
            .collect();

        // On Windows, invisible windows don't receive RedrawRequested events,
        // so we must make the active window visible before entering the event loop.
        // Position the strip behind the taskbar but in front of all other windows.
        #[cfg(target_os = "windows")]
        {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::Foundation::HWND;
            use windows::core::w;

            // Find the taskbar so we can place our strip just behind it.
            let taskbar_hwnd = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok();

            for slot in &slots {
                if let Ok(handle) = slot.window.window_handle() {
                    if let RawWindowHandle::Win32(h) = handle.as_raw() {
                        let hwnd = HWND(h.hwnd.get() as *mut _);
                        unsafe {
                            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                            let new_style = (ex_style & !(WS_EX_APPWINDOW.0 as i32))
                                | WS_EX_TOOLWINDOW.0 as i32
                                | WS_EX_TOPMOST.0 as i32;
                            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);

                            // Place just behind the taskbar in z-order.
                            if let Some(tb) = taskbar_hwnd {
                                let _ = SetWindowPos(
                                    hwnd, tb,
                                    0, 0, 0, 0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED,
                                );
                            } else {
                                let _ = SetWindowPos(
                                    hwnd, HWND_TOPMOST,
                                    0, 0, 0, 0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED,
                                );
                            }
                        }
                    }
                }
            }

            slots[0].window.set_visible(true);
            self.first_frame = false;

            for slot in &slots {
                if let Ok(handle) = slot.window.window_handle() {
                    if let RawWindowHandle::Win32(h) = handle.as_raw() {
                        let hwnd = HWND(h.hwnd.get() as *mut _);
                        unsafe {
                            // Re-apply after visibility change to ensure z-order sticks.
                            if let Some(tb) = taskbar_hwnd {
                                let _ = SetWindowPos(
                                    hwnd, tb,
                                    0, 0, 0, 0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED,
                                );
                            } else {
                                let _ = SetWindowPos(
                                    hwnd, HWND_TOPMOST,
                                    0, 0, 0, 0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED,
                                );
                            }
                        }
                    }
                }
            }
        }

        let _ = slots[0].window.set_cursor_hittest(false);

        // Load saved settings (editor geometry etc.)
        if let Some(save_data) = save::load() {
            self.settings = save_data.settings;
        }

        // --- Mod loading ---
        let mods = modding::load_mods(&modding::mods_dir());
        self.available_commands = modding::collect_initial_commands(&mods);
        self.command_defs = modding::collect_command_defs(&mods);

        // Merge sprite/pivot/config registries from all loaded mods.
        for loaded_mod in &mods {
            for (etype, sprite) in &loaded_mod.sprites {
                self.sprite_registry.entry(etype.clone())
                    .or_insert_with(|| SpriteData {
                        png: sprite.png.clone(),
                        json: sprite.json.clone(),
                    });
            }
            for (etype, pivot) in &loaded_mod.pivots {
                self.pivot_registry.entry(etype.clone()).or_insert(*pivot);
            }
            for (etype, config) in &loaded_mod.entity_configs {
                self.entity_configs.entry(etype.clone()).or_insert_with(|| config.clone());
            }
        }

        // --- Unit system init ---
        let mut um = UnitManager::new();
        let mut sim = SimWorld::new(42);

        // Spawn entities defined in mod manifests.
        for loaded_mod in &mods {
            for spawn_def in &loaded_mod.manifest.spawn {
                // Spawn render unit if sprite data is available.
                if let Some(sprite) = self.sprite_registry.get(&spawn_def.entity_type) {
                    let [px, py] = self.pivot_registry
                        .get(&spawn_def.entity_type)
                        .copied()
                        .unwrap_or([24.0, 0.0]);
                    um.spawn(
                        &spawn_def.name,
                        &sprite.png,
                        &sprite.json,
                        spawn_def.position as f32,
                        px,
                        py,
                    );
                }

                // Spawn sim entity with optional stat overrides.
                let config = self.entity_configs.get(&spawn_def.entity_type);
                sim.spawn_entity_with_config(
                    spawn_def.entity_type.clone(),
                    spawn_def.name.clone(),
                    spawn_def.position,
                    config,
                );
            }
        }

        // Register custom command definitions with the sim.
        for def in self.command_defs.values() {
            sim.register_custom_command(def);
        }

        // Copy entity configs to sim for spawn effects.
        for (etype, config) in &self.entity_configs {
            sim.entity_configs.insert(etype.clone(), config.clone());
        }

        // Auto-start simulation — it runs continuously from game open.
        sim.start();

        self.unit_manager = Some(um);
        self.sim_world = Some(sim);

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
        self.last_tick = Instant::now();

        // --- Editor system init ---
        let scripts_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("scripts");
        self.script_store = Some(ScriptStore::new(scripts_dir));

        open_editor(&mut self.webview_manager, &self.ipc_sender, self.settings.editor_window.as_ref().map(|g| (g.x, g.y, g.width, g.height)));

        // Spawn a background thread that sends Tick events every ~33ms.
        // This keeps the game loop alive during Win32 modal loops (window drag).
        #[cfg(target_os = "windows")]
        if !self.tick_thread_started {
            self.tick_thread_started = true;
            let proxy = self.proxy.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(33));
                    if proxy.send_event(UserEvent::Tick).is_err() {
                        break; // Event loop closed.
                    }
                }
            });
        }
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
            }

            WindowEvent::Resized(new_size) => {
                // On macOS, canvas uses logical pixels (CALayer handles DPI).
                // On Windows, canvas must match physical size (softbuffer blits 1:1).
                #[cfg(target_os = "macos")]
                let (resize_w, resize_h) = {
                    let scale = self.monitor_slots
                        .get(self.active_monitor)
                        .map(|s| s.window.scale_factor())
                        .unwrap_or(1.0);
                    ((new_size.width as f64 / scale).round() as u32,
                     (new_size.height as f64 / scale).round() as u32)
                };
                #[cfg(not(target_os = "macos"))]
                let (resize_w, resize_h) = (new_size.width, new_size.height);

                if resize_w == 0 || resize_h == 0 {
                    return;
                }

                if let Some(slot) = self.monitor_slots.get_mut(self.active_monitor) {
                    slot.renderer.resize(resize_w, resize_h);
                }
            }

            WindowEvent::CloseRequested => {
                if let Some((x, y, w, h)) = get_window_geometry(&self.webview_manager) {
                    self.settings.editor_window = Some(save::WindowGeometry { x, y, width: w, height: h });
                }
                save::save(&save::SaveData {
                    last_active_unix: save::now_unix(),
                    settings: self.current_settings(),
                });
                self.webview_manager.cleanup();
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                let slot = match self.monitor_slots.get_mut(self.active_monitor) {
                    Some(s) => s,
                    None => return,
                };

                if let Some(um) = &self.unit_manager {
                    let info = slot.info;

                    #[cfg(target_os = "macos")]
                    let (rw, rh, rd) = (info.monitor_width, info.strip_height, info.dock_height);
                    #[cfg(not(target_os = "macos"))]
                    let (rw, rh, rd) = (info.phys_width, info.phys_height, info.phys_dock_height);

                    slot.surface
                        .resize(
                            std::num::NonZeroU32::new(rw).unwrap(),
                            std::num::NonZeroU32::new(rh).unwrap(),
                        )
                        .expect("Failed to resize surface");

                    slot.renderer.render(
                        &mut slot.surface,
                        rw,
                        rh,
                        um,
                        rd,
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
                save::save(&save::SaveData {
                    last_active_unix: save::now_unix(),
                    settings: self.current_settings(),
                });
                self.webview_manager.cleanup();
                event_loop.exit();
            }
            UserEvent::MenuEvent(ref e) if e.id().0 == tray::editor_id() => {
                if self.webview_manager.is_visible() {
                    if let Some((x, y, w, h)) = get_window_geometry(&self.webview_manager) {
                        self.settings.editor_window = Some(save::WindowGeometry { x, y, width: w, height: h });
                    }
                    self.webview_manager.close();
                } else {
                    if self.webview_manager.is_open() {
                        self.webview_manager.close();
                    }
                    open_editor(&mut self.webview_manager, &self.ipc_sender, self.settings.editor_window.as_ref().map(|g| (g.x, g.y, g.width, g.height)));
                }
            }
            UserEvent::MenuEvent(_) => {}
            UserEvent::Tick => {
                // Keep the game alive during Win32 modal loops (editor drag).
                self.do_tick();
                self.do_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.do_tick();

        // --- Dynamic FPS ---
        let redraw_interval = if self.active_until.map(|t| Instant::now() < t).unwrap_or(false) {
            Duration::from_millis(33) // ~30 FPS when active
        } else {
            Duration::from_millis(100) // 10 FPS when idle
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + redraw_interval));

        self.do_redraw();
    }
}

// ---------------------------------------------------------------------------
// Editor IPC dispatch
// ---------------------------------------------------------------------------

impl App {
    fn send_available_commands(&self) {
        let mut commands: Vec<String> = if deadcode_desktop::is_dev_mode() {
            // In dev mode, send all game builtins as available
            let mut cmds: Vec<String> = vec![
                "move", "get_pos", "scan", "nearest", "distance", "attack",
                "flee", "wait", "set_target", "get_target", "has_target",
                "get_health", "get_energy", "get_shield", "get_type",
                "get_name", "get_owner",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect();
            // Add all custom command names too.
            cmds.extend(self.command_defs.keys().cloned());
            cmds
        } else {
            self.available_commands.iter().cloned().collect()
        };
        commands.sort();
        commands.dedup();

        // Build command info for custom commands (for editor autocomplete).
        let command_info: Vec<CommandInfo> = self.command_defs.values().map(|def| {
            CommandInfo {
                name: def.name.clone(),
                description: def.description.clone(),
                args: def.args.clone(),
            }
        }).collect();

        let msg = RustToJs::AvailableCommands {
            commands,
            dev_mode: deadcode_desktop::is_dev_mode(),
            command_info,
        };
        self.webview_manager.send_to_all(&msg);
    }

    /// Build custom command arg counts map for the compiler.
    fn custom_command_arg_counts(&self) -> std::collections::HashMap<String, usize> {
        self.command_defs.iter().map(|(name, def)| {
            (name.clone(), def.args.len())
        }).collect()
    }

    /// Compile script and assign to summoner's ScriptState in the sim.
    fn handle_run_script_sim(&mut self, script_id: &str) {
        let source = match self.script_store.as_ref().and_then(|s| s.scripts.get(script_id)) {
            Some(script) => script.content.clone(),
            None => return,
        };
        let sid = script_id.to_string();

        // Build available commands for the compiler.
        let available = self.available_commands_for_compiler();
        let custom = self.custom_command_arg_counts();

        // Compile source to IR.
        let compiled = deadcode_sim::compiler::compile_source_full(&source, available, custom);
        match compiled {
            Ok(script) => {
                // Find summoner entity in sim and assign script.
                if let Some(sim) = &mut self.sim_world {
                    // Find the summoner (first entity of type "summoner").
                    let summoner_id = sim.entities()
                        .find(|e| e.entity_type == "summoner" && e.alive)
                        .map(|e| e.id);

                    if let Some(eid) = summoner_id {
                        let num_vars = script.num_variables;
                        let mut state = deadcode_sim::entity::ScriptState::new(script, num_vars);
                        // Set self = EntityRef for the summoner.
                        if !state.variables.is_empty() {
                            state.variables[0] = deadcode_sim::SimValue::EntityRef(eid);
                        }
                        if let Some(entity) = sim.get_entity_mut(eid) {
                            entity.script_state = Some(state);
                        }
                    }
                }
                self.webview_manager.send_to_all(&RustToJs::ScriptStarted {
                    script_id: sid,
                });
            }
            Err(error) => {
                self.webview_manager.send_to_all(&RustToJs::ScriptFinished {
                    script_id: sid,
                    success: false,
                    error: Some(error),
                });
            }
        }
    }

    /// Stop the summoner's script (clear its ScriptState).
    fn handle_stop_script_sim(&mut self, script_id: &str) {
        if let Some(sim) = &mut self.sim_world {
            let summoner_id = sim.entities()
                .find(|e| e.entity_type == "summoner" && e.alive)
                .map(|e| e.id);

            if let Some(eid) = summoner_id {
                if let Some(entity) = sim.get_entity_mut(eid) {
                    entity.script_state = None;
                }
            }
        }
        self.webview_manager.send_to_all(&RustToJs::ScriptFinished {
            script_id: script_id.to_string(),
            success: true,
            error: None,
        });
    }

    /// Get available commands for the compiler (None = all allowed in dev mode).
    fn available_commands_for_compiler(&self) -> Option<HashSet<String>> {
        if deadcode_desktop::is_dev_mode() {
            None // all commands available
        } else {
            Some(self.available_commands.clone())
        }
    }

    fn available_commands_for_interpreter(&self) -> Option<HashSet<String>> {
        self.available_commands_for_compiler()
    }

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
                    self.execution_manager.set_available_commands(
                        self.available_commands_for_interpreter(),
                    );
                    self.send_available_commands();
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
                JsToRust::TabChanged { .. } => {}
                JsToRust::RunScript { script_id } => {
                    self.handle_run_script_sim(&script_id);
                }
                JsToRust::StopScript { script_id } => {
                    self.handle_stop_script_sim(&script_id);
                }
                JsToRust::DebugStart { script_id } => {
                    // Debug uses the same compile→sim path for now.
                    // TODO: IR-level debug stepping support.
                    self.handle_run_script_sim(&script_id);
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
                    if let Some((x, y, w, h)) = get_window_geometry(&self.webview_manager) {
                        self.settings.editor_window = Some(save::WindowGeometry { x, y, width: w, height: h });
                    }
                    self.webview_manager.handle_window_control(
                        WindowControlEvent::Close,
                        &mut self.maximized_state.maximized,
                    );
                }
                JsToRust::WindowDragStart => {}
                JsToRust::WindowResizeStart { .. } => {}
                JsToRust::WindowShake => {
                    self.webview_manager.start_shake();
                }
                JsToRust::WindowSetSize { width, height, resizable } => {
                    self.webview_manager.set_size(width, height, resizable);
                }
                JsToRust::ConsoleCommand { command } => {
                    self.execution_manager.handle_console_command(&command, &self.webview_manager);
                }
                JsToRust::StartSimulation => {
                    // Sim runs continuously from game open — no-op.
                }
                JsToRust::StopSimulation => {
                    // Sim runs continuously from game open — no-op.
                }
                JsToRust::PauseSimulation => {
                    // Sim runs continuously from game open — no-op.
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Context menu display
// ---------------------------------------------------------------------------

#[allow(dead_code)]
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
