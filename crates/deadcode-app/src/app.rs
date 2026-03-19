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
use deadcode_desktop::animation::{SUMMONER_ATLAS_PNG, summoner_atlas_json};
use deadcode_desktop::fullscreen;
use deadcode_desktop::renderer::Renderer;
use deadcode_desktop::save;
use deadcode_desktop::save::Settings;
use deadcode_desktop::tray;
use deadcode_desktop::unit::{UnitManager, WORLD_WIDTH};
use deadcode_desktop::window::{StripInfo, enumerate_monitors};

use deadcode_editor::ipc::{JsToRust, RustToJs, WindowControlEvent};
use deadcode_sim::{EntityType, SimWorld};
use deadcode_editor::window::{WebViewManager, MaximizedState, open_editor, get_window_geometry};
use deadcode_editor::scripts::ScriptStore;
use deadcode_editor::tabs::EditorWindowState;
use deadcode_editor::execution::ScriptExecutionManager;
use grimscript_lang::DebugCommand;

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

                // Forward script output events to editor console.
                let events = sim.take_events();
                for event in &events {
                    match event {
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

        // --- Unit system init ---
        let mut um = UnitManager::new();

        // Summoner — render unit, driven by sim.
        let summoner_json = summoner_atlas_json();
        um.spawn("summoner", SUMMONER_ATLAS_PNG, &summoner_json, 500.0, 49.0, 2.0);

        self.unit_manager = Some(um);

        // --- Simulation init ---
        let mut sim = SimWorld::new(42);
        sim.spawn_entity(EntityType::Mothership, "summoner".into(), 500);
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
                JsToRust::TabChanged { .. } => {}
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
                    eprintln!("[console] command: {}", command);
                }
                JsToRust::StartSimulation => {
                    if self.sim_world.is_none() {
                        self.sim_world = Some(SimWorld::new(42));
                    }
                    if let Some(sim) = &mut self.sim_world {
                        sim.start();
                    }
                    self.webview_manager.send_to_all(&RustToJs::SimulationStarted);
                    eprintln!("[sim] started");
                }
                JsToRust::StopSimulation => {
                    if let Some(sim) = &mut self.sim_world {
                        sim.stop();
                    }
                    self.webview_manager.send_to_all(&RustToJs::SimulationStopped);
                    eprintln!("[sim] stopped");
                }
                JsToRust::PauseSimulation => {
                    if let Some(sim) = &mut self.sim_world {
                        let is_running = sim.is_running();
                        sim.set_paused(is_running);
                        if is_running {
                            self.webview_manager.send_to_all(&RustToJs::SimulationStopped);
                        } else {
                            self.webview_manager.send_to_all(&RustToJs::SimulationStarted);
                        }
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
