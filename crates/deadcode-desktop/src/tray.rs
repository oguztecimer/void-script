use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

/// Stable ID used to identify the Quit menu item when matching events.
const QUIT_ID: &str = "quit";
/// Stable IDs for care action menu items.
const FEED_ID: &str = "feed";
const CLEAN_ID: &str = "clean";
const PLAY_ID: &str = "play";
const SETTINGS_ID: &str = "settings";
const EDITOR_ID: &str = "editor";
const ABOUT_ID: &str = "about";

/// Returns the stable quit menu item ID string.
pub fn quit_id() -> &'static str {
    QUIT_ID
}

/// Returns the stable feed menu item ID string.
pub fn feed_id() -> &'static str {
    FEED_ID
}

/// Returns the stable clean menu item ID string.
pub fn clean_id() -> &'static str {
    CLEAN_ID
}

/// Returns the stable play menu item ID string.
#[allow(dead_code)]
pub fn play_id() -> &'static str {
    PLAY_ID
}

/// Returns the stable editor menu item ID string.
pub fn editor_id() -> &'static str {
    EDITOR_ID
}

/// Create a simple 22x22 RGBA icon for the system tray.
///
/// Draws a white paw print on a transparent background -- simple and recognisable
/// without requiring an external image asset. Each pixel is [R, G, B, A].
fn make_tray_icon() -> Icon {
    const SIZE: u32 = 22;
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];

    // Helper to paint a filled circle in the pixel buffer.
    let paint_circle = |buf: &mut [u8], cx: f32, cy: f32, r: f32| {
        let r2 = r * r;
        for py in 0..SIZE {
            for px in 0..SIZE {
                let dx = px as f32 - cx;
                let dy = py as f32 - cy;
                if dx * dx + dy * dy <= r2 {
                    let idx = ((py * SIZE + px) * 4) as usize;
                    buf[idx]     = 255; // R
                    buf[idx + 1] = 255; // G
                    buf[idx + 2] = 255; // B
                    buf[idx + 3] = 255; // A
                }
            }
        }
    };

    // Main pad (large circle in lower-centre).
    paint_circle(&mut pixels, 11.0, 15.0, 5.5);
    // Four toe pads arranged above the main pad.
    paint_circle(&mut pixels, 5.5,  9.0, 2.5);
    paint_circle(&mut pixels, 9.5,  6.5, 2.5);
    paint_circle(&mut pixels, 13.5, 6.5, 2.5);
    paint_circle(&mut pixels, 17.5, 9.0, 2.5);

    Icon::from_rgba(pixels, SIZE, SIZE).expect("Failed to create tray icon")
}

/// Create and return a system tray icon with care action menu items and a Quit item.
///
/// The returned `TrayIcon` MUST be kept alive for the duration of the application.
/// Dropping it removes the tray icon from the system tray.
///
/// The `proxy` is used to bridge muda menu events into the winit event loop.
pub fn create_tray(proxy: EventLoopProxy<UserEvent>) -> TrayIcon {
    // Build care action items.
    let feed_item = MenuItem::with_id(FEED_ID, "Feed", true, None);
    let clean_item = MenuItem::with_id(CLEAN_ID, "Clean", true, None);
    let play_item = MenuItem::with_id(PLAY_ID, "Ball", true, None);
    let separator1 = PredefinedMenuItem::separator();
    let editor_item = MenuItem::with_id(EDITOR_ID, "Editor", true, None);
    let separator1b = PredefinedMenuItem::separator();
    // Settings and About are disabled for Phase 3; enabled in a future phase.
    let settings_item = MenuItem::with_id(SETTINGS_ID, "Settings", false, None);
    let about_item = MenuItem::with_id(ABOUT_ID, "About", false, None);
    let separator2 = PredefinedMenuItem::separator();
    let quit_item = MenuItem::with_id(QUIT_ID, "Quit", true, None);

    let menu = Menu::new();
    menu.append(&feed_item).expect("append feed");
    menu.append(&clean_item).expect("append clean");
    menu.append(&play_item).expect("append play");
    menu.append(&separator1).expect("append separator1");
    menu.append(&editor_item).expect("append editor");
    menu.append(&separator1b).expect("append separator1b");
    menu.append(&settings_item).expect("append settings");
    menu.append(&about_item).expect("append about");
    menu.append(&separator2).expect("append separator2");
    menu.append(&quit_item).expect("append quit");

    // Wire the menu event handler to forward events through the winit proxy.
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        // Ignore send errors — the event loop may have already exited.
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    // Build and return the tray icon. An icon image is required on macOS —
    // without it the tray icon is invisible and the menu cannot be opened.
    let icon = make_tray_icon();
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("deadcode")
        .with_menu(Box::new(menu))
        .build()
        .expect("Failed to create system tray icon")
}

/// Create and return a right-click context menu for the dog sprite.
///
/// Uses the same stable item IDs as the tray menu — muda fires a global
/// MenuEvent handler for all menus, so the same id-based matching works.
pub fn create_context_menu() -> Menu {
    let feed_item = MenuItem::with_id(FEED_ID, "Feed", true, None);
    let clean_item = MenuItem::with_id(CLEAN_ID, "Clean", true, None);
    let pet_item = MenuItem::with_id("pet", "Pet", true, None);
    let play_item = MenuItem::with_id(PLAY_ID, "Ball", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit_item = MenuItem::with_id(QUIT_ID, "Quit", true, None);

    let menu = Menu::new();
    menu.append(&feed_item).expect("append feed");
    menu.append(&clean_item).expect("append clean");
    menu.append(&pet_item).expect("append pet");
    menu.append(&play_item).expect("append play");
    menu.append(&separator).expect("append separator");
    menu.append(&quit_item).expect("append quit");
    menu
}
