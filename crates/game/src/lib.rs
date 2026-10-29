//! Filesystem-based game format. Never interpret author-controlled asset paths
//! as unrestricted paths outside the chosen game directory.

use serde::{Deserialize, Serialize};
pub mod save;
pub mod preferences;
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};
use vesper_story::Story;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format_version: u32,
    pub id: String,
    pub title: String,
    pub story: String,
    pub entry: String,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub backgrounds: BTreeMap<String, String>,
    /// Optional external dialogue surfaces. Never compiled into the engine.
    #[serde(default)]
    pub panel_skins: BTreeMap<String, PanelSkin>,
    #[serde(default)]
    pub music: BTreeMap<String, String>,
    #[serde(default)]
    pub sounds: BTreeMap<String, String>,
    #[serde(default)]
    pub characters: BTreeMap<String, Character>,
    /// Background/non-speaking scene sprites; independent from named speakers.
    #[serde(default)]
    pub ambient_actors: BTreeMap<String, AmbientActor>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub expressions: BTreeMap<String, String>,
    /// Optional external stage styling. Existing manifests remain valid.
    #[serde(default)]
    pub stage: CharacterStage,
}

/// Static ambient-scene declaration; movement paths and flipbooks belong to
/// QOL07. All positions are on the same normalized 1280x720 canvas as the
/// background stage, independent of character expression/speaker slots.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AmbientActor {
    /// Safe, game-root-relative PNG path, read/decoded via Bevy assets.
    pub image: String,
    /// Normalized canvas coordinate: x/y in 0..=1.
    pub anchor: [f32; 2],
    /// Scale multiplier, not texture dimensions.
    pub scale: f32,
    /// Draw alpha: 0 invisible, 1 opaque.
    pub opacity: f32,
    /// Background composition ordering, behind foreground characters.
    pub depth: f32,
    /// None means all scenes. Some selects an existing named scene only.
    pub scene: Option<String>,
    /// Author-controlled initial state; per-scene visibility may be overridden.
    pub initially_visible: bool,
    /// Reserved deterministic phase for future QOL07 motion.
    pub start_phase_seconds: f32,
}

impl Default for AmbientActor {
    fn default() -> Self {
        Self {
            image: String::new(),
            anchor: [0.5, 0.5],
            scale: 1.0,
            opacity: 1.0,
            depth: 0.0,
            scene: None,
            initially_visible: true,
            start_phase_seconds: 0.0,
        }
    }
}

impl AmbientActor {
    pub fn validate(&self) -> Result<(), String> {
        if self.image.is_empty()
            || self.anchor.iter().any(|coord|
                !coord.is_finite() || !(0.0..=1.0).contains(coord))
            || !self.scale.is_finite() || !(0.1..=3.0).contains(&self.scale)
            || !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity)
            || !self.depth.is_finite() || !(-8.0..=8.0).contains(&self.depth)
            || !self.start_phase_seconds.is_finite()
            || !(0.0..=120.0).contains(&self.start_phase_seconds)
            || self.scene.as_ref().is_some_and(|scene| !id_valid(scene))
        {
            return Err("invalid ambient image, anchor, scale, opacity, depth, scene or phase".into());
        }
        Ok(())
    }
}

/// A validated, image-backed dialogue panel. Insets are logical pixels
/// ordered left, right, top, bottom, added to theme.panel.padding.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PanelSkin {
    pub image: String,
    pub insets: [f32; 4],
    /// Texture pixels retained along each edge for nine-slice scaling.
    pub slice_border: f32,
}
impl Default for PanelSkin {
    fn default() -> Self {
        Self { image: String::new(), insets: [32.0; 4], slice_border: 24.0 }
    }
}
impl PanelSkin {
    pub fn validate(&self) -> Result<(), String> {
        if self.image.is_empty()
            || self.insets.iter().any(|v| !v.is_finite() || !(0.0..=160.0).contains(v))
            || !self.slice_border.is_finite()
            || !(2.0..=160.0).contains(&self.slice_border)
        {
            return Err("panel skin image, content insets or slice border invalid".into());
        }
        Ok(())
    }
}

/// Logical 1280x720 stage offsets, scale, depth and subtle appearance motion.
/// All limits are enforced at load time, including during hot reload.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CharacterStage {
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale: f32,
    pub depth: f32,
    pub crossfade_seconds: f32,
    pub enter_seconds: f32,
    pub exit_seconds: f32,
    pub slide_pixels: f32,
    pub idle_pixels: f32,
    pub idle_period_seconds: f32,
}

impl Default for CharacterStage {
    fn default() -> Self {
        Self {
            offset_x: 0.0, offset_y: 0.0, scale: 1.0, depth: 0.0,
            crossfade_seconds: 0.0, enter_seconds: 0.0,
            exit_seconds: 0.0, slide_pixels: 0.0,
            idle_pixels: 0.0, idle_period_seconds: 4.0,
        }
    }
}

impl CharacterStage {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("offset_x", self.offset_x, -400.0, 400.0),
            ("offset_y", self.offset_y, -180.0, 180.0),
            ("scale", self.scale, 0.5, 1.5),
            ("depth", self.depth, 0.0, 8.0),
            ("crossfade_seconds", self.crossfade_seconds, 0.0, 2.0),
            ("enter_seconds", self.enter_seconds, 0.0, 2.0),
            ("exit_seconds", self.exit_seconds, 0.0, 2.0),
            ("slide_pixels", self.slide_pixels, 0.0, 60.0),
            ("idle_pixels", self.idle_pixels, 0.0, 8.0),
            ("idle_period_seconds", self.idle_period_seconds, 0.5, 10.0),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(format!("character.stage.{name} must be finite and in {min}..={max}"));
            }
        }
        Ok(())
    }
}

/// An externally editable visual theme; values are CSS-like logical UI sizes.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    pub ui_scale: f32,
    pub fade_seconds: f32,
    pub cinematic: CinematicTheme,
    pub atmosphere: AtmosphereTheme,
    pub feedback: FeedbackTheme,
    pub reveal_speed: TextRevealSpeed,
    pub font: FontSelection,
    pub panel: PanelStyle,
    pub speaker: TextStyle,
    pub dialogue: TextStyle,
    pub choices: TextStyle,
    pub hint: TextStyle,
}

/// Subtle shader-free visual atmosphere; Off keeps legacy games unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtmospherePreset {
    #[default]
    Off,
    Motes,
    Glow,
    Vignette,
    Moonlit,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AtmosphereTheme {
    pub preset: AtmospherePreset,
    /// Stable author-controlled seed; identical settings yield identical layout.
    pub seed: u64,
    /// Only relevant to Motes/Moonlit; always capped at 32 entities.
    pub count: u8,
    /// 0 disables motion; logical canvas pixels per second otherwise.
    pub speed: f32,
    /// Bounded logical radius of each mote.
    pub radius: f32,
    /// Contribution of tint/edge overlays; never hides story or characters.
    pub opacity: f32,
    pub color: [f32; 4],
}

impl Default for AtmosphereTheme {
    fn default() -> Self {
        Self {
            preset: AtmospherePreset::Off,
            seed: 73, count: 16, speed: 8.0, radius: 3.0,
            opacity: 0.15, color: [0.9, 0.84, 1.0, 1.0],
        }
    }
}

impl AtmosphereTheme {
    pub fn validate(&self) -> Result<(), String> {
        if self.count > 32
            || !self.speed.is_finite() || !(0.0..=24.0).contains(&self.speed)
            || !self.radius.is_finite() || !(1.0..=8.0).contains(&self.radius)
            || !self.opacity.is_finite() || !(0.0..=0.35).contains(&self.opacity)
            || !self.color.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        {
            return Err("theme.atmosphere has invalid count, speed, radius, opacity or color".into());
        }
        Ok(())
    }
}

/// Shared, validated interaction design tokens for menus and story choices.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FeedbackTheme {
    pub focus_seconds: f32,
    pub confirm_seconds: f32,
    pub cue_opacity: f32,
    pub cue_color: [f32; 4],
}

impl Default for FeedbackTheme {
    fn default() -> Self {
        Self {
            // Opt-in: legacy game themes retain instant focus and no cue.
            focus_seconds: 0.0, confirm_seconds: 0.0, cue_opacity: 0.45,
            cue_color: [0.96, 0.75, 0.40, 1.0],
        }
    }
}

impl FeedbackTheme {
    pub fn validate(&self) -> Result<(), String> {
        if !self.focus_seconds.is_finite() || !(0.0..=0.35).contains(&self.focus_seconds)
            || !self.confirm_seconds.is_finite()
            || !(0.0..=0.35).contains(&self.confirm_seconds)
            || !self.cue_opacity.is_finite()
            || !(0.0..=0.65).contains(&self.cue_opacity)
            || !self.cue_color.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        {
            return Err("theme.feedback contains invalid animation duration, cue opacity or color".into());
        }
        Ok(())
    }
}

/// Optional external-background cinematics. The default Legacy mode retains
/// existing authored games' FadeIn behavior without changing their .vn files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundTransitionKind {
    #[default]
    Legacy,
    Crossfade,
    FadeThroughColor,
    SoftSlide,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CinematicTheme {
    pub transition: BackgroundTransitionKind,
    /// Total seconds for crossfade, through-color or soft-slide, 0 = instant.
    pub seconds: f32,
    /// Opaque underlay during fade-through-color (alpha must be 1.0).
    pub through_color: [f32; 4],
    /// Initial pixel offset for a soft slide on the 1280x720 art canvas.
    pub slide_pixels: f32,
    /// 0 disables optional Ken Burns motion entirely.
    pub motion_seconds: f32,
    /// End scale multiplier; restrained zoom and overscan leave no image edge.
    pub motion_zoom: f32,
    /// Normalized 1280x720 design-frame pan, bounded by overscan per axis.
    pub motion_pan: [f32; 2],
}

impl Default for CinematicTheme {
    fn default() -> Self {
        Self {
            transition: BackgroundTransitionKind::Legacy,
            seconds: 0.6,
            through_color: [0.02, 0.03, 0.06, 1.0],
            slide_pixels: 30.0,
            motion_seconds: 0.0,
            motion_zoom: 1.08,
            motion_pan: [0.015, 0.0],
        }
    }
}

/// Text reveal speed is authorable in theme TOML and can be overridden locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextRevealSpeed {
    #[default]
    Instant,
    Slow,
    Normal,
    Fast,
}
impl TextRevealSpeed {
    pub const fn rate(self) -> Option<f64> {
        match self {
            Self::Instant => None,
            Self::Slow => Some(14.0),
            Self::Normal => Some(34.0),
            Self::Fast => Some(80.0),
        }
    }
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FontSelection {
    /// Optional game-relative .ttf or .otf path; do not embed font bytes in Rust.
    pub file: Option<String>,
    /// System font family alternative when no font file is specified.
    pub family: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PanelStyle {
    pub background: [f32; 4],
    /// Game-authored highlight bar above dialogue.
    pub accent: [f32; 4],
    /// Game-authored visual division between dialogue and hints.
    pub divider: [f32; 4],
    /// Optional key into the game manifest's panel_skins table.
    pub skin: Option<String>,
    pub width_percent: f32,
    pub min_height: f32,
    pub padding: f32,
    pub gap: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TextStyle {
    pub size: f32,
    pub color: [f32; 4],
}

impl Default for PanelStyle {
    fn default() -> Self {
        Self {
            background: [0.04, 0.05, 0.09, 0.96],
            accent: [0.96, 0.75, 0.40, 1.0],
            divider: [0.70, 0.76, 0.86, 0.34],
            skin: None,
            width_percent: 90.0,
            min_height: 180.0,
            padding: 20.0,
            gap: 8.0,
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self { size: 28.0, color: [1.0, 1.0, 1.0, 1.0] }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            fade_seconds: 0.25,
            cinematic: CinematicTheme::default(),
            atmosphere: AtmosphereTheme::default(),
            feedback: FeedbackTheme::default(),
            reveal_speed: TextRevealSpeed::Instant,
            font: FontSelection::default(),
            panel: PanelStyle::default(),
            speaker: TextStyle { size: 25.0, color: [0.96, 0.75, 0.40, 1.0] },
            dialogue: TextStyle { size: 28.0, color: [1.0, 1.0, 1.0, 1.0] },
            choices: TextStyle { size: 26.0, color: [0.88, 0.94, 1.0, 1.0] },
            hint: TextStyle { size: 16.0, color: [0.75, 0.78, 0.83, 1.0] },
        }
    }
}

fn color_valid(color: [f32; 4]) -> bool {
    color.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))
}

impl Theme {
    pub fn validate(&self) -> Result<(), String> {
        if !self.ui_scale.is_finite() || !(0.5..=2.5).contains(&self.ui_scale) {
            return Err("theme.ui_scale must be between 0.5 and 2.5".into());
        }
        if !self.fade_seconds.is_finite() || !(0.0..=3.0).contains(&self.fade_seconds) {
            return Err("theme.fade_seconds must be between 0 and 3".into());
        }
        let cine = &self.cinematic;
        if !cine.seconds.is_finite() || !(0.0..=3.0).contains(&cine.seconds)
            || !color_valid(cine.through_color) || cine.through_color[3] != 1.0
            || !cine.slide_pixels.is_finite() || !(0.0..=80.0).contains(&cine.slide_pixels)
            || !cine.motion_seconds.is_finite() || !(0.0..=120.0).contains(&cine.motion_seconds)
            || !cine.motion_zoom.is_finite() || !(1.0..=1.15).contains(&cine.motion_zoom)
            || cine.motion_pan.iter().any(|p| !p.is_finite() || p.abs() > 0.04)
        {
            return Err("theme.cinematic has invalid transition, duration, zoom, pan or color".into());
        }
        self.atmosphere.validate()?;
        self.feedback.validate()?;
        let panel = &self.panel;
        if !color_valid(panel.background)
            || !color_valid(panel.accent)
            || !color_valid(panel.divider)
            || !panel.width_percent.is_finite()
            || !(20.0..=100.0).contains(&panel.width_percent)
            || !panel.min_height.is_finite()
            || !(80.0..=600.0).contains(&panel.min_height)
            || !panel.padding.is_finite()
            || !(0.0..=80.0).contains(&panel.padding)
            || !panel.gap.is_finite()
            || !(0.0..=48.0).contains(&panel.gap)
        {
            return Err("theme.panel contains an invalid color or dimension".into());
        }
        for (name, style) in [
            ("speaker", &self.speaker),
            ("dialogue", &self.dialogue),
            ("choices", &self.choices),
            ("hint", &self.hint),
        ] {
            if !style.size.is_finite()
                || !(8.0..=96.0).contains(&style.size)
                || !color_valid(style.color)
            {
                return Err(format!("theme.{name} has invalid text size or color"));
            }
        }
        if panel.skin.as_ref().is_some_and(|id| id.is_empty() || !id_valid(id)) {
            return Err("theme.panel.skin has an invalid identifier".into());
        }
        if self.font.file.is_some() && self.font.family.is_some() {
            return Err("theme.font specifies both file and family; select one".into());
        }
        if self.font.family.as_ref().is_some_and(|name| name.trim().is_empty()) {
            return Err("theme.font.family cannot be empty".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub story: Story,
    pub theme: Theme,
}

fn id_valid(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Resolve and validate paths, including symlink targets, before Bevy receives
/// an author-controlled asset path. Reject absolute and traversal components.
fn checked_file(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative.is_empty()
        || !relative_path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(format!("unsafe or invalid relative path: '{relative}'"));
    }
    let full = root.join(relative_path);
    let canonical = full
        .canonicalize()
        .map_err(|error| format!("{}: {error}", full.display()))?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(format!("path is not a regular file inside game directory: '{relative}'"));
    }
    Ok(canonical)
}

/// Validate the fixed PNG signature/IHDR and bound the image allocation
/// before handing untrusted external panel art to Bevy's asynchronous loader.
/// Full chunk decoding remains Bevy's job; a failed decode leaves the solid
/// panel visible underneath instead of blocking the render thread.
fn validate_panel_image(path: &Path, slice_border: f32) -> Result<(), String> {
    use std::io::Read;
    let mut file = fs::File::open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let mut header = [0_u8; 24];
    file.read_exact(&mut header)
        .map_err(|error| format!("invalid PNG '{}': {error}", path.display()))?;
    if &header[..8] != b"\x89PNG\r\n\x1a\n"
        || header[8..12] != [0, 0, 0, 13]
        || &header[12..16] != b"IHDR"
    {
        return Err(format!("invalid PNG signature/IHDR: {}", path.display()));
    }
    let width = u32::from_be_bytes(header[16..20].try_into().expect("four width bytes"));
    let height = u32::from_be_bytes(header[20..24].try_into().expect("four height bytes"));
    if !(32..=4096).contains(&width)
        || !(32..=4096).contains(&height)
        || u64::from(width) * u64::from(height) > 8_388_608
        || slice_border * 2.0 >= width.min(height) as f32
    {
        return Err(format!(
            "panel PNG dimensions {width}x{height} or slice border {slice_border} are unsafe"
        ));
    }
    Ok(())
}

impl Game {
    pub fn load(directory: impl AsRef<Path>) -> Result<Self, String> {
        let root = directory.as_ref().canonicalize().map_err(|error| {
            format!("{}: {error}", directory.as_ref().display())
        })?;
        if !root.is_dir() {
            return Err(format!("not a game directory: {}", root.display()));
        }
        let manifest_path = checked_file(&root, "game.toml")?;
        let text = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
        let manifest: Manifest = toml::from_str(&text)
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
        if manifest.format_version != 1 {
            return Err(format!(
                "unsupported game format version {}; expected 1",
                manifest.format_version
            ));
        }
        if !id_valid(&manifest.id) || manifest.title.trim().is_empty() {
            return Err("manifest requires valid 'id' and nonempty 'title'".into());
        }
        let script_path = checked_file(&root, &manifest.story)?;
        let source = fs::read_to_string(&script_path)
            .map_err(|error| format!("{}: {error}", script_path.display()))?;
        let story = Story::parse(&source).map_err(|diagnostics| {
            diagnostics
                .into_iter()
                .map(|diag| format!("{}:{}: {}", script_path.display(), diag.line, diag.message))
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        if !story.has_scene(&manifest.entry) {
            return Err(format!("entry scene '{}' does not exist", manifest.entry));
        }
        let theme = if let Some(path) = &manifest.theme {
            let path = checked_file(&root, path)?;
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            toml::from_str::<Theme>(&content)
                .map_err(|error| format!("{}: {error}", path.display()))?
        } else {
            Theme::default()
        };
        theme.validate()?;
        if let Some(id) = &theme.panel.skin
            && !manifest.panel_skins.contains_key(id)
        {
            return Err(format!("theme.panel.skin references unknown skin '{id}'"));
        }
        for (name, skin) in &manifest.panel_skins {
            if !id_valid(name) || name == "default" {
                return Err(format!("invalid dialogue panel skin id '{name}'"));
            }
            skin.validate().map_err(|err| format!("panel skin '{name}': {err}"))?;
            let file = checked_file(&root, &skin.image)?;
            if !file.extension().and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            {
                return Err(format!("panel skin '{name}' must be a PNG"));
            }
            validate_panel_image(&file, skin.slice_border)
                .map_err(|error| format!("panel skin '{name}': {error}"))?;
        }
        if let Some(file) = &theme.font.file {
            let path = checked_file(&root, file)?;
            if !matches!(
                path.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase).as_deref(),
                Some("ttf" | "otf")
            ) {
                return Err("theme.font.file must be a .ttf or .otf file".into());
            }
        }
        for (name, filename) in &manifest.backgrounds {
            if !id_valid(name) {
                return Err(format!("invalid background id '{name}'"));
            }
            checked_file(&root, filename)?;
        }
        for (kind, files) in [("music", &manifest.music), ("sound", &manifest.sounds)] {
            for (name, filename) in files {
                if !id_valid(name) {
                    return Err(format!("invalid {kind} identifier '{name}'"));
                }
                let file = checked_file(&root, filename)?;
                if !matches!(file.extension().and_then(|ext| ext.to_str())
                    .map(str::to_ascii_lowercase).as_deref(), Some("wav" | "ogg")) {
                    return Err(format!("{kind} '{name}' must be a .wav or .ogg file"));
                }
            }
        }
        if manifest.ambient_actors.len() > 32 {
            return Err("ambient actor count exceeds 32".into());
        }
        for (name, actor) in &manifest.ambient_actors {
            if !id_valid(name) {
                return Err(format!("invalid ambient actor id '{name}'"));
            }
            actor.validate().map_err(|error| format!("ambient actor '{name}': {error}"))?;
            if let Some(scene) = &actor.scene
                && !story.has_scene(scene)
            {
                return Err(format!("ambient actor '{name}': unknown scene '{scene}'"));
            }
            let path = checked_file(&root, &actor.image)?;
            if !path.extension().and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            {
                return Err(format!("ambient actor '{name}' requires a PNG image"));
            }
        }
        for (name, character) in &manifest.characters {
            if !id_valid(name) || character.expressions.is_empty() {
                return Err(format!("invalid character '{name}' or missing expressions"));
            }
            character.stage.validate()
                .map_err(|error| format!("character '{name}': {error}"))?;
            for (expression, filename) in &character.expressions {
                if !id_valid(expression) {
                    return Err(format!("invalid expression '{name}.{expression}'"));
                }
                checked_file(&root, filename)?;
            }
        }
        for (line, visual) in story.visual_references() {
            let missing = match visual {
                vesper_story::Visual::Background(id) => {
                    if manifest.backgrounds.contains_key(&id) { None }
                    else { Some(format!("background '{id}'")) }
                }
                vesper_story::Visual::Panel(id) => {
                    if id == "default" || manifest.panel_skins.contains_key(&id) { None }
                    else { Some(format!("panel skin '{id}'")) }
                }
                vesper_story::Visual::Show { character, expression, .. } => {
                    if manifest.characters.get(&character)
                        .is_some_and(|entry| entry.expressions.contains_key(&expression)) {
                        None
                    } else { Some(format!("character expression '{character}.{expression}'")) }
                }
                vesper_story::Visual::Music(id) => {
                    if manifest.music.contains_key(&id) { None }
                    else { Some(format!("music '{id}'")) }
                }
                vesper_story::Visual::Sound(id) => {
                    if manifest.sounds.contains_key(&id) { None }
                    else { Some(format!("sound '{id}'")) }
                }
                vesper_story::Visual::AmbientShow(id)
                | vesper_story::Visual::AmbientHide(id) => {
                    if manifest.ambient_actors.contains_key(&id) { None }
                    else { Some(format!("ambient actor '{id}'")) }
                }
                _ => None,
            };
            if let Some(reference) = missing {
                return Err(format!(
                    "{}:{line}: unknown {reference} in game manifest",
                    script_path.display()
                ));
            }
        }
        Ok(Self { root, manifest, story, theme })
    }

    pub fn music(&self, id: &str) -> Result<&str, String> {
        self.manifest.music.get(id).map(String::as_str)
            .ok_or_else(|| format!("unknown music '{id}'"))
    }

    pub fn sound(&self, id: &str) -> Result<&str, String> {
        self.manifest.sounds.get(id).map(String::as_str)
            .ok_or_else(|| format!("unknown sound '{id}'"))
    }

    pub fn panel_skin(&self, id: &str) -> Result<&PanelSkin, String> {
        self.manifest.panel_skins.get(id)
            .ok_or_else(|| format!("unknown panel skin '{id}'"))
    }

    pub fn background(&self, id: &str) -> Result<&str, String> {
        self.manifest.backgrounds.get(id).map(String::as_str)
            .ok_or_else(|| format!("unknown background '{id}'"))
    }

    pub fn character_expression(&self, id: &str, expression: &str) -> Result<&str, String> {
        self.manifest.characters.get(id)
            .ok_or_else(|| format!("unknown character '{id}'"))?
            .expressions.get(expression).map(String::as_str)
            .ok_or_else(|| format!("unknown expression '{id}.{expression}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::atomic::{AtomicU64, Ordering}, time::{SystemTime, UNIX_EPOCH}};
    static NONCE: AtomicU64 = AtomicU64::new(0);

    struct Fixture { root: PathBuf }

    impl Fixture {
        fn new(id: &str) -> Self {
            let number = NONCE.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let root = std::env::temp_dir().join(format!(
                "vesper-{id}-{}-{nanos}-{number}", std::process::id()
            ));
            fs::create_dir_all(root.join("scripts")).unwrap();
            fs::create_dir_all(root.join("assets")).unwrap();
            fs::write(root.join("scripts/story.vn"), "@scene opening\n\"Hello\"\n").unwrap();
            fs::write(root.join("assets/room.png"), b"fixture").unwrap();
            fs::write(root.join("game.toml"), format!(r#"
format_version = 1
id = "{id}"
title = "Example"
story = "scripts/story.vn"
entry = "opening"
[backgrounds]
room = "assets/room.png"
"#)).unwrap();
            Self { root }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) { fs::remove_dir_all(&self.root).unwrap(); }
    }

    #[test]
    fn independent_game_folders_load_without_compilation() {
        let first = Fixture::new("first");
        let second = Fixture::new("second");
        assert_eq!(Game::load(&first.root).unwrap().manifest.id, "first");
        assert_eq!(Game::load(&second.root).unwrap().manifest.id, "second");
    }

    #[test]
    fn rejects_parent_path_escape() {
        let sample = Fixture::new("traversal");
        let manifest = fs::read_to_string(sample.root.join("game.toml")).unwrap();
        fs::write(sample.root.join("game.toml"), manifest.replace(
            "scripts/story.vn", "../outside.vn")).unwrap();
        let error = Game::load(&sample.root).unwrap_err();
        assert!(error.contains("unsafe"), "{error}");
    }

    #[test]
    fn rejects_missing_assets_and_unknown_entry_scene() {
        let sample = Fixture::new("missing");
        fs::remove_file(sample.root.join("assets/room.png")).unwrap();
        assert!(Game::load(&sample.root).unwrap_err().contains("room.png"));
        fs::write(sample.root.join("assets/room.png"), b"fixture").unwrap();
        let manifest = fs::read_to_string(sample.root.join("game.toml")).unwrap();
        fs::write(sample.root.join("game.toml"), manifest.replace(
            "entry = \"opening\"", "entry = \"missing\"")).unwrap();
        assert!(Game::load(&sample.root).unwrap_err().contains("entry scene"));
    }

    #[test]
    fn rejects_undeclared_visual_references() {
        let sample = Fixture::new("visual-ref");
        fs::write(
            sample.root.join("scripts/story.vn"),
            "@scene opening\nbackground missing\n",
        ).unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("story.vn:2:"), "{err}");
        assert!(err.contains("background 'missing'"), "{err}");
        fs::write(
            sample.root.join("scripts/story.vn"),
            "@scene opening\nshow elena happy at center\n",
        ).unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("character expression 'elena.happy'"), "{err}");
    }

    #[test]
    fn external_theme_and_font_path_are_validated() {
        let fixture = Fixture::new("theme");
        fs::create_dir_all(fixture.root.join("styles")).unwrap();
        let config = fixture.root.join("game.toml");
        let initial = fs::read_to_string(&config).unwrap();
        fs::write(&config, initial.replace(
            "entry = \"opening\"",
            "entry = \"opening\"\ntheme = \"styles/visual.toml\"",
        )).unwrap();
        fs::write(fixture.root.join("styles/visual.toml"),
            "ui_scale = 1.4\n[font]\nfile = \"../outside.ttf\"\n").unwrap();
        let err = Game::load(&fixture.root).unwrap_err();
        assert!(err.contains("unsafe") || err.contains("font"), "{err}");
        fs::write(fixture.root.join("styles/visual.toml"),
            "ui_scale = 1.4\n[dialogue]\nsize = 36.0\ncolor = [0.7, 0.8, 0.9, 1.0]\n"
        ).unwrap();
        let loaded = Game::load(&fixture.root).unwrap();
        assert_eq!(loaded.theme.ui_scale, 1.4);
        assert_eq!(loaded.theme.dialogue.size, 36.0);
        fs::write(fixture.root.join("styles/visual.toml"), "ui_scale = 0.1\n").unwrap();
        assert!(Game::load(&fixture.root).unwrap_err().contains("ui_scale"));
    }

    #[test]
    fn atmosphere_and_feedback_are_bounded_and_legacy_optional() {
        let legacy: Theme = toml::from_str("fade_seconds = 0.2").unwrap();
        assert_eq!(legacy.atmosphere.preset, AtmospherePreset::Off);
        assert!(legacy.validate().is_ok());
        let themed: Theme = toml::from_str(
            "[atmosphere]\npreset = \"moonlit\"\nseed = 4321\ncount = 24\nspeed = 6.0\n[feedback]\nfocus_seconds = 0.1\nconfirm_seconds = 0.2\n"
        ).unwrap();
        assert_eq!(themed.atmosphere.preset, AtmospherePreset::Moonlit);
        assert_eq!(themed.atmosphere.count, 24);
        assert!(themed.validate().is_ok());
        for bad in [
            "[atmosphere]\ncount = 33",
            "[atmosphere]\nopacity = 0.95",
            "[atmosphere]\nspeed = -1.0",
            "[feedback]\nfocus_seconds = 5.0",
            "[feedback]\ncue_opacity = -0.1",
        ] {
            let parsed: Theme = toml::from_str(bad).unwrap();
            assert!(parsed.validate().is_err(), "invalid theme accepted: {bad}");
        }
        assert!(toml::from_str::<Theme>("[atmosphere]\npreset = \"unknown\"").is_err());
    }

    #[test]
    fn panel_palette_defaults_and_validation() {
        let theme: Theme = toml::from_str("[panel]\naccent = [0.72, 0.84, 1.0, 1.0]\ndivider = [0.6, 0.8, 1.0, 0.4]").unwrap();
        assert_eq!(theme.panel.accent[0], 0.72);
        assert!(theme.validate().is_ok());
        let invalid: Theme = toml::from_str("[panel]\naccent = [1.2, 0.5, 0.5, 1.0]").unwrap();
        assert!(invalid.validate().is_err());
        // Existing themes without optional tokens remain backwards compatible.
        assert!(toml::from_str::<Theme>("[panel]\nwidth_percent = 90.0")
            .unwrap().validate().is_ok());
    }

    #[test]
    fn rejects_undeclared_sound_and_invalid_transition() {
        let sample = Fixture::new("sounds");
        fs::write(sample.root.join("scripts/story.vn"), "@scene opening\nmusic night\n").unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("story.vn:2:") && err.contains("music 'night'"), "{err}");
        fs::write(sample.root.join("scripts/story.vn"), "@scene opening\nsound hello\n").unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("sound 'hello'"), "{err}");
        assert!(toml::from_str::<Theme>("fade_seconds = -1.0").unwrap().validate().is_err());
        assert!(toml::from_str::<Theme>("fade_seconds = 0.0").unwrap().validate().is_ok());
    }

    #[test]
    fn external_staging_is_optional_and_rejects_unsafe_values() {
        let fixture = Fixture::new("staging");
        let file = fixture.root.join("game.toml");
        let original = fs::read_to_string(&file).unwrap();
        let base = format!("{original}\n[characters.tess.expressions]\ncalm = \"assets/room.png\"\n");
        fs::write(&file, &base).unwrap();
        let standard = Game::load(&fixture.root).unwrap();
        assert_eq!(standard.manifest.characters["tess"].stage.scale, 1.0);
        assert_eq!(standard.manifest.characters["tess"].stage.crossfade_seconds, 0.0);

        let styled = format!("{base}\n[characters.tess.stage]\noffset_x = 24.0\nscale = 0.9\ndepth = 3.0\ncrossfade_seconds = 0.5\nidle_pixels = 2.0\n");
        fs::write(&file, &styled).unwrap();
        let loaded = Game::load(&fixture.root).unwrap();
        assert_eq!(loaded.manifest.characters["tess"].stage.offset_x, 24.0);
        assert_eq!(loaded.manifest.characters["tess"].stage.depth, 3.0);
        for bad in ["scale = 7.0", "crossfade_seconds = -1.0",
                    "depth = -2.0", "idle_pixels = 500.0",
                    "offset_x = 10000.0"] {
            fs::write(&file, format!("{base}\n[characters.tess.stage]\n{bad}\n")).unwrap();
            let error = Game::load(&fixture.root).unwrap_err();
            assert!(error.contains("stage"), "{bad}: {error}");
        }
    }

    #[test]
    fn skin_validation_scene_selection_and_default_are_safe() {
        let fixture = Fixture::new("panel-skins");
        let manifest = fixture.root.join("game.toml");
        let base = fs::read_to_string(&manifest).unwrap();
        // The general fixture uses a fake background header. Panel skins
        // require a real bounded PNG, so use the externally authored sample.
        fs::write(fixture.root.join("assets/room.png"),
            include_bytes!("../../../examples/panel-showcase/assets/panels/notebook.png"))
            .unwrap();
        let panel = "\n[panel_skins.notebook]\nimage = \"assets/room.png\"\ninsets = [36.0, 24.0, 42.0, 26.0]\nslice_border = 30.0\n";
        fs::write(&manifest, format!("{base}{panel}")).unwrap();
        let loaded = Game::load(&fixture.root).unwrap();
        assert_eq!(loaded.panel_skin("notebook").unwrap().insets[0], 36.0);
        fs::write(fixture.root.join("assets/room.png"), b"bad PNG data").unwrap();
        let error = Game::load(&fixture.root).unwrap_err();
        assert!(error.contains("invalid PNG"), "{error}");
        fs::write(fixture.root.join("assets/room.png"),
            include_bytes!("../../../examples/panel-showcase/assets/panels/notebook.png"))
            .unwrap();
        fs::write(fixture.root.join("scripts/story.vn"),
            "@scene opening\npanel notebook\n\"Writing\"\npanel default\n\"Plain\"\n").unwrap();
        assert!(Game::load(&fixture.root).is_ok());
        fs::write(fixture.root.join("scripts/story.vn"),
            "@scene opening\npanel missing\n").unwrap();
        assert!(Game::load(&fixture.root).unwrap_err().contains("panel skin 'missing'"));
        fs::write(fixture.root.join("scripts/story.vn"),
            "@scene opening\npanel notebook\n").unwrap();
        for bad in ["../escape.png", "/etc/hosts"] {
            let invalid = panel.replace("assets/room.png", bad);
            fs::write(&manifest, format!("{base}{invalid}")).unwrap();
            assert!(Game::load(&fixture.root).is_err(), "unsafe skin {bad}");
        }
        for bad in ["slice_border = -1.0", "slice_border = 700.0",
                    "insets = [-1.0, 0.0, 0.0, 0.0]"] {
            let invalid = if bad.starts_with("insets") {
                panel.replace("insets = [36.0, 24.0, 42.0, 26.0]", bad)
            } else {
                panel.replace("slice_border = 30.0", bad)
            };
            fs::write(&manifest, format!("{base}{invalid}")).unwrap();
            assert!(Game::load(&fixture.root).is_err(), "accepted invalid {bad}");
        }
        fs::write(&manifest, format!("{base}{panel}")).unwrap();
        let theme = fixture.root.join("styles/diegetic.toml");
        fs::create_dir_all(theme.parent().unwrap()).unwrap();
        fs::write(&theme, "[panel]\nskin = \"notebook\"\n").unwrap();
        let with_theme = base.replace("entry = \"opening\"",
            "entry = \"opening\"\ntheme = \"styles/diegetic.toml\"");
        fs::write(&manifest, format!("{with_theme}{panel}")).unwrap();
        assert!(Game::load(&fixture.root).is_ok());
        fs::write(&theme, "[panel]\nskin = \"missing\"\n").unwrap();
        assert!(Game::load(&fixture.root).unwrap_err().contains("unknown skin"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_outside_game_folder() {
        use std::os::unix::fs::symlink;
        let sample = Fixture::new("symlink");
        fs::remove_file(sample.root.join("assets/room.png")).unwrap();
        symlink("/etc/hosts", sample.root.join("assets/room.png")).unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("inside game directory"), "{err}");
    }

    #[test]
    fn ambient_actors_validate_external_assets_and_scene_associations() {
        let fixture = Fixture::new("ambient");
        let file = fixture.root.join("game.toml");
        let baseline = fs::read_to_string(&file).unwrap();
        let actor = r#"
[ambient_actors.bird]
image = "assets/room.png"
anchor = [0.75, 0.20]
scale = 0.55
opacity = 0.85
depth = 1.25
scene = "opening"
initially_visible = false
start_phase_seconds = 2.5
"#;
        fs::write(&file, format!("{baseline}{actor}")).unwrap();
        let loaded = Game::load(&fixture.root).unwrap();
        let bird = &loaded.manifest.ambient_actors["bird"];
        assert_eq!(bird.anchor, [0.75, 0.20]);
        assert!(!bird.initially_visible);
        assert_eq!(bird.scene.as_deref(), Some("opening"));
        assert_eq!(bird.start_phase_seconds, 2.5);
        let legacy = Fixture::new("ambient-legacy");
        assert!(Game::load(&legacy.root).is_ok());

        for (needle, replacement, expected) in [
            ("scale = 0.55", "scale = 0.0", "ambient"),
            ("depth = 1.25", "depth = 1000", "ambient"),
            ("opacity = 0.85", "opacity = 1.1", "ambient"),
            ("anchor = [0.75, 0.20]", "anchor = [1.5, 0.20]", "ambient"),
            ("scene = \"opening\"", "scene = \"absent\"", "unknown scene"),
            ("image = \"assets/room.png\"", "image = \"../outside.png\"", "unsafe"),
        ] {
            fs::write(&file, format!("{baseline}{}", actor.replace(needle, replacement))).unwrap();
            let err = Game::load(&fixture.root).unwrap_err();
            assert!(err.contains(expected), "{needle}: {err}");
        }
        let many = (0..33).map(|index| format!(
            "[ambient_actors.a{index}]\nimage = \"assets/room.png\"\n"
        )).collect::<Vec<_>>().join("");
        fs::write(&file, format!("{baseline}{many}")).unwrap();
        assert!(Game::load(&fixture.root).unwrap_err().contains("count"));
    }


}
