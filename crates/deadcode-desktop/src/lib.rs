pub mod animation;
pub mod behavior;
pub mod fetch;
pub mod fullscreen;
pub mod renderer;
pub mod save;
pub mod tray;
pub mod window;

/// Custom events sent through the winit event loop from background sources.
#[derive(Debug)]
pub enum UserEvent {
    /// A menu event forwarded from the system tray event handler.
    MenuEvent(tray_icon::menu::MenuEvent),
}
