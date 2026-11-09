//! Author-owned, bounded data model for optional 2D narrative locations.
//!
//! No Bevy dependency, no executable author code, no implied avatar movement.
//! The player will consume validated locations only after NA05/NA06 supplies
//! an exploration controller and a deterministic story/event bridge.
use std::{collections::BTreeMap, path::Path};
use serde::Deserialize;
use vesper_story::{Story, Value};
use super::{checked_file, id_valid};

const MAX_LOCATIONS: usize = 24;
const MAX_INTERACTABLES: usize = 32;
const MAX_LAYERS: usize = 12;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exploration {
    /// Separate from the legacy visual novel's game format version.
    pub schema_version: u32,
    /// An optional standalone walk-first opening. Omit it for pure VNs.
    #[serde(default)]
    pub start: Option<Start>,
    /// Optional sprites. Missing artwork will use an engine-owned placeholder
    /// at the future movement milestone; no texture path is synthesized.
    #[serde(default)]
    pub avatar: Avatar,
    /// Reader-facing world prompts are supplied by the game, with English
    /// fallback for existing fixtures and pure narrative projects.
    #[serde(default)]
    pub ui: ExplorationUi,
    pub locations: BTreeMap<String, Location>,
}

/// Optional per-game words in the walking HUD and focus prompts.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExplorationUi {
    pub walk: String,
    pub interact: String,
    pub talk_to: String,
    pub inspect: String,
    pub enter: String,
    pub pause: String,
    pub menu: String,
    /// Optional author-facing names for NPC/prop IDs and destination location IDs.
    /// Missing entries preserve the original internal ID labels.
    pub target_names: BTreeMap<String, String>,
}

impl ExplorationUi {
    pub fn target_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.target_names.get(id).map_or(id, String::as_str)
    }
}

impl Default for ExplorationUi {
    fn default() -> Self {
        Self {
            walk: "Walk".into(),
            interact: "Interact/Exit".into(),
            talk_to: "Talk to".into(),
            inspect: "Inspect".into(),
            enter: "Enter".into(),
            pause: "Pause".into(),
            menu: "Menu".into(),
            target_names: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    pub location: String,
    pub spawn: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Avatar {
    pub idle: Option<String>,
    pub walk: Vec<String>,
    pub walk_fps: f32,
    pub speed: f32,
    pub scale: f32,
}
impl Default for Avatar {
    fn default() -> Self {
        Self { idle: None, walk: Vec::new(), walk_fps: 8.0, speed: 230.0, scale: 1.0 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// Game-relative PNG, rendered with aspect-preserving framing.
    pub background: String,
    /// Logical world pixels; a single screen is perfectly valid.
    pub width: f32,
    pub height: f32,
    /// Camera framing is an author-owned preference, not a forced scrolling
    /// camera. A single-screen location can opt out of following entirely.
    #[serde(default)]
    pub camera: CameraFrame,
    /// Optional one-key access to an authored NPC or interactive prop
    /// in this location, without mandatory walking or teleportation.
    #[serde(default)]
    pub quick_interact: Option<String>,
    pub spawns: BTreeMap<String, Spawn>,
    #[serde(default)]
    pub npcs: BTreeMap<String, Npc>,
    #[serde(default)]
    pub props: BTreeMap<String, Prop>,
    #[serde(default)]
    pub exits: BTreeMap<String, Exit>,
    #[serde(default)]
    pub layers: BTreeMap<String, Layer>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CameraFrame {
    /// Logical zoom, independent of physical display size.
    pub zoom: f32,
    /// Normalized vertical focus, so the avatar may sit above/below center.
    pub horizon: f32,
    pub follow_x: bool,
}
impl Default for CameraFrame {
    fn default() -> Self {
        Self { zoom: 1.0, horizon: 0.5, follow_x: true }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spawn {
    pub x: f32,
    #[serde(default)]
    pub facing: super::experience::Facing,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub x: f32,
    pub interact_scene: String,
    #[serde(default)]
    pub image: Option<String>,
    /// Relative to the existing 85px tall actor reference. Default 1.0.
    #[serde(default = "default_actor_scale")]
    pub scale: f32,
    /// Render-only raise above the area's ground line in logical world pixels.
    /// Positive is up; does not move the NPC's horizontal interaction point.
    #[serde(default)]
    pub elevation: f32,
    #[serde(default = "default_radius")]
    pub interaction_radius: f32,
    /// Optional story-state gate; missing variables are treated as false.
    #[serde(default)]
    pub visible_if: Option<VisibilityCondition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prop {
    pub x: f32,
    #[serde(default)]
    pub interact_scene: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    /// Relative to the existing 46px tall prop reference. Default 1.0.
    #[serde(default = "default_actor_scale")]
    pub scale: f32,
    /// Render-only raise above the area's ground line in logical world pixels.
    /// Positive is up; x and the interaction radius remain unchanged.
    #[serde(default)]
    pub elevation: f32,
    #[serde(default = "default_radius")]
    pub interaction_radius: f32,
    #[serde(default)]
    pub visible_if: Option<VisibilityCondition>,
}

/// Simple equality against persistent, typed .vn variables.
/// No script evaluation, time-dependent behavior, or hidden quest state.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityCondition {
    pub variable: String,
    pub equals: ConditionValue,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ConditionValue {
    Number(i64),
    Boolean(bool),
    Text(String),
}

impl VisibilityCondition {
    pub fn matches(&self, variables: &BTreeMap<String, Value>) -> bool {
        match (variables.get(&self.variable), &self.equals) {
            (Some(Value::Number(actual)), ConditionValue::Number(expected)) => actual == expected,
            (Some(Value::Boolean(actual)), ConditionValue::Boolean(expected)) => actual == expected,
            (Some(Value::Text(actual)), ConditionValue::Text(expected)) => actual == expected,
            _ => false,
        }
    }

    pub fn validate(&self, label: &str) -> Result<(), String> {
        identifier(&self.variable, &format!("{label}.visible_if.variable"))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exit {
    pub x: f32,
    pub to_location: String,
    pub to_spawn: String,
    #[serde(default = "default_radius")]
    pub interaction_radius: f32,
}

/// Z-order group for original game-owned PNG layers. The default preserves
/// all existing scenery layering; foreground sits above NPCs and props but
/// below the player, and overlay sits above the complete world cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerPlane {
    #[default]
    Background,
    Foreground,
    Overlay,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub image: String,
    #[serde(default = "default_parallax")]
    pub parallax: f32,
    #[serde(default)]
    pub depth: f32,
    /// Optional actor-occluding layer: legacy manifests default to background.
    #[serde(default)]
    pub plane: LayerPlane,
}

fn default_radius() -> f32 { 72.0 }
fn default_actor_scale() -> f32 { 1.0 }
fn default_parallax() -> f32 { 1.0 }

fn valid_coordinate(x: f32, width: f32) -> bool {
    x.is_finite() && x >= 0.0 && x <= width
}

fn validate_radius(radius: f32) -> bool {
    radius.is_finite() && (20.0..=200.0).contains(&radius)
}

fn image(root: &Path, path: &str, label: &str) -> Result<(), String> {
    let file = checked_file(root, path).map_err(|error| format!("{label}: {error}"))?;
    if !file.extension().and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    {
        return Err(format!("{label} must reference an existing game-relative PNG"));
    }
    Ok(())
}

fn identifier(id: &str, label: &str) -> Result<(), String> {
    if !id_valid(id) {
        return Err(format!("{label}: invalid identifier '{id}'"));
    }
    Ok(())
}

fn scene(story: &Story, id: &str, label: &str) -> Result<(), String> {
    identifier(id, label)?;
    if !story.has_scene(id) {
        return Err(format!("{label}: unknown authored scene '{id}'"));
    }
    Ok(())
}

impl Exploration {
    /// Only an explicit [exploration.start] opts into the walk-first preview.
    /// Standalone VNs have no player-world position to initialize.
    pub fn start_position(&self) -> Option<super::experience::WorldPosition> {
        let start = self.start.as_ref()?;
        let spawn = self.locations.get(&start.location)?.spawns.get(&start.spawn)?;
        super::experience::WorldPosition::new(
            start.location.clone(), spawn.x, spawn.facing
        ).ok()
    }

    /// Resolve a declared destination only while inside the named exit's
    /// interaction radius. No forced traversal or implicit teleport.
    pub fn exit_position(
        &self, current: &super::experience::WorldPosition, exit_name: &str
    ) -> Result<super::experience::WorldPosition, String> {
        let area = self.locations.get(&current.location)
            .ok_or_else(|| format!("unknown current location '{}'", current.location))?;
        if !valid_coordinate(current.x, area.width) {
            return Err("current world X is outside authored walking bounds".into());
        }
        let exit = area.exits.get(exit_name)
            .ok_or_else(|| format!("unknown exit '{}' in '{}'", exit_name, current.location))?;
        if (current.x - exit.x).abs() > exit.interaction_radius {
            return Err(format!("exit '{exit_name}' is outside the interaction radius"));
        }
        let to = self.locations.get(&exit.to_location)
            .ok_or_else(|| format!("missing exit destination '{}'", exit.to_location))?;
        let spawn = to.spawns.get(&exit.to_spawn)
            .ok_or_else(|| format!("missing destination spawn '{}'", exit.to_spawn))?;
        super::experience::WorldPosition::new(
            exit.to_location.clone(), spawn.x, spawn.facing
        )
    }

    /// Every reference is checked before Bevy sees this data; a hot reload may
    /// atomically reject invalid world state without disturbing the old world.
    pub fn validate(&self, root: &Path, story: &Story) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!("exploration.schema_version: unsupported version {}; expected 1",
                self.schema_version));
        }
        if self.locations.is_empty() || self.locations.len() > MAX_LOCATIONS {
            return Err(format!("exploration.locations must contain 1..={MAX_LOCATIONS} locations"));
        }
        if let Some(start) = &self.start {
            identifier(&start.location, "exploration.start.location")?;
            identifier(&start.spawn, "exploration.start.spawn")?;
            let target = self.locations.get(&start.location).ok_or_else(|| format!(
                "exploration.start.location: missing location '{}'", start.location
            ))?;
            if !target.spawns.contains_key(&start.spawn) {
                return Err(format!("exploration.start.spawn: missing spawn '{}' in location '{}'",
                    start.spawn, start.location));
            }
        }
        for (field, value) in [
            ("walk", &self.ui.walk),
            ("interact", &self.ui.interact),
            ("talk_to", &self.ui.talk_to),
            ("inspect", &self.ui.inspect),
            ("enter", &self.ui.enter),
            ("pause", &self.ui.pause),
            ("menu", &self.ui.menu),
        ] {
            if value.trim().is_empty() || value.chars().count() > 32
                || value.chars().any(char::is_control)
            {
                return Err(format!("exploration.ui.{field} must be 1..=32 printable characters"));
            }
        }
        if self.ui.target_names.len() > 128 {
            return Err("exploration.ui.target_names supports at most 128 labels".into());
        }
        for (id, display) in &self.ui.target_names {
            identifier(id, "exploration.ui.target_names")?;
            if display.trim().is_empty()
                || display.chars().count() > 64
                || display.chars().any(char::is_control)
            {
                return Err(format!(
                    "exploration.ui.target_names.{id} must be 1..=64 printable characters"
                ));
            }
            if !self.locations.contains_key(id)
                && !self.locations.values().any(|area| {
                    area.npcs.contains_key(id)
                        || area.props.contains_key(id)
                        || area.exits.contains_key(id)
                })
            {
                return Err(format!(
                    "exploration.ui.target_names.{id}: unknown authored target"
                ));
            }
        }
        let avatar = &self.avatar;
        for (field, value, lo, hi) in [
            ("walk_fps", avatar.walk_fps, 1.0, 30.0),
            ("speed", avatar.speed, 40.0, 650.0),
            ("scale", avatar.scale, 0.2, 4.0),
        ] {
            if !value.is_finite() || !(lo..=hi).contains(&value) {
                return Err(format!("exploration.avatar.{field} must be finite within {lo}..={hi}"));
            }
        }
        if avatar.walk.len() > 16 {
            return Err("exploration.avatar.walk supports at most 16 PNG frames".into());
        }
        if let Some(path) = &avatar.idle {
            image(root, path, "exploration.avatar.idle")?;
        }
        for (i, path) in avatar.walk.iter().enumerate() {
            image(root, path, &format!("exploration.avatar.walk[{i}]"))?;
        }

        for (id, location) in &self.locations {
            let label = format!("exploration.locations.{id}");
            identifier(id, &label)?;
            if !location.width.is_finite() || !(320.0..=8192.0).contains(&location.width)
                || !location.height.is_finite()
                || !(360.0..=2160.0).contains(&location.height)
            {
                return Err(format!("{label}: world dimensions must be finite and within 320..=8192 by 360..=2160"));
            }
            image(root, &location.background, &format!("{label}.background"))?;
            if !location.camera.zoom.is_finite()
                || !(0.5..=2.0).contains(&location.camera.zoom)
                || !location.camera.horizon.is_finite()
                || !(0.0..=1.0).contains(&location.camera.horizon)
            {
                return Err(format!("{label}.camera: zoom/horizon must be finite within bounds"));
            }
            if location.spawns.is_empty() || location.spawns.len() > 64 {
                return Err(format!("{label}.spawns must contain 1..=64 entries"));
            }
            if location.npcs.len() + location.props.len() + location.exits.len() > MAX_INTERACTABLES {
                return Err(format!("{label} exceeds {MAX_INTERACTABLES} NPCs/props/exits"));
            }
            if let Some(id) = &location.quick_interact {
                identifier(id, &format!("{label}.quick_interact"))?;
                let npc = location.npcs.contains_key(id);
                let prop = location.props.get(id)
                    .is_some_and(|prop| prop.interact_scene.is_some());
                if npc == prop {
                    return Err(format!(
                        "{label}.quick_interact must identify exactly one NPC or interactive prop"
                    ));
                }
            }
            if location.layers.len() > MAX_LAYERS {
                return Err(format!("{label}.layers exceeds {MAX_LAYERS} layers"));
            }
            for (name, spawn) in &location.spawns {
                let slot = format!("{label}.spawns.{name}");
                identifier(name, &slot)?;
                if !valid_coordinate(spawn.x, location.width) {
                    return Err(format!("{slot}.x must be finite and inside the walkable bounds"));
                }
            }
            for (name, npc) in &location.npcs {
                let slot = format!("{label}.npcs.{name}");
                identifier(name, &slot)?;
                if !valid_coordinate(npc.x, location.width)
                    || !validate_radius(npc.interaction_radius)
                    || !npc.scale.is_finite()
                    || !(0.35..=3.5).contains(&npc.scale)
                    || !npc.elevation.is_finite()
                    || npc.elevation.abs() > location.height
                {
                    return Err(format!(
                        "{slot}: invalid interaction position/radius/scale/elevation"
                    ));
                }
                scene(story, &npc.interact_scene, &format!("{slot}.interact_scene"))?;
                if let Some(condition) = &npc.visible_if {
                    condition.validate(&slot)?;
                }
                if let Some(path) = &npc.image {
                    image(root, path, &format!("{slot}.image"))?;
                }
            }
            for (name, prop) in &location.props {
                let slot = format!("{label}.props.{name}");
                identifier(name, &slot)?;
                if !valid_coordinate(prop.x, location.width)
                    || !validate_radius(prop.interaction_radius)
                    || !prop.scale.is_finite()
                    || !(0.35..=3.5).contains(&prop.scale)
                    || !prop.elevation.is_finite()
                    || prop.elevation.abs() > location.height
                {
                    return Err(format!(
                        "{slot}: invalid interaction position/radius/scale/elevation"
                    ));
                }
                if let Some(scene_id) = &prop.interact_scene {
                    scene(story, scene_id, &format!("{slot}.interact_scene"))?;
                }
                if let Some(condition) = &prop.visible_if {
                    condition.validate(&slot)?;
                }
                if let Some(path) = &prop.image {
                    image(root, path, &format!("{slot}.image"))?;
                }
            }
            for (name, exit) in &location.exits {
                let slot = format!("{label}.exits.{name}");
                identifier(name, &slot)?;
                if !valid_coordinate(exit.x, location.width)
                    || !validate_radius(exit.interaction_radius)
                {
                    return Err(format!("{slot}: invalid exit position/radius"));
                }
                identifier(&exit.to_location, &format!("{slot}.to_location"))?;
                identifier(&exit.to_spawn, &format!("{slot}.to_spawn"))?;
                let Some(target) = self.locations.get(&exit.to_location) else {
                    return Err(format!("{slot}.to_location: missing location '{}'", exit.to_location));
                };
                if !target.spawns.contains_key(&exit.to_spawn) {
                    return Err(format!("{slot}.to_spawn: missing spawn '{}' in location '{}'",
                        exit.to_spawn, exit.to_location));
                }
            }
            for (name, layer) in &location.layers {
                let slot = format!("{label}.layers.{name}");
                identifier(name, &slot)?;
                if !layer.parallax.is_finite() || !(0.0..=2.0).contains(&layer.parallax)
                    || !layer.depth.is_finite() || !(-12.0..=12.0).contains(&layer.depth)
                {
                    return Err(format!("{slot}: parallax/depth out of bounds"));
                }
                image(root, &layer.image, &format!("{slot}.image"))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::atomic::{AtomicU64, Ordering}};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    const GOOD: &str = r#"
schema_version = 1
[start]
location = "street"
spawn = "home"
[avatar]
speed = 250.0
[locations.street]
background = "scene.png"
width = 1280.0
height = 720.0
[locations.street.camera]
zoom = 1.1
horizon = 0.55
[locations.street.spawns.home]
x = 120.0
[locations.street.npcs.friend]
x = 230.0
interact_scene = "chat"
interaction_radius = 75.0
[locations.street.exits.cafe]
x = 1000.0
to_location = "cafe"
to_spawn = "door"
[locations.cafe]
background = "scene.png"
width = 640.0
height = 720.0
[locations.cafe.spawns.door]
x = 50.0
facing = "left"
[locations.cafe.props.counter]
x = 145.0
interact_scene = "chat"
"#;

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(
                format!("vesper-exploration-validation-{}-{id}", std::process::id()));
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join("scene.png"), b"test-png-path").unwrap();
            Self(dir)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { fs::remove_dir_all(&self.0).unwrap(); }
    }

    fn load(source: &str) -> (Exploration, Fixture, Story) {
        let fixture = Fixture::new();
        let value: Exploration = toml::from_str(source).unwrap();
        let story = Story::parse("@scene chat\nfriend: \"Hello\"").unwrap();
        (value, fixture, story)
    }

    #[test]
    fn two_compact_locations_with_npc_prop_and_exit_validate() {
        let (world, fixture, story) = load(GOOD);
        world.validate(&fixture.0, &story).unwrap();
        assert_eq!(world.locations["street"].spawns["home"].x, 120.0);
        assert_eq!(world.locations["street"].camera.zoom, 1.1);
        assert_eq!(world.start.as_ref().unwrap().location, "street");
        assert_eq!(world.start_position().unwrap().x, 120.0);
        assert_eq!(world.locations["cafe"].camera.zoom, 1.0);
        assert_eq!(world.locations["cafe"].spawns["door"].facing,
            super::super::experience::Facing::Left);
        assert!(world.avatar.walk.is_empty());
    }

    #[test]
    fn invalid_references_and_out_of_bounds_data_are_rejected() {
        for (from, to, key) in [
            ("to_location = \"cafe\"", "to_location = \"missing\"", "to_location"),
            ("to_spawn = \"door\"", "to_spawn = \"missing\"", "to_spawn"),
            ("interact_scene = \"chat\"", "interact_scene = \"not_here\"", "interact_scene"),
            ("x = 1000.0", "x = 99999.0", "exits"),
            ("interaction_radius = 75.0", "interaction_radius = 999.0", "npcs"),
            ("width = 640.0", "width = -1.0", "dimensions"),
            ("schema_version = 1", "schema_version = 9", "schema_version"),
            ("zoom = 1.1", "zoom = 0.1", "camera"),
            ("spawn = \"home\"", "spawn = \"not_here\"", "start.spawn"),
            ("background = \"scene.png\"", "background = \"../outside.png\"", "background"),
        ] {
            let (world, fixture, story) = load(&GOOD.replacen(from, to, 1));
            let error = world.validate(&fixture.0, &story).unwrap_err();
            assert!(error.contains(key), "expected {key} in: {error}");
        }
    }

    #[test]
    fn exit_requires_proximity_and_uses_named_destination_spawn() {
        let (world, fixture, story) = load(GOOD);
        world.validate(&fixture.0, &story).unwrap();
        let near = super::super::experience::WorldPosition::new(
            "street", 970.0, super::super::experience::Facing::Right
        ).unwrap();
        let arrival = world.exit_position(&near, "cafe").unwrap();
        assert_eq!(arrival.location, "cafe");
        assert_eq!(arrival.x, 50.0);
        assert_eq!(arrival.facing, super::super::experience::Facing::Left);
        let far = super::super::experience::WorldPosition::new(
            "street", 100.0, super::super::experience::Facing::Right
        ).unwrap();
        assert!(world.exit_position(&far, "cafe").is_err());
        assert!(world.exit_position(&near, "unknown").is_err());
    }

    #[test]
    fn conditional_visibility_uses_persistent_typed_story_variables() {
        let gate: VisibilityCondition = toml::from_str(
            "variable='met_friend'\nequals=true"
        ).unwrap();
        let mut vars = BTreeMap::new();
        assert!(!gate.matches(&vars));
        vars.insert("met_friend".into(), Value::Boolean(false));
        assert!(!gate.matches(&vars));
        vars.insert("met_friend".into(), Value::Boolean(true));
        assert!(gate.matches(&vars));
        vars.insert("met_friend".into(), Value::Text("true".into()));
        assert!(!gate.matches(&vars));
        assert!(gate.validate("test").is_ok());

        let bad: VisibilityCondition = toml::from_str(
            "variable='bad name'\nequals=1"
        ).unwrap();
        assert!(bad.validate("test").is_err());
        assert!(toml::from_str::<VisibilityCondition>(
            "variable='valid'\nequals=1.2"
        ).is_err());
    }

    #[test]
    fn opt_in_quick_interaction_validates_an_unambiguous_scene_target() {
        let (mut world, fixture, story) = load(GOOD);
        assert!(world.locations["street"].quick_interact.is_none());
        world.locations.get_mut("street").unwrap().quick_interact =
            Some("friend".into());
        world.validate(&fixture.0, &story).unwrap();
        world.locations.get_mut("street").unwrap().quick_interact =
            Some("missing".into());
        assert!(world.validate(&fixture.0, &story).unwrap_err()
            .contains("quick_interact"));
        world.locations.get_mut("street").unwrap().quick_interact =
            Some("friend".into());
        world.locations.get_mut("street").unwrap().props.insert(
            "friend".into(),
            Prop { x: 130.0, interact_scene: Some("chat".into()),
                image: None, scale: 1.0, elevation: 0.0,
                interaction_radius: 70.0, visible_if: None },
        );
        assert!(world.validate(&fixture.0, &story).unwrap_err()
            .contains("quick_interact"));
    }

    #[test]
    fn localized_labels_and_actor_scaling_are_validated() {
        let (mut world, fixture, story) = load(GOOD);
        world.ui.walk = "Mover".into();
        world.ui.talk_to = "Hablar con".into();
        let npc = world.locations.get_mut("street").unwrap().npcs.get_mut("friend").unwrap();
        assert_eq!(npc.scale, 1.0);
        npc.scale = 1.6;
        world.validate(&fixture.0, &story).unwrap();
        world.ui.talk_to.clear();
        assert!(world.validate(&fixture.0, &story).unwrap_err().contains("ui.talk_to"));
        world.ui.talk_to = "Hablar con".into();
        world.locations.get_mut("street").unwrap().npcs.get_mut("friend").unwrap().scale = 4.0;
        assert!(world.validate(&fixture.0, &story).unwrap_err().contains("scale"));
    }

    #[test]
    fn authored_focus_names_are_optional_validated_and_do_not_rename_ids() {
        let (mut world, fixture, story) = load(GOOD);
        assert_eq!(world.ui.target_name("friend"), "friend");
        world.ui.target_names.insert("friend".into(), "Elena".into());
        world.ui.target_names.insert("cafe".into(), "la tienda".into());
        world.validate(&fixture.0, &story).unwrap();
        assert_eq!(world.ui.target_name("friend"), "Elena");
        assert_eq!(world.ui.target_name("cafe"), "la tienda");
        assert_eq!(world.locations["street"].npcs["friend"].interact_scene, "chat");

        world.ui.target_names.insert("friend".into(), "  ".into());
        assert!(world.validate(&fixture.0, &story).unwrap_err()
            .contains("ui.target_names.friend"));
        world.ui.target_names.insert("friend".into(), "Elena".into());
        world.ui.target_names.insert("unknown_id".into(), "????".into());
        assert!(world.validate(&fixture.0, &story).unwrap_err()
            .contains("unknown authored target"));
        world.ui.target_names.remove("unknown_id");
        world.ui.target_names.insert("friend".into(), "Elena\n".into());
        assert!(world.validate(&fixture.0, &story).is_err());
    }


    #[test]
    fn authored_occlusion_plane_is_optional_typed_and_keeps_existing_layers() {
        let source = format!("{GOOD}\n[locations.street.layers.old_sky]\nimage = \"scene.png\"\n[locations.street.layers.counter_front]\nimage = \"scene.png\"\nplane = \"foreground\"\ndepth = 2.0\n[locations.street.layers.rain_glass]\nimage = \"scene.png\"\nplane = \"overlay\"\n");
        let (world, fixture, story) = load(&source);
        world.validate(&fixture.0, &story).unwrap();
        let layers = &world.locations["street"].layers;
        assert_eq!(layers["old_sky"].plane, LayerPlane::Background);
        assert_eq!(layers["counter_front"].plane, LayerPlane::Foreground);
        assert_eq!(layers["rain_glass"].plane, LayerPlane::Overlay);
        assert_eq!(layers["counter_front"].depth, 2.0);

        let invalid = source.replace("plane = \"foreground\"", "plane = \"under_npc\"");
        assert!(toml::from_str::<Exploration>(&invalid).is_err());
        let invalid = source.replace("depth = 2.0", "depth = 50.0");
        let (world, fixture, story) = load(&invalid);
        assert!(world.validate(&fixture.0, &story).unwrap_err()
            .contains("layers.counter_front"));
        let invalid = source.replace("plane = \"foreground\"", "plane = 1");
        assert!(toml::from_str::<Exploration>(&invalid).is_err());
    }

    #[test]
    fn authored_actor_elevation_and_prop_scale_preserve_legacy_defaults() {
        let (mut world, fixture, story) = load(GOOD);
        assert_eq!(world.locations["street"].npcs["friend"].elevation, 0.0);
        assert_eq!(world.locations["cafe"].props["counter"].elevation, 0.0);
        assert_eq!(world.locations["cafe"].props["counter"].scale, 1.0);
        {
            let npc = world.locations.get_mut("street").unwrap()
                .npcs.get_mut("friend").unwrap();
            npc.elevation = 80.0;
        }
        {
            let prop = world.locations.get_mut("cafe").unwrap()
                .props.get_mut("counter").unwrap();
            prop.elevation = 170.0;
            prop.scale = 2.0;
        }
        world.validate(&fixture.0, &story).unwrap();
        assert_eq!(world.locations["cafe"].props["counter"].x, 145.0);
        // A visual raise never changes the independent one-dimensional
        // interaction point or the authored scene transition.
        assert_eq!(world.locations["cafe"].props["counter"].interact_scene.as_deref(),
            Some("chat"));
        let mut invalid = world.clone();
        invalid.locations.get_mut("cafe").unwrap()
            .props.get_mut("counter").unwrap().elevation = 721.0;
        assert!(invalid.validate(&fixture.0, &story).unwrap_err()
            .contains("props.counter"));
        let mut invalid = world.clone();
        invalid.locations.get_mut("street").unwrap()
            .npcs.get_mut("friend").unwrap().elevation = f32::NAN;
        assert!(invalid.validate(&fixture.0, &story).unwrap_err()
            .contains("npcs.friend"));
        let mut invalid = world.clone();
        invalid.locations.get_mut("cafe").unwrap()
            .props.get_mut("counter").unwrap().scale = 0.0;
        assert!(invalid.validate(&fixture.0, &story).unwrap_err()
            .contains("props.counter"));
    }

    #[test]
    fn missing_optional_world_never_forces_a_legacy_vn_to_walk() {
        let legacy: super::super::Manifest = toml::from_str(
            "format_version=1\nid='legacy'\ntitle='Legacy'\nstory='story.vn'\nentry='chat'"
        ).unwrap();
        assert!(legacy.exploration.is_none());
        let (world, fixture, story) = load(GOOD);
        world.validate(&fixture.0, &story).unwrap();
        assert!(toml::from_str::<Exploration>(
            "schema_version=1\nlocations='not_a_table'"
        ).is_err());
    }

    #[test]
    fn safe_locations_reject_undeclared_assets_and_unknown_fields() {
        let (mut world, fixture, story) = load(GOOD);
        world.locations.get_mut("cafe").unwrap().background = "not_here.png".into();
        assert!(world.validate(&fixture.0, &story).unwrap_err().contains("not_here.png"));
        assert!(toml::from_str::<Exploration>(
            &GOOD.replace("interaction_radius = 75.0",
                "interaction_radius = 75.0\ninvalid_field = true")
        ).is_err());
    }
}
