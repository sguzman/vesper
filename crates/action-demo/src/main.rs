//! Separate action game executable. The old visual-novel player is untouched.
//! Run from the repo root: cargo run -p vesper-action-demo -- examples/action-adventure

use std::{env, path::PathBuf};

use bevy::prelude::*;
use vesper_action::{ActionPlugin, DemoRoot};

fn main() {
    let root = PathBuf::from(
        env::args()
            .nth(1)
            .unwrap_or_else(|| "examples/action-adventure".to_string()),
    );
    App::new()
        .insert_resource(DemoRoot(root))
        .insert_resource(ClearColor(Color::srgb(0.035, 0.060, 0.075)))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Vesper — Signal Depot (action prototype)".into(),
                        resolution: (960, 540).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(ActionPlugin)
        .run();
}
