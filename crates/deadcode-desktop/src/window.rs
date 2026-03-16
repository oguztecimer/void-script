use std::sync::Arc;

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowLevel};

/// Information about the strip window position and dimensions.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct StripInfo {
    /// Height of the strip window in logical pixels.
    pub strip_height: u32,
    /// Full width of the monitor in logical pixels.
    pub monitor_width: u32,
    /// Y coordinate (top of strip) in logical pixels.
    pub strip_y: i32,
    /// Full height of the monitor in logical pixels (Phase 4 multi-monitor field).
    pub monitor_height: u32,
    /// Left edge of the monitor in logical pixels in virtual screen coordinates (Phase 4 multi-monitor field).
    pub monitor_x: i32,
    /// Index of this monitor in the monitor list; 0 = primary (Phase 4 multi-monitor field).
    pub monitor_index: usize,
}

/// Create the transparent, frameless, always-on-top strip window.
///
/// The strip is positioned at the bottom of the work area (above the taskbar)
/// with the full monitor width and a height of 96 logical pixels.
///
/// Returns the Arc<Window> and strip position info.
///
/// Note: `App::resumed()` uses `enumerate_monitors()` for multi-monitor setups, so this
/// function is kept as a utility/fallback but not called directly from App.
#[allow(dead_code)]
pub fn create_strip_window(event_loop: &ActiveEventLoop) -> (Arc<Window>, StripInfo) {
    let strip_height: u32 = 96;

    // Get primary monitor info.
    let monitor = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next())
        .expect("No monitors found");

    let monitor_size = monitor.size(); // physical pixels
    let scale_factor = monitor.scale_factor();

    // Physical to logical pixels.
    let monitor_width_phys = monitor_size.width;
    let monitor_height_phys = monitor_size.height;
    let monitor_width = (monitor_width_phys as f64 / scale_factor) as u32;
    let monitor_height = (monitor_height_phys as f64 / scale_factor) as u32;
    let monitor_pos = monitor.position(); // physical position of monitor
    let monitor_x = (monitor_pos.x as f64 / scale_factor) as i32;

    // Detect work area bottom (above OS taskbar) via platform-specific code.
    let work_area_bottom = get_work_area_bottom(monitor_width, monitor_height, scale_factor);

    let strip_y = work_area_bottom - strip_height as i32;

    let attrs = Window::default_attributes()
        .with_title("good-boi")
        .with_transparent(true)
        .with_decorations(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_resizable(false)
        .with_visible(false) // Shown after first paint to avoid white flash
        .with_inner_size(LogicalSize::new(monitor_width as f64, strip_height as f64))
        .with_position(LogicalPosition::new(monitor_x as f64, strip_y as f64));

    let window = Arc::new(
        event_loop
            .create_window(attrs)
            .expect("Failed to create strip window"),
    );

    let info = StripInfo {
        strip_height,
        monitor_width,
        strip_y,
        monitor_height,
        monitor_x,
        monitor_index: 0, // Primary monitor window is always index 0.
    };

    (window, info)
}

/// Enumerate all connected monitors and return a `Vec<StripInfo>` sorted by `monitor_x`
/// (left-to-right order).
///
/// This is used by Plan 03 (multi-monitor roaming) to discover adjacent monitors so the
/// dog can walk off the edge of one screen and appear on the next.
///
/// # Notes on work-area detection for secondary monitors
///
/// `get_work_area_bottom()` currently targets the *primary* monitor only
/// (MonitorFromPoint(0,0) on Windows, NSScreen.mainScreen on macOS). For secondary monitors
/// we fall back to `monitor_height - 40` (typical taskbar height) as an approximation.
/// Accurate secondary-monitor taskbar detection is a Phase 5 polish concern.
#[allow(dead_code)]
pub fn enumerate_monitors(event_loop: &ActiveEventLoop) -> Vec<StripInfo> {
    let strip_height: u32 = 96;

    // Determine which monitor is primary so we can use accurate work-area detection for it.
    let primary = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next());

    let mut monitors: Vec<StripInfo> = event_loop
        .available_monitors()
        .enumerate()
        .map(|(index, monitor)| {
            let monitor_size = monitor.size(); // physical pixels
            let scale_factor = monitor.scale_factor();

            let monitor_width = (monitor_size.width as f64 / scale_factor) as u32;
            let monitor_height = (monitor_size.height as f64 / scale_factor) as u32;

            let monitor_pos = monitor.position(); // physical position
            let monitor_x = (monitor_pos.x as f64 / scale_factor) as i32;

            // Use accurate work-area detection for the primary monitor; fall back for others.
            let work_area_bottom = match &primary {
                Some(p) if p.name() == monitor.name() => {
                    get_work_area_bottom(monitor_width, monitor_height, scale_factor)
                }
                _ => {
                    // Non-primary: approximate by subtracting a typical taskbar height.
                    monitor_height as i32 - 40
                }
            };

            let strip_y = work_area_bottom - strip_height as i32;

            StripInfo {
                strip_height,
                monitor_width,
                strip_y,
                monitor_height,
                monitor_x,
                monitor_index: index,
            }
        })
        .collect();

    // Sort by monitor_x ascending so adjacent Vec entries are physically adjacent.
    monitors.sort_by_key(|m| m.monitor_x);

    monitors
}

/// Get the logical Y coordinate of the bottom of the usable work area
/// (i.e., the screen height minus the OS taskbar).
fn get_work_area_bottom(_monitor_width: u32, monitor_height: u32, scale_factor: f64) -> i32 {
    #[cfg(target_os = "macos")]
    {
        let _ = scale_factor;
        get_work_area_bottom_macos().unwrap_or(monitor_height as i32)
    }

    #[cfg(target_os = "windows")]
    {
        // rcWork.bottom is in physical pixels; convert to logical.
        get_work_area_bottom_windows(scale_factor).unwrap_or(monitor_height as i32)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Linux/X11: approximate by subtracting a typical taskbar height (40px).
        let _ = (_monitor_width, scale_factor);
        monitor_height as i32 - 40
    }
}

/// macOS: use NSScreen.mainScreen.visibleFrame to get the usable screen area.
#[cfg(target_os = "macos")]
fn get_work_area_bottom_macos() -> Option<i32> {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_foundation::NSRect;

    unsafe {
        let ns_screen_class = class!(NSScreen);
        // [NSScreen mainScreen]
        let main_screen: *mut AnyObject = msg_send![ns_screen_class, mainScreen];
        if main_screen.is_null() {
            return None;
        }

        // [mainScreen frame] - total screen bounds (NSRect)
        let frame: NSRect = msg_send![main_screen, frame];
        // [mainScreen visibleFrame] - excludes Dock and menu bar (NSRect)
        let visible_frame: NSRect = msg_send![main_screen, visibleFrame];

        // On macOS, coordinate system is bottom-up (0,0 = bottom-left).
        // visibleFrame.origin.y is the height of the Dock (bottom of work area in flipped coords).
        // We want the logical Y of the work area bottom in a top-down coord system:
        //   work_area_bottom = screen_height - visible_frame.origin.y
        let screen_height = frame.size.height as i32;
        let dock_height = visible_frame.origin.y as i32;
        Some(screen_height - dock_height)
    }
}

/// Windows: use GetMonitorInfo to get rcWork (work area excluding taskbar).
#[cfg(target_os = "windows")]
fn get_work_area_bottom_windows(scale_factor: f64) -> Option<i32> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    };
    use windows::Win32::Foundation::POINT;

    unsafe {
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            // rcWork.bottom is in physical pixels; convert to logical.
            Some((info.rcWork.bottom as f64 / scale_factor) as i32)
        } else {
            None
        }
    }
}
