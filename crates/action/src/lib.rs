//! An independent, action-first Bevy plugin. The visual novel is not loaded.
//! One bounded room is deliberate: feel and real rendering come before systems.

pub mod mechanics;
pub mod pixels;

use std::path::PathBuf;

use bevy::{
    asset::RenderAssetUsages,
    camera::ScalingMode,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use mechanics::{HERO_RADIUS, directional_hit, movement_axis, room_solids, slide_move};
use pixels::PixelSource;

const HERO_START: Vec2 = Vec2::new(-113.0, -51.0);
const DRONE_START: Vec2 = Vec2::new(80.0, -43.0);
const CONSOLE_POS: Vec2 = Vec2::new(215.0, -28.0);
const GATE_POS: Vec2 = Vec2::new(270.0, 59.0);
const HERO_SPEED: f32 = 133.0;
const DODGE_SPEED: f32 = 315.0;
const DRONE_SPEED: f32 = 66.0;

#[derive(Resource)]
pub struct DemoRoot(pub PathBuf);

#[derive(Resource)]
struct ArtHandles {
    hero_idle: Handle<Image>,
    hero_step: Handle<Image>,
    drone: Handle<Image>,
    console_off: Handle<Image>,
    console_on: Handle<Image>,
    gate_closed: Handle<Image>,
    gate_open: Handle<Image>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Playing,
    Lost,
    Won,
}

#[derive(Resource)]
struct Encounter {
    outcome: Outcome,
    hp: u8,
    drone_hp: u8,
    facing: Vec2,
    attack_cooldown: f32,
    attack_visible: f32,
    dodge_cooldown: f32,
    dodge_remaining: f32,
    invulnerability: f32,
    drone_cooldown: f32,
    drone_windup: f32,
    drone_stagger: f32,
    step_clock: f32,
    terminal_on: bool,
    notice: &'static str,
}

impl Default for Encounter {
    fn default() -> Self {
        Self {
            outcome: Outcome::Playing,
            hp: 5,
            drone_hp: 3,
            facing: Vec2::X,
            attack_cooldown: 0.0,
            attack_visible: 0.0,
            dodge_cooldown: 0.0,
            dodge_remaining: 0.0,
            invulnerability: 0.0,
            drone_cooldown: 0.0,
            drone_windup: 0.0,
            drone_stagger: 0.0,
            step_clock: 0.0,
            terminal_on: false,
            notice: "Disable the security drone.",
        }
    }
}

#[derive(Component)]
struct Hero;
#[derive(Component)]
struct Drone;
#[derive(Component)]
struct Console;
#[derive(Component)]
struct Gate;
#[derive(Component)]
struct Slash;
#[derive(Component)]
struct StatusHud;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Encounter>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    restart,
                    move_hero,
                    strike,
                    move_drone,
                    interact,
                    update_slash,
                    update_hud,
                )
                    .chain(),
            );
    }
}

fn art_handle(name: &str, root: &DemoRoot, images: &mut Assets<Image>) -> Handle<Image> {
    let path = root.0.join("art").join(name);
    let source = PixelSource::read(&path)
        .unwrap_or_else(|error| panic!("Action prototype asset error: {error}"));
    let image = Image::new(
        Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        source.rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    images.add(image)
}

fn setup(mut commands: Commands, root: Res<DemoRoot>, mut images: ResMut<Assets<Image>>) {
    let room = art_handle("room.px", &root, &mut images);
    let hero_idle = art_handle("hero-idle.px", &root, &mut images);
    let hero_step = art_handle("hero-step.px", &root, &mut images);
    let drone = art_handle("drone.px", &root, &mut images);
    let console_off = art_handle("terminal-off.px", &root, &mut images);
    let console_on = art_handle("terminal-on.px", &root, &mut images);
    let gate_closed = art_handle("gate-closed.px", &root, &mut images);
    let gate_open = art_handle("gate-open.px", &root, &mut images);
    let slash = art_handle("slash.px", &root, &mut images);

    commands.insert_resource(ArtHandles {
        hero_idle: hero_idle.clone(),
        hero_step,
        drone: drone.clone(),
        console_off: console_off.clone(),
        console_on,
        gate_closed: gate_closed.clone(),
        gate_open,
    });

    // AutoMin *expands* the view on odd aspect ratios. It never stretches the
    // artwork or cuts off the action on a tall / narrow desktop window.
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 640.0,
                min_height: 360.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Msaa::Off,
    ));
    commands.spawn((
        Sprite::from_image(room),
        Transform::from_xyz(0.0, 0.0, -5.0).with_scale(Vec3::splat(2.0)),
        Name::new("Depot room - external pixel source"),
    ));
    commands.spawn((
        Sprite::from_image(gate_closed),
        Transform::from_xyz(GATE_POS.x, GATE_POS.y, 3.0).with_scale(Vec3::splat(2.0)),
        Gate,
        Name::new("Locked transit door"),
    ));
    commands.spawn((
        Sprite::from_image(console_off),
        Transform::from_xyz(CONSOLE_POS.x, CONSOLE_POS.y, 4.0)
            .with_scale(Vec3::splat(2.0)),
        Console,
        Name::new("Power terminal"),
    ));
    commands.spawn((
        Sprite::from_image(drone),
        Transform::from_xyz(DRONE_START.x, DRONE_START.y, 7.0)
            .with_scale(Vec3::splat(2.0)),
        Drone,
        Name::new("Security drone"),
    ));
    commands.spawn((
        Sprite::from_image(hero_idle),
        Transform::from_xyz(HERO_START.x, HERO_START.y, 8.0)
            .with_scale(Vec3::splat(2.0)),
        Hero,
        Name::new("Maintenance worker"),
    ));
    commands.spawn((
        Sprite::from_image(slash),
        Transform::from_xyz(HERO_START.x, HERO_START.y, 9.0)
            .with_scale(Vec3::splat(2.0)),
        Visibility::Hidden,
        Slash,
        Name::new("Melee arc"),
    ));

    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.93, 0.92, 0.83)),
        Node {
            position_type: PositionType::Absolute,
            left: px(18),
            top: px(14),
            width: percent(92.0),
            ..default()
        },
        StatusHud,
    ));
    commands.spawn((
        Text::new("WASD/ARROWS  MOVE  |  J/ENTER  STRIKE  |  SPACE  DODGE\nE  INTERACT  |  R  RETRY"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.78, 0.81, 0.77)),
        Node {
            position_type: PositionType::Absolute,
            left: px(18),
            bottom: px(12),
            width: percent(92.0),
            ..default()
        },
    ));
    info!("Vesper action room ready; original external pixel sources loaded.");
}

fn facing_from_axis(axis: Vec2, previous: Vec2) -> Vec2 {
    if axis == Vec2::ZERO {
        previous
    } else if axis.x.abs() >= axis.y.abs() {
        Vec2::new(axis.x.signum(), 0.0)
    } else {
        Vec2::new(0.0, axis.y.signum())
    }
}

fn move_hero(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut encounter: ResMut<Encounter>,
    mut hero: Query<(&mut Transform, &mut Sprite), With<Hero>>,
    art: Res<ArtHandles>,
) {
    if encounter.outcome != Outcome::Playing {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    encounter.attack_cooldown = (encounter.attack_cooldown - dt).max(0.0);
    encounter.dodge_cooldown = (encounter.dodge_cooldown - dt).max(0.0);
    encounter.invulnerability = (encounter.invulnerability - dt).max(0.0);
    encounter.drone_cooldown = (encounter.drone_cooldown - dt).max(0.0);
    let axis = movement_axis(
        keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft),
        keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight),
        keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown),
        keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp),
    );
    if encounter.dodge_remaining <= 0.0 {
        encounter.facing = facing_from_axis(axis, encounter.facing);
    }
    if keys.just_pressed(KeyCode::Space) && encounter.dodge_cooldown <= 0.0 {
        encounter.dodge_remaining = 0.17;
        encounter.dodge_cooldown = 0.78;
        encounter.invulnerability = 0.30;
    }
    let dodging = encounter.dodge_remaining > 0.0;
    let motion = if dodging {
        encounter.facing * DODGE_SPEED
    } else {
        axis * HERO_SPEED
    };
    encounter.dodge_remaining = (encounter.dodge_remaining - dt).max(0.0);
    encounter.step_clock += dt;

    let Ok((mut transform, mut sprite)) = hero.single_mut() else {
        return;
    };
    let next = slide_move(
        transform.translation.truncate(),
        motion * dt,
        HERO_RADIUS,
        &room_solids(),
    );
    transform.translation.x = next.x;
    transform.translation.y = next.y;
    sprite.image = if axis != Vec2::ZERO && (encounter.step_clock * 9.0) as u32 % 2 == 0 {
        art.hero_step.clone()
    } else {
        art.hero_idle.clone()
    };
    sprite.flip_x = encounter.facing.x < 0.0;
    sprite.color = if encounter.invulnerability > 0.0
        && (encounter.step_clock * 20.0) as u32 % 2 == 0
    {
        Color::srgba(0.6, 1.0, 1.0, 0.70)
    } else {
        Color::WHITE
    };
}

fn strike(
    keys: Res<ButtonInput<KeyCode>>,
    mut encounter: ResMut<Encounter>,
    hero: Query<&Transform, With<Hero>>,
    mut drone: Query<(&mut Transform, &mut Sprite, &mut Visibility), (With<Drone>, Without<Hero>)>,
    mut arc: Query<(&mut Transform, &mut Visibility), (With<Slash>, Without<Hero>, Without<Drone>)>,
) {
    if encounter.outcome != Outcome::Playing
        || encounter.attack_cooldown > 0.0
        || !(keys.just_pressed(KeyCode::KeyJ) || keys.just_pressed(KeyCode::Enter))
    {
        return;
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    encounter.attack_cooldown = 0.37;
    encounter.attack_visible = 0.15;
    let facing = encounter.facing;
    if let Ok((mut arc_transform, mut arc_visibility)) = arc.single_mut() {
        arc_transform.translation = (hero.translation.truncate() + facing * 28.0).extend(9.0);
        arc_transform.rotation = Quat::from_rotation_z(facing.y.atan2(facing.x));
        *arc_visibility = Visibility::Visible;
    }
    if encounter.drone_hp == 0 {
        return;
    }
    if let Ok((mut transform, mut sprite, mut visibility)) = drone.single_mut()
        && directional_hit(
            hero.translation.truncate(),
            facing,
            transform.translation.truncate(),
        )
    {
        encounter.drone_hp -= 1;
        encounter.drone_stagger = 0.24;
        encounter.drone_windup = 0.0;
        let next = slide_move(
            transform.translation.truncate(),
            facing * 17.0,
            12.0,
            &room_solids(),
        );
        transform.translation.x = next.x;
        transform.translation.y = next.y;
        sprite.color = Color::srgb(1.0, 0.86, 0.72);
        if encounter.drone_hp == 0 {
            *visibility = Visibility::Hidden;
            encounter.notice = "Drone disabled. Restore the power terminal (E).";
        } else {
            encounter.notice = "Direct hit. Strike again!";
        }
    }
}

fn move_drone(
    time: Res<Time>,
    mut encounter: ResMut<Encounter>,
    hero: Query<&Transform, With<Hero>>,
    mut drone: Query<(&mut Transform, &mut Sprite), (With<Drone>, Without<Hero>)>,
    art: Res<ArtHandles>,
) {
    if encounter.outcome != Outcome::Playing || encounter.drone_hp == 0 {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    let Ok(hero) = hero.single() else {
        return;
    };
    let Ok((mut transform, mut sprite)) = drone.single_mut() else {
        return;
    };
    let offset = hero.translation.truncate() - transform.translation.truncate();
    let distance = offset.length();
    if encounter.drone_stagger > 0.0 {
        encounter.drone_stagger = (encounter.drone_stagger - dt).max(0.0);
        return;
    }
    if encounter.drone_windup > 0.0 {
        encounter.drone_windup -= dt;
        sprite.color = Color::srgb(1.0, 0.78, 0.36);
        if encounter.drone_windup <= 0.0 && distance < 49.0 && encounter.invulnerability <= 0.0 {
            encounter.hp = encounter.hp.saturating_sub(1);
            encounter.invulnerability = 0.80;
            encounter.notice = "Hit! Watch the orange telegraph and dodge.";
            if encounter.hp == 0 {
                encounter.outcome = Outcome::Lost;
                encounter.notice = "Power lost. Press R to retry the room.";
            }
        }
        return;
    }
    sprite.image = art.drone.clone();
    sprite.color = Color::WHITE;
    if distance < 52.0 && encounter.drone_cooldown <= 0.0 {
        encounter.drone_windup = 0.45;
        encounter.drone_cooldown = 1.20;
    } else if distance > 47.0 && distance < 232.0 {
        let velocity = offset.normalize_or_zero() * DRONE_SPEED * dt;
        let next = slide_move(transform.translation.truncate(), velocity, 11.0, &room_solids());
        transform.translation.x = next.x;
        transform.translation.y = next.y;
    }
}

fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    mut encounter: ResMut<Encounter>,
    hero: Query<&Transform, With<Hero>>,
    mut console: Query<(&Transform, &mut Sprite), (With<Console>, Without<Gate>, Without<Hero>)>,
    mut gate: Query<&mut Sprite, (With<Gate>, Without<Console>, Without<Hero>)>,
    art: Res<ArtHandles>,
) {
    if encounter.outcome != Outcome::Playing || !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Ok(hero) = hero.single() else {
        return;
    };
    let point = hero.translation.truncate();
    if let Ok((console_transform, mut console_sprite)) = console.single_mut()
        && point.distance(console_transform.translation.truncate()) < 65.0
    {
        if encounter.drone_hp > 0 {
            encounter.notice = "Power console locked by the security drone.";
            return;
        }
        encounter.terminal_on = true;
        console_sprite.image = art.console_on.clone();
        if let Ok(mut gate_sprite) = gate.single_mut() {
            gate_sprite.image = art.gate_open.clone();
        }
        encounter.notice = "Power restored. Reach the illuminated exit (E).";
        return;
    }
    if point.distance(GATE_POS) < 59.0 {
        if encounter.terminal_on {
            encounter.outcome = Outcome::Won;
            encounter.notice = "DEVELOPMENT ROOM CLEARED. Press R to replay.";
        } else {
            encounter.notice = "Door is locked. Restore the power first.";
        }
    }
}

fn update_slash(
    time: Res<Time>,
    mut encounter: ResMut<Encounter>,
    mut slash: Query<&mut Visibility, With<Slash>>,
) {
    encounter.attack_visible = (encounter.attack_visible - time.delta_secs()).max(0.0);
    if encounter.attack_visible <= 0.0 && let Ok(mut visible) = slash.single_mut() {
        *visible = Visibility::Hidden;
    }
}

fn update_hud(encounter: Res<Encounter>, mut text: Query<&mut Text, With<StatusHud>>) {
    if !encounter.is_changed() {
        return;
    }
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let objective = match encounter.outcome {
        Outcome::Playing => encounter.notice,
        Outcome::Lost => "YOU WERE DISABLED. Press R to retry.",
        Outcome::Won => "ROOM CLEARED. Press R to play again.",
    };
    text.0 = format!(
        "SIGNAL DEPOT   |   HEALTH {}/5   |   DRONE {}/3\n{}",
        encounter.hp, encounter.drone_hp, objective
    );
}

fn restart(
    keys: Res<ButtonInput<KeyCode>>,
    mut encounter: ResMut<Encounter>,
    mut hero: Query<(&mut Transform, &mut Sprite), With<Hero>>,
    mut drone: Query<(&mut Transform, &mut Sprite, &mut Visibility), (With<Drone>, Without<Hero>)>,
    mut console: Query<&mut Sprite, (With<Console>, Without<Hero>, Without<Drone>)>,
    mut gate: Query<&mut Sprite, (With<Gate>, Without<Hero>, Without<Drone>, Without<Console>)>,
    art: Res<ArtHandles>,
) {
    if encounter.outcome == Outcome::Playing || !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    *encounter = Encounter::default();
    if let Ok((mut transform, mut sprite)) = hero.single_mut() {
        transform.translation = HERO_START.extend(8.0);
        sprite.image = art.hero_idle.clone();
        sprite.color = Color::WHITE;
    }
    if let Ok((mut transform, mut sprite, mut visibility)) = drone.single_mut() {
        transform.translation = DRONE_START.extend(7.0);
        sprite.image = art.drone.clone();
        sprite.color = Color::WHITE;
        *visibility = Visibility::Visible;
    }
    if let Ok(mut sprite) = console.single_mut() {
        sprite.image = art.console_off.clone();
    }
    if let Ok(mut sprite) = gate.single_mut() {
        sprite.image = art.gate_closed.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn idle_facing_is_stable_and_diagonal_is_cardinalized() {
        assert_eq!(facing_from_axis(Vec2::ZERO, -Vec2::X), -Vec2::X);
        assert_eq!(facing_from_axis(Vec2::new(0.2, 1.0), Vec2::X), Vec2::Y);
        assert_eq!(facing_from_axis(Vec2::new(-1.0, 0.2), Vec2::Y), -Vec2::X);
    }

    #[test]
    fn cardinal_melee_orientation_does_not_drift() {
        assert!((Vec2::Y.y.atan2(Vec2::Y.x) - PI / 2.0).abs() < 0.001);
    }
}
