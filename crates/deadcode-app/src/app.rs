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
use deadcode_desktop::animation::{SKELETON_ATLAS_PNG, skeleton_atlas_json, SUMMONER_ATLAS_PNG, summoner_atlas_json};
use deadcode_desktop::fullscreen;
use deadcode_desktop::renderer::Renderer;
use deadcode_desktop::save;
use deadcode_desktop::save::Settings;
use deadcode_desktop::tray;
use deadcode_desktop::unit::{UnitManager, WORLD_WIDTH};
use deadcode_desktop::window::{StripInfo, enumerate_monitors};

use deadcode_editor::ipc::{JsToRust, RustToJs, WindowControlEvent};
use deadcode_editor::window::{WebViewManager, MaximizedState, open_editor, get_window_geometry};
use deadcode_editor::scripts::ScriptStore;
use deadcode_editor::tabs::EditorWindowState;
use deadcode_editor::execution::ScriptExecutionManager;
use deadcode_lang::DebugCommand;

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
            last_tick: Instant::now(),
            active_until: None,
            context_menu: None,
            cursor_position: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            save_timer: Duration::ZERO,
            unit_manager: None,

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

                let mut renderer = Renderer::new(info.monitor_width, info.strip_height);
                renderer.pixel_scale = (info.monitor_width / WORLD_WIDTH).max(1);
                renderer.set_window(&window);

                MonitorSlot { window, surface, renderer, info }
            })
            .collect();

        // On Windows, invisible windows don't receive RedrawRequested events,
        // so we must make the active window visible before entering the event loop.
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

            slots[0].window.set_visible(true);
            self.first_frame = false;

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

        // Load saved settings (editor geometry etc.)
        if let Some(save_data) = save::load() {
            self.settings = save_data.settings;
        }

        // --- Unit system init ---
        let mut um = UnitManager::new();

        // Skeleton.
        let skeleton_json = skeleton_atlas_json();
        let id = um.spawn("skeleton", SKELETON_ATLAS_PNG, &skeleton_json, 500.0,23.0,0.0);
        um.move_to(id, 600.0, 30.0);

        // Summoner (behind skeletons).
        let summoner_json = summoner_atlas_json();
        let summoner_id = um.spawn("summoner", SUMMONER_ATLAS_PNG, &summoner_json, 300.0,49.0,2.0);
        if let Some(s) = um.get_mut(summoner_id) {
            s.z_order = -1;
        }

        self.unit_manager = Some(um);

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

                    slot.surface
                        .resize(
                            std::num::NonZeroU32::new(info.monitor_width).unwrap(),
                            std::num::NonZeroU32::new(info.strip_height).unwrap(),
                        )
                        .expect("Failed to resize surface");

                    slot.renderer.render(
                        &mut slot.surface,
                        info.monitor_width,
                        info.strip_height,
                        um,
                        info.dock_height,
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
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_tick);
        self.last_tick = now;

        // --- Unit system tick ---
        if let Some(um) = &mut self.unit_manager {
            um.tick(delta);
            // Random wandering: pick a new target when idle.
            let idle: Vec<_> = um.iter()
                .filter(|u| u.movement.is_none() && u.name == "skeleton")
                .map(|u| u.id)
                .collect();
            for id in idle {
                let seed = now.elapsed().as_nanos() as u32;
                let target = (seed % 1000) as f32;
                let speed = 10.0 + (seed % 20) as f32;
                um.move_to(id, target, speed);
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

        // --- Dynamic FPS ---
        let redraw_interval = if self.active_until.map(|t| Instant::now() < t).unwrap_or(false) {
            Duration::from_millis(33) // ~30 FPS when active
        } else {
            Duration::from_millis(100) // 10 FPS when idle
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + redraw_interval));

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
