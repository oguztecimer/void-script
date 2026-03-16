/// Returns `true` if any application is currently running in fullscreen mode.
///
/// Detection is per-platform:
/// - macOS: Uses `NSApplication.currentSystemPresentationOptions()` to detect system-wide
///   fullscreen state — no Screen Recording permission required.
/// - Windows: Uses `SHQueryUserNotificationState` to detect D3D fullscreen and presentation mode.
/// - Linux: Queries `_NET_ACTIVE_WINDOW` then `_NET_WM_STATE_FULLSCREEN` via x11rb.
///
/// This function is fail-open: any error returns `false` so the pet stays visible.
/// It must be non-blocking — no OS calls that can stall the winit event loop.
pub fn is_any_fullscreen() -> bool {
    is_any_fullscreen_impl()
}

// -- macOS --

#[cfg(target_os = "macos")]
fn is_any_fullscreen_impl() -> bool {
    use objc2_app_kit::{NSApplication, NSApplicationPresentationOptions};
    use objc2_foundation::MainThreadMarker;

    // MainThreadMarker::new() returns None if not on the main thread.
    // On macOS winit always calls about_to_wait (and therefore this function)
    // on the main thread, so this should always succeed. Fail-open on the
    // off-chance it does not.
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };

    let app = NSApplication::sharedApplication(mtm);
    let opts = app.currentSystemPresentationOptions();
    opts.contains(NSApplicationPresentationOptions::FullScreen)
}

// -- Windows --

#[cfg(target_os = "windows")]
fn is_any_fullscreen_impl() -> bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    };

    // SHQueryUserNotificationState reports whether any exclusive fullscreen or
    // presentation-mode application is active. This covers both DirectX exclusive
    // fullscreen games and Windows presentation mode.
    let mut state = windows::Win32::UI::Shell::QUERY_USER_NOTIFICATION_STATE(0);
    match unsafe { SHQueryUserNotificationState(&mut state) } {
        Ok(_) => state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE,
        // Fail-open: if the query fails, don't hide the pet.
        Err(_) => false,
    }
}

// -- Linux --

#[cfg(target_os = "linux")]
fn is_any_fullscreen_impl() -> bool {
    // Wrap in inner function returning Option<bool> for clean ? operator usage.
    fn check() -> Option<bool> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{self, ConnectionExt};

        let (conn, screen_num) = x11rb::connect(None).ok()?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        // Intern atoms needed for the query.
        let net_active_window = conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let net_wm_state = conn
            .intern_atom(false, b"_NET_WM_STATE")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let net_wm_state_fullscreen = conn
            .intern_atom(false, b"_NET_WM_STATE_FULLSCREEN")
            .ok()?
            .reply()
            .ok()?
            .atom;

        // Get the active window from the root window property.
        let prop = conn
            .get_property(false, root, net_active_window, xproto::AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?;
        let active_win = prop.value32()?.next()?;

        // Check whether _NET_WM_STATE contains the fullscreen flag.
        let state = conn
            .get_property(false, active_win, net_wm_state, xproto::AtomEnum::ATOM, 0, 64)
            .ok()?
            .reply()
            .ok()?;
        let atoms: Vec<u32> = state.value32()?.collect();
        Some(atoms.contains(&net_wm_state_fullscreen))
    }

    // Fail-open: if anything fails (no DISPLAY, unsupported WM, etc.) return false
    // so the pet stays visible rather than hiding for no reason.
    check().unwrap_or(false)
}

// -- Fallback for other platforms --

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn is_any_fullscreen_impl() -> bool {
    false
}
