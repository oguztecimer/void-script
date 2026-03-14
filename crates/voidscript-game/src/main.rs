use bevy::prelude::*;
use voidscript_editor::plugin::EditorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "VOID//SCRIPT".to_string(),
                resolution: bevy::window::WindowResolution::new(1280.0, 720.0),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EditorPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, open_editor_on_key)
        .run();
}

fn setup(
    mut commands: Commands,
    mut open_events: EventWriter<voidscript_editor::window::OpenEditorEvent>,
) {
    commands.spawn(Camera2d);
    open_events.send(voidscript_editor::window::OpenEditorEvent);
    info!("VOID//SCRIPT initialized. Editor opening...");
}

fn open_editor_on_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut open_events: EventWriter<voidscript_editor::window::OpenEditorEvent>,
) {
    if keyboard.just_pressed(KeyCode::KeyE) {
        open_events.send(voidscript_editor::window::OpenEditorEvent);
    }
}
