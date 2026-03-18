use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

const QUIT_ID: &str = "quit";
const EDITOR_ID: &str = "editor";
const SETTINGS_ID: &str = "settings";
const ABOUT_ID: &str = "about";

pub fn quit_id() -> &'static str {
    QUIT_ID
}

pub fn editor_id() -> &'static str {
    EDITOR_ID
}

/// Create a simple 22x22 RGBA icon for the system tray.
fn make_tray_icon() -> Icon {
    const SIZE: u32 = 22;
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];

    let paint_circle = |buf: &mut [u8], cx: f32, cy: f32, r: f32| {
        let r2 = r * r;
        for py in 0..SIZE {
            for px in 0..SIZE {
                let dx = px as f32 - cx;
                let dy = py as f32 - cy;
                if dx * dx + dy * dy <= r2 {
                    let idx = ((py * SIZE + px) * 4) as usize;
                    buf[idx]     = 255;
                    buf[idx + 1] = 255;
                    buf[idx + 2] = 255;
                    buf[idx + 3] = 255;
                }
            }
        }
    };

    paint_circle(&mut pixels, 11.0, 15.0, 5.5);
    paint_circle(&mut pixels, 5.5,  9.0, 2.5);
    paint_circle(&mut pixels, 9.5,  6.5, 2.5);
    paint_circle(&mut pixels, 13.5, 6.5, 2.5);
    paint_circle(&mut pixels, 17.5, 9.0, 2.5);

    Icon::from_rgba(pixels, SIZE, SIZE).expect("Failed to create tray icon")
}

/// Create and return a system tray icon with menu items.
pub fn create_tray(proxy: EventLoopProxy<UserEvent>) -> TrayIcon {
    let editor_item = MenuItem::with_id(EDITOR_ID, "Editor", true, None);
    let separator1 = PredefinedMenuItem::separator();
    let settings_item = MenuItem::with_id(SETTINGS_ID, "Settings", false, None);
    let about_item = MenuItem::with_id(ABOUT_ID, "About", false, None);
    let separator2 = PredefinedMenuItem::separator();
    let quit_item = MenuItem::with_id(QUIT_ID, "Quit", true, None);

    let menu = Menu::new();
    menu.append(&editor_item).expect("append editor");
    menu.append(&separator1).expect("append separator1");
    menu.append(&settings_item).expect("append settings");
    menu.append(&about_item).expect("append about");
    menu.append(&separator2).expect("append separator2");
    menu.append(&quit_item).expect("append quit");

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let icon = make_tray_icon();
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("Dead Code")
        .with_menu(Box::new(menu))
        .build()
        .expect("Failed to create system tray icon")
}

/// Create and return a right-click context menu.
pub fn create_context_menu() -> Menu {
    let editor_item = MenuItem::with_id(EDITOR_ID, "Editor", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit_item = MenuItem::with_id(QUIT_ID, "Quit", true, None);

    let menu = Menu::new();
    menu.append(&editor_item).expect("append editor");
    menu.append(&separator).expect("append separator");
    menu.append(&quit_item).expect("append quit");
    menu
}
