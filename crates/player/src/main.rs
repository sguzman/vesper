//! Bevy 0.20 presentation foundation; story content stays outside Rust.
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new("Vesper — player foundation"),
        TextFont::from_font_size(px(28)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(32),
            left: px(32),
            ..default()
        },
    ));
}
