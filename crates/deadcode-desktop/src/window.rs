use winit::event_loop::ActiveEventLoop;

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
    /// Height of the OS dock/taskbar in logical pixels.
    pub dock_height: u32,
    /// DPI scale factor for this monitor (1.0, 1.25, 1.5, 2.0, etc.)
    pub scale_factor: f64,
    /// Height of the OS dock/taskbar in physical pixels.
    pub phys_dock_height: u32,
    /// Physical pixel values for window positioning (avoids DPI mismatch on multi-monitor).
    pub phys_width: u32,
    pub phys_height: u32,
    pub phys_x: i32,
    pub phys_y: i32,
}

/// Enumerate all connected monitors and return a `Vec<StripInfo>` sorted by `monitor_x`
/// (left-to-right order).
///
/// This is used by Plan 03 (multi-monitor roaming) to discover adjacent monitors so the
/// dog can walk off the edge of one screen and appear on the next.
///
#[allow(dead_code)]
pub fn enumerate_monitors(event_loop: &ActiveEventLoop) -> Vec<StripInfo> {
    let strip_height: u32 = 316;

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

            let dock_height = get_dock_height(monitor_height, scale_factor);
            let strip_y = monitor_height as i32 - strip_height as i32;

            // Physical values for window positioning (avoids DPI mismatch)
            let phys_strip_height = (strip_height as f64 * scale_factor) as u32;
            let phys_y = monitor_size.height as i32 - phys_strip_height as i32 + monitor_pos.y;

            StripInfo {
                strip_height,
                monitor_width,
                strip_y,
                monitor_height,
                monitor_x,
                monitor_index: index,
                dock_height,
                scale_factor,
                phys_dock_height: (dock_height as f64 * scale_factor) as u32,
                phys_width: monitor_size.width,
                phys_height: phys_strip_height,
                phys_x: monitor_pos.x,
                phys_y,
            }
        })
        .collect();

    // Sort by monitor_x ascending so adjacent Vec entries are physically adjacent.
    monitors.sort_by_key(|m| m.monitor_x);

    monitors
}

/// Get the height of the OS dock/taskbar in logical pixels.
fn get_dock_height(monitor_height: u32, _scale_factor: f64) -> u32 {
    #[cfg(target_os = "macos")]
    {
        let _ = (monitor_height, _scale_factor);
        get_dock_height_macos().unwrap_or(0)
    }

    #[cfg(target_os = "windows")]
    {
        get_dock_height_windows(monitor_height, _scale_factor).unwrap_or(40)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (monitor_height, _scale_factor);
        40
    }
}

#[cfg(target_os = "macos")]
fn get_dock_height_macos() -> Option<u32> {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_foundation::NSRect;

    unsafe {
        let ns_screen_class = class!(NSScreen);
        let main_screen: *mut AnyObject = msg_send![ns_screen_class, mainScreen];
        if main_screen.is_null() {
            return None;
        }

        let visible_frame: NSRect = msg_send![main_screen, visibleFrame];
        // visibleFrame.origin.y is the dock height (macOS coords are bottom-up).
        Some(visible_frame.origin.y as u32)
    }
}

#[cfg(target_os = "windows")]
fn get_dock_height_windows(_monitor_height: u32, scale_factor: f64) -> Option<u32> {
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
            // Taskbar height = difference between full monitor bottom and work area bottom
            let dock_physical = (info.rcMonitor.bottom - info.rcWork.bottom).max(0);
            let dock = (dock_physical as f64 / scale_factor) as u32;
            Some(dock)
        } else {
            None
        }
    }
}
