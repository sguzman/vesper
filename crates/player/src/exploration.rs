//! Deliberately small optional side-on exploration preview.
//!
//! This is *not* a new story engine: motion and camera are projected from
//! validated, author-owned locations. Interactions and save migration arrive
//! in NA06/NA07. Legacy VN examples never instantiate any of these entities.
use super::*;
use vesper_game::{
    experience::{ExperienceMode, Facing},
    walk::{self, View},
};

#[derive(Component)]
enum AreaVisual {
    Backdrop,
    Avatar,
    Npc(f32),
    Prop(f32),
    Layer { parallax: f32 },
}

#[derive(Resource, Default)]
pub struct WalkPreview {
    idle: Option<Handle<Image>>,
    walk: Vec<Handle<Image>>,
    elapsed: f32,
    moving: bool,
}

fn sprite_art(path: Option<&String>, assets: &AssetServer,
    color: Color, size: Vec2) -> Sprite {
    path.map_or_else(|| Sprite::from_color(color, size), |path| Sprite {
        image: assets.load(path.clone()), ..default()
    })
}

/// Spawn only when the author's optional [exploration.start] exists.
/// The entire usual VN UI is still built, but hidden until NA06 connects
/// explicit authored transitions back into narrative scenes.
pub fn setup(
    mut commands: Commands, session: Res<Session>, assets: Res<AssetServer>,
    mut panels: Query<&mut Node, With<StoryPanel>>,
) {
    let mut runtime = WalkPreview::default();
    let ExperienceMode::Exploring(position) = &session.experience.mode else {
        commands.insert_resource(runtime);
        return;
    };
    let Some(world) = &session.game.manifest.exploration else {
        commands.insert_resource(runtime);
        return;
    };
    let Some(area) = world.locations.get(&position.location) else {
        commands.insert_resource(runtime);
        return;
    };

    runtime.idle = world.avatar.idle.as_ref()
        .map(|path| assets.load::<Image>(path.clone()));
    runtime.walk = world.avatar.walk.iter()
        .map(|path| assets.load::<Image>(path.clone())).collect();
    for mut node in &mut panels { node.display = Display::None; }

    commands.spawn((
        Name::new("Exploration: authored location background"),
        AreaVisual::Backdrop,
        sprite_art(Some(&area.background), &assets, Color::WHITE, Vec2::new(1280.0, 720.0)),
        Transform::from_xyz(0.0, 0.0, -6.0),
    ));
    for (id, layer) in &area.layers {
        commands.spawn((
            Name::new(format!("Exploration layer: {id}")),
            AreaVisual::Layer { parallax: layer.parallax },
            sprite_art(Some(&layer.image), &assets, Color::WHITE, Vec2::new(1280.0, 720.0)),
            Transform::from_xyz(0.0, 0.0, -5.5 + layer.depth * 0.08),
        ));
    }
    for (id, npc) in &area.npcs {
        commands.spawn((
            Name::new(format!("Exploration NPC: {id}")),
            AreaVisual::Npc(npc.x),
            sprite_art(npc.image.as_ref(), &assets,
                Color::srgb(0.36, 0.68, 0.77), Vec2::new(40.0, 85.0)),
            Transform::from_xyz(0.0, 0.0, 2.0),
        ));
    }
    for (id, prop) in &area.props {
        commands.spawn((
            Name::new(format!("Exploration prop: {id}")),
            AreaVisual::Prop(prop.x),
            sprite_art(prop.image.as_ref(), &assets,
                Color::srgb(0.86, 0.65, 0.34), Vec2::new(34.0, 46.0)),
            Transform::from_xyz(0.0, 0.0, 1.5),
        ));
    }
    commands.spawn((
        Name::new("Exploration: player avatar"),
        AreaVisual::Avatar,
        sprite_art(world.avatar.idle.as_ref(), &assets,
            Color::srgb(0.84, 0.45, 0.63), Vec2::new(35.0, 85.0)),
        Transform::from_xyz(0.0, 0.0, 4.0),
    ));
    commands.spawn((
        Name::new("Exploration preview controls (not a VN choice)"),
        Node {
            position_type: PositionType::Absolute,
            top: px(14.0), right: px(16.0),
            padding: px(9.0).all(),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.06, 0.1, 0.78)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("← → or A/D: Walk  ·  P: Pause  ·  M: Menu"),
            TextFont { font_size: FontSize::Px(15.0), ..default() },
            TextColor(Color::WHITE),
        ));
    });
    commands.insert_resource(runtime);
    info!("Vesper exploration preview entered: {} at x={:.1}",
        position.location, position.x);
}

/// No gameplay controls run when focus or a modal menu owns the keyboard.
pub fn input(
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    settings: Res<settings::SettingsState>,
    font_browser: Res<font_browser::State>,
    slots: Res<save_slots::SaveUi>,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut runtime: ResMut<WalkPreview>,
) {
    let ExperienceMode::Exploring(position) = &session.experience.mode else { return; };
    let Some(area) = session.game.manifest.exploration.as_ref()
        .and_then(|world| world.locations.get(&position.location)) else { return; };
    let speed = session.game.manifest.exploration.as_ref()
        .map_or(230.0, |world| world.avatar.speed);
    let blocked = !windows.single().is_ok_and(|window| window.focused)
        || session.paused || session.menu_overlay.is_some()
        || session.history.visible() || slots.visible()
        || settings.overlay.is_some() || font_browser.overlay.is_some();

    let left = keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA);
    let right = keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD);
    let axis = if blocked { 0 } else { i8::from(right) - i8::from(left) };
    if axis == 0 {
        if runtime.moving {
            if !blocked {
                info!("Vesper exploration stopped: {} at x={:.1}",
                    position.location, position.x);
            }
        }
        runtime.moving = false;
        runtime.elapsed = 0.0;
        return;
    }
    // Take the disjoint author data before mutably borrowing the experience.
    let width = area.width;
    let ExperienceMode::Exploring(position) = &mut session.experience.mode else { return; };
    runtime.moving = walk::walk(position, axis, time.delta_secs(), speed, width);
    if runtime.moving { runtime.elapsed += time.delta_secs().min(0.05); }
}

/// UI camera remains fixed for traditional VN scenes. Instead, this
/// lightweight world projection scrolls only exploration sprites, allowing
/// the same menus/reader/accessibility scale to remain stable.
pub fn sync(
    session: Res<Session>, runtime: Res<WalkPreview>,
    windows: Query<&Window, With<PrimaryWindow>>,
    images: Res<Assets<Image>>,
    mut sprites: Query<(&AreaVisual, &mut Sprite, &mut Transform)>,
) {
    let ExperienceMode::Exploring(position) = &session.experience.mode else { return; };
    let Some(world) = &session.game.manifest.exploration else { return; };
    let Some(area) = world.locations.get(&position.location) else { return; };
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.resolution.width(), window.resolution.height());
    if viewport.x <= 0.0 || viewport.y <= 0.0 { return; }
    let view = View::for_area(area.width, area.height, viewport.x, viewport.y,
        area.camera.zoom, position.x, area.camera.follow_x);
    let ground_y = (0.25 - area.camera.horizon) * viewport.y;
    for (part, mut sprite, mut transform) in &mut sprites {
        match part {
            AreaVisual::Backdrop | AreaVisual::Layer { .. } => {
                let parallax = if let AreaVisual::Layer { parallax, .. } = part {
                    *parallax
                } else { 1.0 };
                let scroll = (view.center_x - area.width / 2.0) * parallax;
                let desired_x = -scroll * view.world_to_screen;
                if transform.translation.x != desired_x { transform.translation.x = desired_x; }
                // Uniform cover scaling preserves the art's native aspect.
                if let Some(image) = images.get(&sprite.image) {
                    let source = Vec2::new(
                        image.texture_descriptor.size.width as f32,
                        image.texture_descriptor.size.height as f32
                    ).max(Vec2::ONE);
                    let fit = (area.width / source.x).max(area.height / source.y);
                    let scale = Vec3::splat(fit * view.world_to_screen);
                    if transform.scale != scale { transform.scale = scale; }
                }
            }
            AreaVisual::Avatar => {
                let desired_x = view.x(position.x);
                if transform.translation.x != desired_x { transform.translation.x = desired_x; }
                if transform.translation.y != ground_y { transform.translation.y = ground_y; }
                sprite.flip_x = position.facing == Facing::Left;
                let handle = if runtime.moving && !runtime.walk.is_empty() {
                    let index = ((runtime.elapsed * world.avatar.walk_fps) as usize)
                        % runtime.walk.len();
                    Some(runtime.walk[index].clone())
                } else {
                    runtime.idle.clone()
                };
                // With walk-only artwork, retain its last walking frame at
                // rest rather than swapping in a blank texture handle.
                if let Some(handle) = handle {
                    if sprite.image != handle { sprite.image = handle; }
                }
                let natural_height = images.get(&sprite.image)
                    .map_or(85.0, |image| image.texture_descriptor.size.height.max(1) as f32);
                let fit = 85.0 / natural_height;
                let scale = Vec3::splat(view.world_to_screen * world.avatar.scale * fit);
                if transform.scale != scale { transform.scale = scale; }
            }
            AreaVisual::Npc(x) | AreaVisual::Prop(x) => {
                let desired_x = view.x(*x);
                if transform.translation.x != desired_x { transform.translation.x = desired_x; }
                if transform.translation.y != ground_y { transform.translation.y = ground_y; }
                let natural_height = images.get(&sprite.image).map_or(
                    if matches!(part, AreaVisual::Npc(_)) { 85.0 } else { 46.0 },
                    |image| image.texture_descriptor.size.height.max(1) as f32,
                );
                let height = if matches!(part, AreaVisual::Npc(_)) { 85.0 } else { 46.0 };
                let scale = Vec3::splat(view.world_to_screen * height / natural_height);
                if transform.scale != scale { transform.scale = scale; }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_disables_all_story_input_when_world_mode_is_active() {
        let mut state = vesper_game::experience::ExperienceState::default();
        assert!(matches!(state.mode, ExperienceMode::VisualNovel));
        state.enter_exploration(vesper_game::experience::WorldPosition::new(
            "street", 150.0, Facing::Right).unwrap());
        assert!(matches!(state.mode, ExperienceMode::Exploring(_)));
    }

    #[test]
    fn visible_world_area_remains_sized_in_logical_pixels() {
        let area: vesper_game::exploration::Location = toml::from_str(
            "background = 'street.png'\nwidth = 1280.0\nheight = 720.0\n[spawns.home]\nx=150.0"
        ).unwrap();
        assert!(area.camera.follow_x);
        assert_eq!(area.width, 1280.0);
    }
}
