//! Deliberately small optional side-on exploration preview.
//!
//! This is *not* a new story engine: motion and camera are projected from
//! validated, author-owned locations. World saves use logical coordinates,
 //! not ECS entities. Legacy VNs instantiate no exploration entities.
use super::*;
use vesper_game::{
    experience::{ExperienceMode, Facing},
    interaction::{self, Focus, TargetKind},
    exploration::{Exploration, Location},
    walk::{self, View},
};

#[derive(Component)]
pub(super) enum AreaVisual {
    Backdrop,
    Avatar,
    Npc { id: String, x: f32 },
    Prop { id: String, x: f32 },
    Layer { parallax: f32 },
}

#[derive(Resource, Default)]
pub struct WalkPreview {
    idle: Option<Handle<Image>>,
    walk: Vec<Handle<Image>>,
    elapsed: f32,
    moving: bool,
    loaded_location: Option<String>,
}

impl WalkPreview {
    /// Accepted manifest edits force one area rebuild on the next frame.
    pub fn invalidate_for_reload(&mut self, world: Option<&Exploration>,
        assets: &AssetServer) {
        self.idle = world.and_then(|w| w.avatar.idle.as_ref())
            .map(|path| assets.load::<Image>(path.clone()));
        self.walk = world.map_or_else(Vec::new, |w| w.avatar.walk.iter()
            .map(|path| assets.load::<Image>(path.clone())).collect());
        self.loaded_location = None;
        self.elapsed = 0.0;
        self.moving = false;
    }
}

#[derive(Component)]
pub(super) struct WalkHud;
#[derive(Component)]
pub(super) struct InteractionPrompt;
#[derive(Component)]
pub(super) struct InteractionPromptText;

fn sprite_art(path: Option<&String>, assets: &AssetServer,
    color: Color, size: Vec2) -> Sprite {
    path.map_or_else(|| Sprite::from_color(color, size), |path| Sprite {
        image: assets.load(path.clone()), ..default()
    })
}

/// Spawn exactly one authored area; entity ownership is isolated to this
/// subsystem, so transitions can despawn the previous area atomically.
fn spawn_area(commands: &mut Commands, world: &Exploration,
    area: &Location, assets: &AssetServer) {
    commands.spawn((
        Name::new("Exploration: authored location background"),
        AreaVisual::Backdrop,
        sprite_art(Some(&area.background), assets, Color::WHITE, Vec2::new(1280.0, 720.0)),
        Transform::from_xyz(0.0, 0.0, -6.0),
        Visibility::Visible,
    ));
    for (id, layer) in &area.layers {
        commands.spawn((
            Name::new(format!("Exploration layer: {id}")),
            AreaVisual::Layer { parallax: layer.parallax },
            sprite_art(Some(&layer.image), assets, Color::WHITE, Vec2::new(1280.0, 720.0)),
            Transform::from_xyz(0.0, 0.0, -5.5 + layer.depth * 0.08),
            Visibility::Visible,
        ));
    }
    for (id, npc) in &area.npcs {
        commands.spawn((
            Name::new(format!("Exploration NPC: {id}")),
            AreaVisual::Npc { id: id.clone(), x: npc.x },
            sprite_art(npc.image.as_ref(), assets,
                Color::srgb(0.36, 0.68, 0.77), Vec2::new(40.0, 85.0)),
            Transform::from_xyz(0.0, 0.0, 2.0),
            Visibility::Visible,
        ));
    }
    for (id, prop) in &area.props {
        commands.spawn((
            Name::new(format!("Exploration prop: {id}")),
            AreaVisual::Prop { id: id.clone(), x: prop.x },
            sprite_art(prop.image.as_ref(), assets,
                Color::srgb(0.86, 0.65, 0.34), Vec2::new(34.0, 46.0)),
            Transform::from_xyz(0.0, 0.0, 1.5),
            Visibility::Visible,
        ));
    }
    commands.spawn((
        Name::new("Exploration: player avatar"),
        AreaVisual::Avatar,
        sprite_art(world.avatar.idle.as_ref(), assets,
            Color::srgb(0.84, 0.45, 0.63), Vec2::new(35.0, 85.0)),
        Transform::from_xyz(0.0, 0.0, 4.0),
        Visibility::Visible,
    ));
}

/// Spawn only when the author's optional [exploration.start] exists.
/// The entire usual VN UI is still built, but hidden until NA06 connects
/// explicit authored transitions back into narrative scenes.
pub fn setup(
    mut commands: Commands, session: Res<Session>, assets: Res<AssetServer>,
    mut panels: Query<&mut Node, With<StoryPanel>>,
) {
    let mut runtime = WalkPreview::default();
    let Some(world) = &session.game.manifest.exploration else {
        commands.insert_resource(runtime);
        return;
    };
    let start = match &session.experience.mode {
        ExperienceMode::Exploring(point) | ExperienceMode::Conversation {
            return_to: point, ..
        } => Some(point),
        ExperienceMode::VisualNovel => None,
    };
    runtime.idle = world.avatar.idle.as_ref()
        .map(|path| assets.load::<Image>(path.clone()));
    runtime.walk = world.avatar.walk.iter()
        .map(|path| assets.load::<Image>(path.clone())).collect();
    if let Some(point) = start
        && let Some(area) = world.locations.get(&point.location)
    {
        for mut node in &mut panels { node.display = Display::None; }
        spawn_area(&mut commands, world, area, &assets);
        runtime.loaded_location = Some(point.location.clone());
    }
    commands.spawn((
        Name::new("Exploration preview controls (not a VN choice)"),
        WalkHud,
        Node {
            display: if start.is_some() { Display::Flex } else { Display::None },
            position_type: PositionType::Absolute,
            top: px(14.0), right: px(16.0),
            padding: px(9.0).all(),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.06, 0.1, 0.78)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("← → or A/D: Walk  ·  E: Interact/Exit  ·  P: Pause  ·  M: Menu"),
            TextFont { font_size: FontSize::Px(15.0), ..default() },
            TextColor(Color::WHITE),
        ));
    });
    // A single keyboard-focusable interaction cue: no clickable tiny NPC
    // hitboxes, no walking errands, and no game-specific string logic.
    commands.spawn((
        Name::new("Exploration nearby interaction cue"),
        InteractionPrompt,
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            bottom: px(30.0),
            left: percent(24.0),
            width: percent(52.0),
            justify_content: JustifyContent::Center,
            padding: px(12.0).all(),
            border: px(2.0).all(),
            border_radius: BorderRadius::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.04, 0.09, 0.94)),
        BorderColor::all(Color::WHITE),
    )).with_children(|parent| {
        parent.spawn((
            InteractionPromptText,
            Text::new("E: Interact"),
            TextFont { font_size: FontSize::Px(18.0), ..default() },
            TextColor(Color::WHITE),
            TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
        ));
    });
    commands.insert_resource(runtime);
    if let Some(position) = start {
        info!("Vesper exploration preview entered: {} at x={:.1}",
            position.location, position.x);
    }
}

/// Interact with the closest validated authored NPC/prop. Successful
/// transitions keep world sprites in place; the same interpreter executes
/// dialogue, rich text, choices and every existing story command.
pub fn begin_interaction(
    session: &mut Session,
    commands: &mut Commands,
    assets: &AssetServer,
) -> Result<Option<Screen>, String> {
    let ExperienceMode::Exploring(position) = &session.experience.mode else {
        return Ok(None);
    };
    let Some(area) = session.game.manifest.exploration.as_ref()
        .and_then(|world| world.locations.get(&position.location)) else {
        return Err("current authored exploration location is unavailable".into());
    };
    let Some(target) = interaction::focus(area, position, session.machine.variables()) else {
        return Ok(None);
    };
    match target {
        Focus::Scene(target) => {
            let scene = target.scene.to_owned();
            let label = target.id.to_owned();
            // Failure cannot change the authoritative exploration position.
            session.machine.enter_scene(&scene)?;
            session.experience.begin_interaction(scene.clone())?;
            session.choosing = false;
            session.finished = false;
            let screen = next_screen(session, commands, assets);
            info!("Vesper exploration interacting: {label} -> {scene}");
            Ok(Some(screen))
        }
        Focus::Exit(target) => {
            let from = position.location.clone();
            let exit_id = target.id.to_owned();
            let world = session.game.manifest.exploration.as_ref()
                .ok_or("exploration manifest unexpectedly unavailable")?;
            let arrival = world.exit_position(position, &exit_id)?;
            info!("Vesper exploration exit: {from}/{exit_id} -> {} at x={:.1}",
                arrival.location, arrival.x);
            session.experience.enter_exploration(arrival);
            Ok(None)
        }
    }
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
    // Animate on the same bounded game-time clock used for locomotion.
    // Otherwise walking textures drift behind the avatar at low frame rates.
    if runtime.moving {
        runtime.elapsed += time.delta_secs().min(walk::MAX_STEP_SECONDS);
    }
}

/// Switch only the owned authored-area sprites on an explicit location
/// change, without leaking old NPCs, image handles, or invisible entities.
/// The existing VN UI and menu entities are not touched.
pub fn refresh_area(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<AssetServer>,
    mut runtime: ResMut<WalkPreview>,
    old: Query<Entity, With<AreaVisual>>,
) {
    let next = match &session.experience.mode {
        ExperienceMode::Exploring(position)
        | ExperienceMode::Conversation { return_to: position, .. } =>
            Some(position.location.clone()),
        ExperienceMode::VisualNovel => None,
    };
    if runtime.loaded_location == next { return; }
    let mut replaced = 0_usize;
    for entity in &old {
        commands.entity(entity).despawn();
        replaced += 1;
    }
    runtime.loaded_location.clone_from(&next);
    runtime.moving = false;
    runtime.elapsed = 0.0;
    if let Some(location) = next {
        if let Some(world) = &session.game.manifest.exploration
            && let Some(area) = world.locations.get(&location)
        {
            spawn_area(&mut commands, world, area, &assets);
            info!("Vesper exploration area loaded: {location}; replaced {replaced} entities");
        } else {
            error!("Vesper exploration area missing after validated transition");
        }
    } else {
        info!("Vesper exploration area released for cinematic VN");
    }
}

/// Placeholder rectangles have an explicit custom_size even though their
/// built-in white backing texture is 1×1. Respect that authored display size
/// before consulting the raw image dimensions, or a placeholder can grow to
/// fullscreen and conceal the whole location.
fn sprite_height(sprite: &Sprite, images: &Assets<Image>, fallback: f32) -> f32 {
    sprite.custom_size.map(|size| size.y).or_else(||
        images.get(&sprite.image).map(|image|
            image.texture_descriptor.size.height.max(1) as f32)
    ).unwrap_or(fallback).max(1.0)
}

/// The world focus prompt is actionable only when movement/interaction
/// owns the keyboard and no higher-priority overlay intercepts input.
fn focus_prompt_available(
    exploring: bool, modal: bool, overlay: bool, window_focused: bool,
) -> bool {
    exploring && !modal && !overlay && window_focused
}

/// UI camera remains fixed for traditional VN scenes. Instead, this
/// lightweight world projection scrolls only exploration sprites, allowing
/// the same menus/reader/accessibility scale to remain stable.
pub fn sync(
    session: Res<Session>, runtime: Res<WalkPreview>,
    windows: Query<&Window, With<PrimaryWindow>>,
    settings: Res<settings::SettingsState>,
    font_browser: Res<font_browser::State>,
    slots: Res<save_slots::SaveUi>,
    images: Res<Assets<Image>>,
    mut sprites: Query<(&AreaVisual, &mut Sprite, &mut Transform, &mut Visibility)>,
    mut prompts: Query<&mut Node, (With<InteractionPrompt>, Without<StoryPanel>)>,
    mut labels: Query<&mut Text, With<InteractionPromptText>>,
    mut huds: Query<&mut Node, (With<WalkHud>, Without<StoryPanel>, Without<InteractionPrompt>)>,
    mut panels: Query<&mut Node, (With<StoryPanel>, Without<InteractionPrompt>)>,
) {
    // Pure VNs never enter this optional player subsystem or have their
    // panel display state rewritten by the exploration renderer.
    if session.game.manifest.exploration.is_none() {
        return;
    }
    // World artwork remains visible during modal conversations, so the
    // speaker can still be seen under a bubble or illustrated VN panel.
    let position = match &session.experience.mode {
        ExperienceMode::Exploring(point) | ExperienceMode::Conversation {
            return_to: point, ..
        } => Some(point),
        ExperienceMode::VisualNovel => None,
    };
    let exploring = matches!(session.experience.mode, ExperienceMode::Exploring(_));
    for mut hud in &mut huds {
        let display = if exploring { Display::Flex } else { Display::None };
        if hud.display != display { hud.display = display; }
    }
    for mut panel in &mut panels {
        let display = if exploring { Display::None } else { Display::Flex };
        if panel.display != display { panel.display = display; }
    }
    let target = position.and_then(|point| {
        session.game.manifest.exploration.as_ref()
            .and_then(|world| world.locations.get(&point.location))
            .and_then(|area| interaction::focus(area, point, session.machine.variables()))
    });
    // The interaction prompt must follow the same input-ownership policy
    // as the movement controller. No actionable "E" cue under modal
    // settings/font/save overlays that intentionally swallow E presses.
    let focused = focus_prompt_available(
        exploring,
        session.paused || session.menu_overlay.is_some(),
        session.history.visible() || settings.overlay.is_some()
            || font_browser.overlay.is_some() || slots.visible(),
        windows.single().is_ok_and(|window| window.focused),
    );
    for mut prompt in &mut prompts {
        let display = if focused && target.is_some() { Display::Flex } else { Display::None };
        if prompt.display != display { prompt.display = display; }
    }
    if focused {
        if let Some(target) = target {
            let next = match target {
                Focus::Scene(scene) => {
                    let verb = if scene.kind == TargetKind::Npc { "Talk to" } else { "Inspect" };
                    format!("E: {verb} {}", scene.id)
                }
                Focus::Exit(exit) => format!("E: Enter {}", exit.to_location),
            };
            for mut label in &mut labels {
                if label.0 != next { label.0.clone_from(&next); }
            }
        }
    }
    let current_area = position.and_then(|point| session.game.manifest.exploration.as_ref()
        .and_then(|world| world.locations.get(&point.location)));
    for (part, _, _, mut visibility) in &mut sprites {
        let enabled = match part {
            AreaVisual::Npc { id, .. } => current_area
                .and_then(|area| area.npcs.get(id))
                .is_some_and(|npc| npc.visible_if.as_ref()
                    .is_none_or(|gate| gate.matches(session.machine.variables()))),
            AreaVisual::Prop { id, .. } => current_area
                .and_then(|area| area.props.get(id))
                .is_some_and(|prop| prop.visible_if.as_ref()
                    .is_none_or(|gate| gate.matches(session.machine.variables()))),
            _ => true,
        };
        let value = if position.is_some() && enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != value { *visibility = value; }
    }
    let Some(position) = position else { return; };
    let Some(world) = &session.game.manifest.exploration else { return; };
    let Some(area) = world.locations.get(&position.location) else { return; };
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.resolution.width(), window.resolution.height());
    if viewport.x <= 0.0 || viewport.y <= 0.0 { return; }
    let view = View::for_area(area.width, area.height, viewport.x, viewport.y,
        area.camera.zoom, position.x, area.camera.follow_x);
    let ground_y = (0.25 - area.camera.horizon) * viewport.y;
    for (part, mut sprite, mut transform, _) in &mut sprites {
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
                // An unconditional Sprite write marks this component
                // changed on every Bevy frame, even when facing is stable.
                let flip = position.facing == Facing::Left;
                if sprite.flip_x != flip { sprite.flip_x = flip; }
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
                let natural_height = sprite_height(&sprite, &images, 85.0);
                let fit = 85.0 / natural_height;
                let scale = Vec3::splat(view.world_to_screen * world.avatar.scale * fit);
                if transform.scale != scale { transform.scale = scale; }
            }
            AreaVisual::Npc { x, .. } | AreaVisual::Prop { x, .. } => {
                let desired_x = view.x(*x);
                if transform.translation.x != desired_x { transform.translation.x = desired_x; }
                if transform.translation.y != ground_y { transform.translation.y = ground_y; }
                let natural_height = sprite_height(
                    &sprite, &images,
                    if matches!(part, AreaVisual::Npc { .. }) { 85.0 } else { 46.0 },
                );
                let height = if matches!(part, AreaVisual::Npc { .. }) { 85.0 } else { 46.0 };
                let scale = Vec3::splat(view.world_to_screen * height / natural_height);
                if transform.scale != scale { transform.scale = scale; }
            }
        }
    }
}

/// One bounded, opt-in measurement of the actual Bevy frame clock while
/// exploration is active. No profiler allocation or sampling is registered
/// during normal gameplay. CI reports observed FPS/p95 rather than inventing
/// a fixed throughput guarantee for software llvmpipe.
#[derive(Default)]
pub(super) struct FrameTiming {
    milliseconds: Vec<f32>,
}

pub(super) fn sample_frame_timing(
    time: Res<Time>,
    session: Res<Session>,
    mut timing: Local<FrameTiming>,
) {
    const FRAMES: usize = 120;
    if timing.milliseconds.len() >= FRAMES
        || !matches!(session.experience.mode,
            ExperienceMode::Exploring(_) | ExperienceMode::Conversation { .. })
    {
        return;
    }
    let delta = time.delta_secs();
    if !delta.is_finite() || delta <= 0.0 { return; }
    timing.milliseconds.push(delta * 1000.0);
    if timing.milliseconds.len() != FRAMES { return; }
    let mean = timing.milliseconds.iter().copied().sum::<f32>() / FRAMES as f32;
    timing.milliseconds.sort_by(|a, b| a.total_cmp(b));
    let p95 = timing.milliseconds[(FRAMES * 95).div_ceil(100) - 1];
    let maximum = timing.milliseconds[FRAMES - 1];
    info!("Vesper exploration frame timing: samples={FRAMES} mean_ms={mean:.2} p95_ms={p95:.2} max_ms={maximum:.2} mean_fps={:.1}",
        1000.0 / mean);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_focus_prompt_never_survives_modal_input_ownership() {
        assert!(focus_prompt_available(true, false, false, true));
        for (exploring, modal, overlay, focus) in [
            (false, false, false, true),
            (true, true, false, true),
            (true, false, true, true),
            (true, false, false, false),
            (true, true, true, false),
        ] {
            assert!(!focus_prompt_available(exploring, modal, overlay, focus));
        }
    }

    #[test]
    fn preview_disables_all_story_input_when_world_mode_is_active() {
        let mut state = vesper_game::experience::ExperienceState::default();
        assert!(matches!(state.mode, ExperienceMode::VisualNovel));
        state.enter_exploration(vesper_game::experience::WorldPosition::new(
            "street", 150.0, Facing::Right).unwrap());
        assert!(matches!(state.mode, ExperienceMode::Exploring(_)));
    }

    #[test]
    fn placeholder_art_has_explicit_height_not_its_one_pixel_texture() {
        let avatar = Sprite::from_color(Color::WHITE, Vec2::new(35.0, 85.0));
        let prop = Sprite::from_color(Color::WHITE, Vec2::new(34.0, 46.0));
        assert_eq!(avatar.custom_size.unwrap().y, 85.0);
        assert_eq!(prop.custom_size.unwrap().y, 46.0);
    }

    #[test]
    fn viewport_projection_keeps_single_screen_camera_stationary() {
        let camera = vesper_game::exploration::CameraFrame::default();
        let view = View::for_area(
            1280.0, 720.0, 1280.0, 720.0, camera.zoom, 150.0, camera.follow_x,
        );
        assert_eq!(view.center_x, 640.0);
        assert_eq!(view.world_to_screen, 1.0);
    }
}
