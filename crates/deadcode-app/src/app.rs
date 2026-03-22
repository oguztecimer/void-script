use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

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
use deadcode_desktop::unit::UnitManager;
use deadcode_desktop::window::{StripInfo, enumerate_monitors};

use deadcode_editor::ipc::{CommandInfo, JsToRust, RustToJs, WindowControlEvent};
use deadcode_sim::SimWorld;
use deadcode_sim::action::{CommandDef, CommandHandler};
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
    /// Entity def ID → sprite data (PNG bytes + JSON metadata).
    sprite_registry: HashMap<String, SpriteData>,
    /// Entity def ID → pivot [x, y].
    pivot_registry: HashMap<String, [f32; 2]>,
    /// Entity def ID → stat overrides.
    entity_configs: HashMap<String, deadcode_sim::entity::EntityConfig>,
    /// Entity def ID → resolved type tags.
    entity_types: HashMap<String, Vec<String>>,

    // --- Simulation system ---
    sim_world: Option<SimWorld>,
    sim_accumulator: Duration,

    // --- Editor system ---
    webview_manager: WebViewManager,
    ipc_sender: Sender<JsToRust>,
    ipc_receiver: Receiver<JsToRust>,
    script_store: Option<ScriptStore>,
    editor_state: EditorWindowState,
    execution_manager: ScriptExecutionManager,
    maximized_state: MaximizedState,
    settings: Settings,
    /// Available resource names (gated like commands).
    available_resources: Vec<String>,
    /// Command display order (for list_commands).
    command_order: Vec<String>,
    /// Custom command definitions from mods.
    command_defs: HashMap<String, CommandDef>,
    /// GrimScript library source (prepended to player scripts before compilation).
    library_source: String,
    /// Type name → type definition (collected from all mods).
    type_defs: HashMap<String, modding::TypeDef>,
    /// Type name → default script source (from mods' grimscript/ directories).
    type_scripts: HashMap<String, String>,
    /// Mapping from sim EntityId to render UnitId for position sync.
    entity_unit_map: HashMap<u64, u64>,
    /// Whether initial effects (from Lua on_init) still need to be sent to the editor.
    initial_effects_pending: bool,
    /// Buffered sim events from Lua on_init, replayed when editor becomes ready.
    pending_init_events: Vec<deadcode_sim::SimEvent>,

    /// Mod hot-reload: mod_id → (mod.lua path, last modified time).
    mod_lua_watch: Vec<(String, PathBuf, SystemTime)>,
    /// Timer for mod.lua polling.
    last_mod_lua_check: Instant,

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
            entity_types: HashMap::new(),

            // Simulation system
            sim_world: None,
            sim_accumulator: Duration::ZERO,

            // Editor system
            webview_manager: WebViewManager::default(),
            ipc_sender,
            ipc_receiver,
            script_store: None,
            editor_state: EditorWindowState::default(),
            execution_manager: ScriptExecutionManager::default(),
            maximized_state: MaximizedState::default(),
            settings: Settings::default(),
            available_resources: Vec::new(),
            command_order: Vec::new(),
            command_defs: HashMap::new(),
            library_source: String::new(),
            type_defs: HashMap::new(),
            type_scripts: HashMap::new(),
            entity_unit_map: HashMap::new(),
            initial_effects_pending: true,
            pending_init_events: Vec::new(),
            mod_lua_watch: Vec::new(),
            last_mod_lua_check: Instant::now(),

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

        // --- Simulation tick (fixed 30 TPS) ---
        const SIM_TICK_INTERVAL: Duration = Duration::from_millis(33);
        const MAX_SIM_TICKS_PER_FRAME: u32 = 4; // cap to prevent spiral of death

        self.sim_accumulator += delta;
        let mut sim_ticked = false;
        let mut ticks_this_frame = 0u32;

        while self.sim_accumulator >= SIM_TICK_INTERVAL && ticks_this_frame < MAX_SIM_TICKS_PER_FRAME {
            self.sim_accumulator -= SIM_TICK_INTERVAL;
            ticks_this_frame += 1;

            let mut spawned_entities: Vec<(deadcode_sim::entity::EntityId, String)> = Vec::new();
            if let Some(sim) = &mut self.sim_world
                && sim.is_running() {
                    sim.tick();
                    sim_ticked = true;

                    // Advance animations by one sim tick (deterministic).
                    if let Some(um) = &mut self.unit_manager {
                        um.tick_animations();
                        um.reap_dead();
                    }

                    // Forward events to editor console and apply to render units.
                    let events = sim.take_events();
                    // Collect newly spawned entities (id + entity_type) for soul assignment.
                    for event in &events {
                        if let deadcode_sim::SimEvent::EntitySpawned { entity_id, entity_type, .. } = event {
                            spawned_entities.push((*entity_id, entity_type.clone()));
                        }
                        self.forward_sim_event_to_editor(event);
                        self.apply_sim_event_to_units(event);
                    }
                }
            // Assign soul scripts to newly spawned entities (outside sim borrow).
            if !spawned_entities.is_empty() {
                let cmd_meta = self.command_metadata();
                for (eid, etype) in &spawned_entities {
                    if let Some(types) = self.entity_types.get(etype).cloned() {
                        self.compile_and_assign_entity_soul(*eid, &types, &cmd_meta);
                    }
                }
            }
        }

        // Sync sim entity positions to UnitManager (once per frame, after all sim ticks).
        if sim_ticked {
            if let Some(sim) = &self.sim_world {
                let snapshot = sim.snapshot();
                if let Some(um) = &mut self.unit_manager {
                    for es in &snapshot.entities {
                        let render_x = es.position as f32;
                        if let Some(&uid) = self.entity_unit_map.get(&es.id.0) {
                            um.move_to(uid, render_x, 100.0);
                        }
                    }
                }

                let msg = RustToJs::SimulationTick { tick: snapshot.tick };
                self.webview_manager.send_to_all(&msg);

                // Send available global resource values to the editor.
                let resources: Vec<_> = sim.resources.iter()
                    .filter(|(name, _)| {
                        sim.available_resources.as_ref()
                            .is_none_or(|set| set.contains(name.as_str()))
                    })
                    .map(|(name, &value)| deadcode_editor::ipc::ResourceValue {
                        name: name.clone(),
                        value,
                        max_value: sim.resource_caps.get(name).copied(),
                    })
                    .collect();
                self.webview_manager.send_to_all(&RustToJs::ResourceUpdate { resources });
            }

            self.active_until = Some(Instant::now() + Duration::from_secs(1));
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

        // --- Mod.lua hot-reload polling (every 1s) ---
        if self.last_mod_lua_check.elapsed() >= Duration::from_secs(1) {
            self.last_mod_lua_check = Instant::now();
            self.poll_mod_lua_reload();
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
        if !self.is_hidden_for_fullscreen
            && let Some(slot) = self.monitor_slots.get(self.active_monitor) {
                slot.window.request_redraw();
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
                    if let Ok(handle) = window.window_handle()
                        && let RawWindowHandle::AppKit(h) = handle.as_raw() {
                            use objc2::msg_send;
                            let ns_window: *mut objc2::runtime::AnyObject = unsafe {
                                msg_send![h.ns_view.cast::<objc2::runtime::AnyObject>().as_ref(), window]
                            };
                            if !ns_window.is_null() {
                                let _: () = unsafe { msg_send![ns_window, setHasShadow: false] };
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
                renderer.pixel_scale = (info.monitor_width / deadcode_desktop::unit::DEFAULT_WORLD_WIDTH).max(1);
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

        // --- Mod loading (with dependency resolution) ---
        let mods = modding::load_mods(&modding::mods_dir());
        modding::validate_dependencies(&mods);
        self.available_resources = modding::collect_available_resources(&mods);
        self.library_source = modding::collect_library_source(&mods);
        self.type_defs = modding::collect_type_defs(&mods);
        self.type_scripts = modding::collect_type_scripts(&mods);

        // Merge sprite/pivot/config registries from all loaded mods.
        for loaded_mod in &mods {
            for (etype, sprite) in &loaded_mod.sprites {
                if self.sprite_registry.contains_key(etype) {
                    eprintln!(
                        "[mod] warning: entity type '{}' already defined, skipping from '{}'",
                        etype, loaded_mod.manifest.meta.id
                    );
                } else {
                    self.sprite_registry.insert(etype.clone(), SpriteData {
                        png: sprite.png.clone(),
                        json: sprite.json.clone(),
                    });
                }
            }
            for (etype, pivot) in &loaded_mod.pivots {
                self.pivot_registry.entry(etype.clone()).or_insert(*pivot);
            }
            for (etype, config) in &loaded_mod.entity_configs {
                self.entity_configs.entry(etype.clone()).or_insert_with(|| config.clone());
            }
            for (etype, types) in &loaded_mod.entity_types {
                self.entity_types.entry(etype.clone()).or_insert_with(|| types.clone());
            }
        }

        // Validate type and entity definitions.
        modding::validate_type_defs(&mods);
        let rejected_entities = modding::validate_entity_defs(&mods, &self.type_defs);
        for id in &rejected_entities {
            self.entity_configs.remove(id);
            self.entity_types.remove(id);
            self.sprite_registry.remove(id);
            self.pivot_registry.remove(id);
        }

        // Collect world_width from mods (first-defined wins).
        let world_width = mods.iter()
            .find_map(|m| m.manifest.meta.world_width)
            .unwrap_or(deadcode_sim::DEFAULT_WORLD_WIDTH);

        // Update pixel_scale for all monitor slots now that we know world_width.
        for slot in &mut self.monitor_slots {
            slot.renderer.pixel_scale = (slot.info.monitor_width / world_width as u32).max(1);
        }

        // --- Unit system init ---
        let mut um = UnitManager::new();
        um.world_width = world_width as u32;
        let mut sim = SimWorld::new(42);
        sim.world_width = world_width;

        self.entity_unit_map.clear();

        // Register buff definitions from all loaded mods.
        for buff in modding::collect_buffs(&mods) {
            sim.register_buff(buff);
        }

        // Initialize global resources from mod definitions.
        let collected = modding::collect_initial_resources(&mods);
        sim.resources = collected.values;
        sim.resource_caps = collected.caps;

        // Set available resources (None = all available in dev mode).
        sim.available_resources = if deadcode_desktop::is_dev_mode() {
            None
        } else {
            Some(self.available_resources.iter().cloned().collect())
        };

        // Copy entity configs and type mappings to sim for spawn effects.
        for (etype, config) in &self.entity_configs {
            sim.entity_configs.insert(etype.clone(), config.clone());
        }
        for (etype, types) in &self.entity_types {
            sim.entity_types_registry.insert(etype.clone(), types.clone());
        }

        // Compute spawn animation durations from sprite atlas metadata.
        for (etype, sprite) in &self.sprite_registry {
            let ticks = deadcode_desktop::animation::spawn_animation_ticks(&sprite.json);
            if ticks > 0 {
                sim.spawn_durations.insert(etype.clone(), ticks);
            }
        }

        // Spawn the grimoire entity (real entity, no sprite).
        sim.spawn_grimoire_entity();

        // Auto-start simulation — it runs continuously from game open.
        sim.start();

        // --- Lua runtime init ---
        // Create the Lua mod runtime and load mod.lua files.
        if let Some(mut lua_runtime) = modding::create_lua_runtime(&mods) {
            // Register Lua command metadata with the sim for the compiler and list_commands.
            let lua_meta = lua_runtime.command_metadata();
            for (name, meta) in &lua_meta {
                let cmd_def = deadcode_sim::CommandDef {
                    name: name.clone(),
                    description: meta.description.clone(),
                    args: meta.args.clone(),
                    unlisted: meta.unlisted,
                    kind: meta.kind.clone(),
                    ..Default::default()
                };
                sim.register_custom_command(&cmd_def);
                self.command_order.push(name.clone());
                self.command_defs.entry(name.clone()).or_insert(cmd_def);
            }
            sim.command_order = self.command_order.clone();

            // Run init handlers (replaces [initial].effects for Lua mods).
            {
                let caster_id = sim.grimoire_id().unwrap_or(deadcode_sim::EntityId(1));
                let mut access = deadcode_sim::WorldAccess::new_from_world_ptr(&mut sim, caster_id);
                let init_events = lua_runtime.run_init(&mut access);
                let access_events = std::mem::take(&mut access.events);
                // Buffer init events to replay when the editor becomes ready.
                self.pending_init_events = access_events.into_iter().chain(init_events).collect();
            }

            sim.command_handler = Some(Box::new(lua_runtime));
        }

        // Populate mod.lua file watch list for hot-reload.
        for m in &mods {
            let lua_path = m.mod_dir.join("mod.lua");
            if lua_path.exists()
                && let Ok(meta) = std::fs::metadata(&lua_path) {
                    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    self.mod_lua_watch.push((m.manifest.meta.id.clone(), lua_path, mtime));
                }
        }

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
        let mut store = ScriptStore::new(scripts_dir);

        // Ensure type scripts exist in scripts/types/ directory.
        let mut type_script_defs: Vec<(String, bool, String)> = self.type_defs.iter()
            .map(|(name, tdef)| {
                let default_src = self.type_scripts.get(name).cloned().unwrap_or_default();
                // "grimoire" is always a soul regardless of mod.toml soul flag.
                let is_soul = tdef.soul || name == "grimoire";
                (name.clone(), is_soul, default_src)
            })
            .collect();
        // Always include grimoire.gs (the grimoire script).
        if !type_script_defs.iter().any(|(n, _, _)| n == "grimoire") {
            let grimoire_default = self.type_scripts.get("grimoire").cloned()
                .unwrap_or_else(|| "# Grimoire — runs every tick before entities\n# No self, no position — use resource ops, queries, print\n".to_string());
            type_script_defs.push(("grimoire".to_string(), true, grimoire_default));
        }
        store.ensure_type_scripts(&type_script_defs);

        self.script_store = Some(store);

        // --- Compile soul scripts and assign to entities ---
        // Must run after script store is initialized so get_type_script_source
        // can read user scripts from scripts/types/.
        self.compile_and_assign_all_souls();

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
        let is_active = self.active_until.map(|t| Instant::now() < t).unwrap_or(false);
        let has_pending_ipc = !self.ipc_receiver.is_empty();
        let redraw_interval = if is_active || has_pending_ipc {
            Duration::from_millis(33) // ~30 FPS when active or IPC pending
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
        // All commands come from mod definitions now.
        let commands: Vec<String> = self.command_defs.keys().cloned().collect();

        let resources: Vec<String> = if deadcode_desktop::is_dev_mode() {
            // In dev mode, all defined resources are available.
            self.sim_world.as_ref()
                .map(|sim| sim.resources.keys().cloned().collect())
                .unwrap_or_default()
        } else {
            self.available_resources.clone()
        };

        // Build command info for all commands (for editor autocomplete).
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
            resources,
        };
        self.webview_manager.send_to_all(&msg);
    }

    /// Build command metadata map for the compiler (all commands from mod definitions).
    fn command_metadata(&self) -> std::collections::HashMap<String, deadcode_sim::compiler::CommandMeta> {
        self.command_defs.iter().map(|(name, def)| {
            (name.clone(), deadcode_sim::compiler::CommandMeta {
                num_args: def.args.len(),
                kind: def.kind.clone(),
                implicit_self: def.implicit_self,
            })
        }).collect()
    }


    /// Compile and execute a console command through the sim.
    ///
    /// The command is compiled to IR and executed against the grimoire
    /// entity. Actions are resolved and events forwarded to the editor.
    /// This ensures custom commands, queries, and all builtins work in
    /// the console.
    fn handle_console_command_sim(&mut self, source: &str) {
        // Terminal uses the grimoire entity's effective commands (type-gated).
        let available = self.effective_commands_for_grimoire();
        let custom = self.command_metadata();

        // Prepend library functions from mods.
        let full_source = self.prepend_library_source(source);

        let compiled = deadcode_sim::compiler::compile_source_full(&full_source, available, custom, false);
        match compiled {
            Ok(script) => {
                if let Some(sim) = &mut self.sim_world {
                    // Terminal commands run as the grimoire entity.
                    let main_id = sim.grimoire_id()
                        .unwrap_or(deadcode_sim::entity::EntityId(0));

                    let num_vars = script.num_variables;
                    let state = deadcode_sim::entity::ScriptState::new(script, num_vars);

                    // Execute until halt or action, collecting events.
                    let mut all_events = Vec::new();
                    let mut error_msg = None;
                    let mut state = state;
                    loop {
                        match deadcode_sim::executor::execute_unit(main_id, &mut state, sim) {
                            Ok(Some(action)) => {
                                // For terminal: resolve phased commands immediately
                                // (all on_start effects from all phases in sequence)
                                // since there's no entity to store channel state on.
                                let action_events = deadcode_sim::action::resolve_action(sim, main_id, action);
                                all_events.extend(action_events);
                            }
                            Ok(None) => break, // Script finished.
                            Err(err) => {
                                error_msg = Some(err.to_string());
                                break;
                            }
                        }
                    }

                    // Forward collected events to editor and apply animations.
                    for event in &all_events {
                        self.forward_sim_event_to_editor(event);
                        self.apply_sim_event_to_units(event);
                    }
                    if let Some(err) = error_msg {
                        self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                            text: format!("[error] {err}"),
                            level: "error".to_string(),
                        });
                    }
                }
            }
            Err(error) => {
                self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                    text: error,
                    level: "error".to_string(),
                });
            }
        }
        self.webview_manager.send_to_all(&RustToJs::TerminalFinished {
            success: true,
            error: None,
        });
    }

    /// Apply a sim event to the render units (spawn, death, animations).
    fn apply_sim_event_to_units(&mut self, event: &deadcode_sim::SimEvent) {
        match event {
            deadcode_sim::SimEvent::EntitySpawned { entity_id, entity_type, name, position, .. } => {
                if let Some(sprite) = self.sprite_registry.get(entity_type)
                    && let Some(um) = &mut self.unit_manager {
                        let [px, py] = self.pivot_registry
                            .get(entity_type)
                            .copied()
                            .unwrap_or([24.0, 0.0]);
                        let uid = um.spawn(
                            name,
                            &sprite.png,
                            &sprite.json,
                            *position as f32,
                            px,
                            py,
                        );
                        self.entity_unit_map.insert(entity_id.0, uid);
                    }
            }
            deadcode_sim::SimEvent::EntityDied { entity_id, .. } => {
                if let Some(um) = &mut self.unit_manager
                    && let Some(&uid) = self.entity_unit_map.get(&entity_id.0) {
                        um.kill(uid);
                        self.entity_unit_map.remove(&entity_id.0);
                    }
            }
            deadcode_sim::SimEvent::PlayAnimation { entity_id, animation } => {
                if let Some(um) = &mut self.unit_manager
                    && let Some(&uid) = self.entity_unit_map.get(&entity_id.0) {
                        um.play_animation(uid, animation);
                    }
            }
            deadcode_sim::SimEvent::EntityFlipped { entity_id, facing_left } => {
                if let Some(um) = &mut self.unit_manager
                    && let Some(&uid) = self.entity_unit_map.get(&entity_id.0) {
                        um.set_facing(uid, *facing_left);
                    }
            }
            _ => {}
        }
    }

    /// Forward a sim event to the editor as a console message.
    fn forward_sim_event_to_editor(&self, event: &deadcode_sim::SimEvent) {
        match event {
            deadcode_sim::SimEvent::ScriptOutput { text, .. } => {
                self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                    text: text.clone(),
                    level: "info".to_string(),
                });
            }
            deadcode_sim::SimEvent::ScriptError { entity_id, error, variables, stack, pc } => {
                self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                    text: format!("[sim error] {error}"),
                    level: "error".to_string(),
                });
                self.webview_manager.send_to_all(&RustToJs::ScriptErrorDetail {
                    entity_id: entity_id.0,
                    error: error.clone(),
                    variables: variables.clone(),
                    stack: stack.clone(),
                    pc: *pc,
                });
            }
            _ => {}
        }
    }


    /// Get effective commands for the grimoire entity (type-gated).
    fn effective_commands_for_grimoire(&self) -> Option<HashSet<String>> {
        if deadcode_desktop::is_dev_mode() {
            return None;
        }
        // Get grimoire entity's types.
        let main_types = self.sim_world.as_ref()
            .and_then(|sim| sim.grimoire_id())
            .and_then(|eid| self.sim_world.as_ref().unwrap().get_entity(eid))
            .map(|e| e.types.clone())
            .unwrap_or_else(|| vec!["grimoire".to_string()]);
        modding::compute_effective_commands(&main_types, &self.type_defs)
    }

    /// Prepend mod library source to player script source.
    /// Library functions are defined before the player's code, making them
    /// available as if they were part of the script.
    fn prepend_library_source(&self, source: &str) -> String {
        if self.library_source.is_empty() {
            source.to_string()
        } else {
            format!("{}\n{}", self.library_source, source)
        }
    }

    /// Compile and assign soul scripts to all entities and the grimoire.
    /// Called at startup and during auto-reload.
    fn compile_and_assign_all_souls(&mut self) {
        let cmd_meta = self.command_metadata();

        // Compile and assign grimoire.
        let main_source = self.get_type_script_source("grimoire");
        if !main_source.is_empty() {
            let available = self.effective_commands_for_grimoire();
            let full_source = self.prepend_library_source(&main_source);
            let enable_soul_loop = deadcode_sim::compiler::source_defines_function(&main_source, "soul");
            match deadcode_sim::compiler::compile_source_full(&full_source, available, cmd_meta.clone(), enable_soul_loop) {
                Ok(script) => {
                    if let Some(sim) = &mut self.sim_world {
                        let num_vars = script.num_variables;
                        let state = deadcode_sim::entity::ScriptState::new(script, num_vars);
                        sim.grimoire = Some(state);
                    }
                }
                Err(err) => {
                    eprintln!("[soul] error compiling grimoire.gs: {err}");
                }
            }
        }

        // Collect entity info before borrowing sim mutably.
        let entity_info: Vec<(deadcode_sim::entity::EntityId, Vec<String>)> =
            if let Some(sim) = &self.sim_world {
                sim.entities()
                    .filter(|e| e.alive)
                    .map(|e| (e.id, e.types.clone()))
                    .collect()
            } else {
                return;
            };

        for (eid, types) in entity_info {
            self.compile_and_assign_entity_soul(eid, &types, &cmd_meta);
        }
    }

    /// Compile and assign a soul script to a single entity.
    fn compile_and_assign_entity_soul(
        &mut self,
        eid: deadcode_sim::entity::EntityId,
        types: &[String],
        cmd_meta: &HashMap<String, deadcode_sim::compiler::CommandMeta>,
    ) {
        // Find the soul type for this entity.
        let soul_type = types.iter()
            .find(|t| self.type_defs.get(*t).is_some_and(|td| td.soul));

        let soul_type = match soul_type {
            Some(bt) => bt.clone(),
            None => return, // No soul type — entity doesn't execute scripts.
        };

        // Get soul script source.
        let soul_source = self.get_type_script_source(&soul_type);
        if soul_source.is_empty() {
            // Empty script — clear the entity's script state so it stops executing.
            if let Some(sim) = &mut self.sim_world
                && let Some(entity) = sim.get_entity_mut(eid) {
                    entity.script_state = None;
                    entity.active_channel = None;
                }
            return;
        }

        // Build library source from non-soul types' scripts.
        let mut type_lib_source = String::new();
        for t in types {
            if self.type_defs.get(t).is_some_and(|td| td.soul) {
                continue; // Skip soul types in library.
            }
            let src = self.get_type_script_source(t);
            if !src.is_empty() {
                if !type_lib_source.is_empty() {
                    type_lib_source.push('\n');
                }
                type_lib_source.push_str(&src);
            }
        }

        // Compose: type library + mod library + soul script.
        let mut full_source = String::new();
        if !type_lib_source.is_empty() {
            full_source.push_str(&type_lib_source);
            full_source.push('\n');
        }
        if !self.library_source.is_empty() {
            full_source.push_str(&self.library_source);
            full_source.push('\n');
        }
        full_source.push_str(&soul_source);

        // Compute effective commands for this entity (type-gated).
        let available = if deadcode_desktop::is_dev_mode() {
            None
        } else {
            modding::compute_effective_commands(types, &self.type_defs)
        };

        // Pre-scan: does the soul type's own script define soul()?
        let enable_soul_loop = deadcode_sim::compiler::source_defines_function(&soul_source, "soul");

        match deadcode_sim::compiler::compile_source_full(&full_source, available, cmd_meta.clone(), enable_soul_loop) {
            Ok(script) => {
                if let Some(sim) = &mut self.sim_world {
                    let num_vars = script.num_variables;
                    let mut state = deadcode_sim::entity::ScriptState::new(script, num_vars);
                    // Set self = EntityRef.
                    if !state.variables.is_empty() {
                        state.variables[0] = deadcode_sim::SimValue::EntityRef(eid);
                    }
                    if let Some(entity) = sim.get_entity_mut(eid) {
                        entity.script_state = Some(state);
                        entity.active_channel = None;
                    }
                }
            }
            Err(err) => {
                eprintln!("[soul] error compiling {soul_type}.gs for entity: {err}");
            }
        }
    }

    /// Get the source code for a type script.
    /// Checks the script store first (user edits), falls back to mod defaults.
    fn get_type_script_source(&self, type_name: &str) -> String {
        // Check script store for user-edited version.
        if let Some(store) = &self.script_store
            && let Some(script) = store.find_type_script(type_name) {
                return script.content.clone();
            }
        // Fall back to mod default.
        self.type_scripts.get(type_name).cloned().unwrap_or_default()
    }

    /// Poll mod.lua files for changes and hot-reload when modified.
    fn poll_mod_lua_reload(&mut self) {
        let mut reloads: Vec<(String, String)> = Vec::new();

        for (mod_id, path, last_mtime) in &mut self.mod_lua_watch {
            let current_mtime = std::fs::metadata(&*path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            if current_mtime > *last_mtime {
                *last_mtime = current_mtime;
                match std::fs::read_to_string(&*path) {
                    Ok(source) => {
                        reloads.push((mod_id.clone(), source));
                    }
                    Err(e) => {
                        eprintln!("[mod reload] failed to read {}: {e}", path.display());
                    }
                }
            }
        }

        for (mod_id, source) in reloads {
            if let Some(sim) = &mut self.sim_world
                && let Some(handler) = &mut sim.command_handler {
                    match handler.reload_mod(&mod_id, &source) {
                        Ok(()) => {
                            // Re-register command metadata after reload.
                            let meta = handler.command_metadata();
                            for (name, cmd_meta) in &meta {
                                let cmd_def = deadcode_sim::CommandDef {
                                    name: name.clone(),
                                    description: cmd_meta.description.clone(),
                                    args: cmd_meta.args.clone(),
                                    unlisted: cmd_meta.unlisted,
                                    kind: cmd_meta.kind.clone(),
                                    ..Default::default()
                                };
                                sim.register_custom_command(&cmd_def);
                                if !self.command_order.contains(name) {
                                    self.command_order.push(name.clone());
                                }
                                self.command_defs.insert(name.clone(), cmd_def);
                            }
                            sim.command_order = self.command_order.clone();

                            let msg = format!("[mod] reloaded {mod_id}/mod.lua");
                            eprintln!("{msg}");
                            self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                                text: msg,
                                level: "info".into(),
                            });
                        }
                        Err(e) => {
                            let msg = format!("[mod] error reloading {mod_id}/mod.lua: {e}");
                            eprintln!("{msg}");
                            self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                                text: msg,
                                level: "error".into(),
                            });
                        }
                    }
                }
        }
    }

    /// Handle auto-reload when a type script is saved.
    /// Recompiles and hot-swaps all affected entities.
    fn handle_type_script_reload(&mut self, type_name: &str) {
        let cmd_meta = self.command_metadata();

        if type_name == "grimoire" {
            // Recompile grimoire.
            let main_source = self.get_type_script_source("grimoire");
            if main_source.is_empty() {
                // Empty script — clear the grimoire so it stops executing.
                if let Some(sim) = &mut self.sim_world {
                    sim.grimoire = None;
                }
                return;
            }
            {
                let available = self.effective_commands_for_grimoire();
                let full_source = self.prepend_library_source(&main_source);
                let enable_soul_loop = deadcode_sim::compiler::source_defines_function(&main_source, "soul");
                match deadcode_sim::compiler::compile_source_full(&full_source, available, cmd_meta, enable_soul_loop) {
                    Ok(script) => {
                        if let Some(sim) = &mut self.sim_world {
                            let num_vars = script.num_variables;
                            let state = deadcode_sim::entity::ScriptState::new(script, num_vars);
                            sim.grimoire = Some(state);
                        }
                        self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                            text: "[reload] grimoire.gs recompiled and loaded".to_string(),
                            level: "info".to_string(),
                        });
                    }
                    Err(err) => {
                        self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
                            text: format!("[error] grimoire.gs: {err}"),
                            level: "error".to_string(),
                        });
                    }
                }
            }
            return;
        }

        // Determine which entities are affected.
        let is_soul = self.type_defs.get(type_name).is_some_and(|td| td.soul);

        let affected: Vec<(deadcode_sim::entity::EntityId, Vec<String>)> = if let Some(sim) = &self.sim_world {
            sim.entities()
                .filter(|e| e.alive && e.has_type(type_name))
                .map(|e| (e.id, e.types.clone()))
                .collect()
        } else {
            return;
        };

        if affected.is_empty() {
            return;
        }

        let cmd_meta = self.command_metadata();
        let count = affected.len();
        for (eid, types) in affected {
            if is_soul {
                // Soul type changed — recompile this entity's soul.
                self.compile_and_assign_entity_soul(eid, &types, &cmd_meta);
            } else {
                // Non-soul type changed — library changed, find the soul and recompile.
                let has_soul = types.iter()
                    .any(|t| self.type_defs.get(t).is_some_and(|td| td.soul));
                if has_soul {
                    self.compile_and_assign_entity_soul(eid, &types, &cmd_meta);
                }
            }
        }

        self.webview_manager.send_to_all(&RustToJs::ConsoleOutput {
            text: format!("[reload] {type_name}.gs recompiled for {count} entities"),
            level: "info".to_string(),
        });
        self.webview_manager.send_to_all(&RustToJs::ScriptReloaded {
            type_name: type_name.to_string(),
        });
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
                    // For the tree-walking interpreter, all mod commands are "custom commands"
                    // and availability is not gated (gating happens at the compiler level).
                    self.execution_manager.set_available_commands(None);
                    self.execution_manager.set_custom_commands(
                        Some(self.command_defs.keys().cloned().collect()),
                    );
                    self.send_available_commands();

                    // Flush pending spawns from Lua init and assign soul scripts.
                    // Replay buffered init events (from Lua on_init) now that the editor is ready.
                    let init_events = std::mem::take(&mut self.pending_init_events);
                    for event in &init_events {
                        self.forward_sim_event_to_editor(event);
                    }

                    if self.initial_effects_pending {
                        self.initial_effects_pending = false;
                        let events = if let Some(sim) = &mut self.sim_world {
                            sim.flush_pending();
                            sim.take_events()
                        } else {
                            Vec::new()
                        };
                        for event in &events {
                            self.forward_sim_event_to_editor(event);
                            self.apply_sim_event_to_units(event);
                        }
                        self.compile_and_assign_all_souls();
                    }
                }
                JsToRust::ScriptSave { script_id, content } => {
                    // Check if this is a type script — trigger auto-reload.
                    let type_name = self.script_store.as_ref()
                        .and_then(|s| s.scripts.get(&script_id))
                        .filter(|s| matches!(s.script_type,
                            deadcode_editor::scripts::ScriptType::TypeSoul |
                            deadcode_editor::scripts::ScriptType::TypeLibrary))
                        .map(|s| s.name.clone());

                    if let Some(store) = &mut self.script_store {
                        store.save_script(&script_id, content);
                    }
                    self.editor_state.set_modified(&script_id, false);

                    // Auto-reload if it was a type script.
                    if let Some(tname) = type_name {
                        self.handle_type_script_reload(&tname);
                    }
                }
                JsToRust::ScriptRequest { script_id } => {
                    if let Some(store) = &self.script_store
                        && let Some(script) = store.scripts.get(&script_id) {
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
                JsToRust::ScriptListRequest => {
                    if let Some(store) = &self.script_store {
                        let infos = store.get_script_infos();
                        let msg = RustToJs::ScriptList { scripts: infos };
                        self.webview_manager.send_to_all(&msg);
                    }
                }
                JsToRust::TabChanged { .. } => {}
                JsToRust::DebugStart { script_id } => {
                    // Type scripts use auto-reload path.
                    let type_name = self.script_store.as_ref()
                        .and_then(|s| s.scripts.get(&script_id))
                        .filter(|s| matches!(s.script_type,
                            deadcode_editor::scripts::ScriptType::TypeSoul |
                            deadcode_editor::scripts::ScriptType::TypeLibrary))
                        .map(|s| s.name.clone());
                    if let Some(tname) = type_name {
                        self.handle_type_script_reload(&tname);
                    }
                    // TODO: IR-level debug stepping support.
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
                    self.handle_console_command_sim(&command);
                }
                JsToRust::StartSimulation => {
                    if let Some(sim) = &mut self.sim_world {
                        sim.start();
                    }
                }
                JsToRust::StopSimulation => {
                    if let Some(sim) = &mut self.sim_world {
                        sim.stop();
                    }
                }
                JsToRust::PauseSimulation => {
                    if let Some(sim) = &mut self.sim_world {
                        sim.set_paused(true);
                    }
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

        if let (Some(w), Some(menu)) = (&self.window, &self.context_menu)
            && let Ok(handle) = w.window_handle()
                && let RawWindowHandle::AppKit(h) = handle.as_raw() {
                    unsafe {
                        menu.show_context_menu_for_nsview(h.ns_view.as_ptr() as _, None);
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
