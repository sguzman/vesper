//! Development-only content watcher. All directory scans and parsing run on
//! a worker thread. The Bevy update system only consumes completed results.
use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{mpsc, Mutex},
    thread,
    time::{Duration, SystemTime},
};

#[derive(Resource)]
pub struct ReloadInbox(Mutex<mpsc::Receiver<Result<Game, String>>>);

/// Stamp all textual authoring files, including newly created .vn and .toml
/// files. Never follow symlinked directories or scan other game installations.
fn stamps(root: &Path) -> Vec<(PathBuf, u64, Option<SystemTime>)> {
    let mut list = Vec::new();
    let mut folders = vec![(root.to_path_buf(), 0_u8)];
    while let Some((folder, depth)) = folders.pop() {
        if depth > 12 || list.len() > 8_192 { break; }
        let Ok(entries) = fs::read_dir(&folder) else { continue };
        for entry in entries.flatten() {
            let Ok(typ) = entry.file_type() else { continue };
            if typ.is_dir() {
                folders.push((entry.path(), depth + 1));
            } else if typ.is_file() && entry.path().extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| matches!(ext, "vn" | "toml"))
            {
                if let Ok(meta) = entry.metadata() {
                    list.push((entry.path(), meta.len(), meta.modified().ok()));
                }
            }
        }
    }
    list.sort_by(|a, b| a.0.cmp(&b.0));
    list
}

pub fn start(root: PathBuf) -> ReloadInbox {
    let (tx, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut old = stamps(&root);
        loop {
            thread::sleep(Duration::from_millis(750));
            let new = stamps(&root);
            if new == old { continue; }
            old = new;
            if tx.send(Game::load(&root)).is_err() { break; }
        }
    });
    ReloadInbox(Mutex::new(receiver))
}

/// Apply only validated, narrative-compatible changes. On failure the last
/// good runtime story, assets and UI remain intact, with a logged diagnostic.
pub fn apply_reload(
    inbox: Res<ReloadInbox>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
    mut panels: Query<(&mut Node, &mut BackgroundColor), With<StoryPanel>>,
    mut accents: Query<&mut BackgroundColor, (With<StoryAccent>, Without<StorySeparator>, Without<StoryPanel>)>,
    mut dividers: Query<&mut BackgroundColor, (With<StorySeparator>, Without<StoryAccent>, Without<StoryPanel>)>,
    mut texts: Query<(&mut Text, &mut TextFont, &mut TextColor, &StoryText)>,
    mut reading: ResMut<readout::ReadingState>,
) {
    let results = match inbox.0.lock() {
        Ok(recv) => recv.try_iter().collect::<Vec<_>>(),
        Err(_) => return,
    };
    if results.is_empty() { return; }

    // Only attempt the newest observed revision. All I/O is already complete.
    let Some(result) = results.into_iter().last() else { return };
    let updated = match result {
        Ok(game) => game,
        Err(error) => {
            warn!("Vesper hot reload rejected: {error}");
            return;
        }
    };

    let rebound = match session.machine.rebind(&updated.story) {
        Ok(machine) => machine,
        Err(error) => {
            warn!("Vesper hot reload rejected: {error}");
            return;
        }
    };
    // Any live asset that is removed must not leave a partially updated scene.
    if let Some(id) = &session.presentation.panel_skin {
        if let Err(error) = updated.panel_skin(id) {
            warn!("Vesper hot reload rejected: {error}");
            return;
        }
    }
    if let Some(id) = &session.presentation.background {
        if let Err(error) = updated.background(id) {
            warn!("Vesper hot reload rejected: {error}");
            return;
        }
    }
    for (id, pose) in &session.presentation.characters {
        if let Err(error) = updated.character_expression(id, &pose.expression) {
            warn!("Vesper hot reload rejected: {error}");
            return;
        }
    }
    for id in session.presentation.ambient_visibility.keys() {
        if !updated.manifest.ambient_actors.contains_key(id) {
            warn!("Vesper hot reload rejected: active ambient actor '{id}' was removed");
            return;
        }
    }
    if let Some(id) = &session.presentation.music {
        if let Err(error) = updated.music(id) {
            warn!("Vesper hot reload rejected: {error}");
            return;
        }
    }

    let old = session.game.clone();
    let saved = session.presentation.clone();
    session.game = updated;
    session.machine = rebound;
    let theme = effective_theme(&session);
    ui_scale.0 = theme.ui_scale;

    // Preserve existing entities and handles unless their declared paths
    // changed. Bevy's file watcher handles image/font file changes in place.
    if let Some(id) = saved.background {
        if old.background(&id).ok() != session.game.background(&id).ok() {
            let _ = apply_visual(&mut session, &mut commands, &assets, Visual::Background(id));
        }
    }
    for (id, pose) in saved.characters {
        let path_changed = old.character_expression(&id, &pose.expression).ok()
            != session.game.character_expression(&id, &pose.expression).ok();
        let depth_changed = old.manifest.characters.get(&id).map(|actor| actor.stage.depth)
            != session.game.manifest.characters.get(&id).map(|actor| actor.stage.depth);
        if path_changed || depth_changed {
            // Recreate the actor when its layer changes. Mutating an existing
            // Transform's Z alone can leave its prior sprite draw order visible
            // after a live reload; new sprites reliably receive their layer.
            // This changes only presentation, never the re-bound story machine.
            let result = apply_visual(&mut session, &mut commands, &assets,
                Visual::Show {
                    character: id.clone(),
                    expression: pose.expression,
                    position: pose.position,
                });
            if let Err(error) = result {
                warn!("Vesper actor reload failed for '{id}': {error}");
            }
        }
    }
    if let Some(id) = saved.music {
        if old.music(&id).ok() != session.game.music(&id).ok() {
            let _ = apply_visual(&mut session, &mut commands, &assets, Visual::Music(id));
        }
    }

    let viewport = windows.single().map(|window|
        Vec2::new(window.resolution.width(), window.resolution.height()))
        .unwrap_or(Vec2::new(1280.0, 720.0));
    for (mut node, mut background) in &mut panels {
        let p = &theme.panel;
        node.left = percent((100. - p.width_percent) / 2.);
        node.width = percent(p.width_percent);
        node.min_height = px(p.min_height);
        node.padding = panel_art::skin_padding(&session, viewport)
            .unwrap_or_else(|| px(p.padding).all());
        node.row_gap = px(p.gap);
        *background = BackgroundColor(Color::srgba(
            p.background[0], p.background[1], p.background[2], p.background[3]
        ));
    }

    for mut color in &mut accents {
        let c = theme.panel.accent;
        *color = BackgroundColor(Color::srgba(c[0], c[1], c[2], c[3]));
    }
    for mut color in &mut dividers {
        let c = theme.panel.divider;
        *color = BackgroundColor(Color::srgba(c[0], c[1], c[2], c[3]));
    }

    let font = text_font(&theme, &assets);
    let last = session.machine.last_output().cloned();
    let screen = match last {
        Some(Output::Dialogue { speaker, text }) =>
            Screen::normal(speaker.unwrap_or_default(), text),
        Some(Output::RichDialogue { speaker, text, spans }) =>
            Screen::rich(speaker.unwrap_or_default(), text, spans),
        Some(Output::Choices(choices)) =>
            Screen::choices(&choices, session.presentation.selected_choice),
        Some(Output::End) => Screen::ending("The End".into()),
        _ => Screen::ending("Story reloaded".into()),
    };
    // Hot reload preserves the already-presented line as complete, even if
    // its text changes. Never replay a partial line or reexecute the machine.
    reading.restore_completed_styled(
        screen.body.clone(), screen.animated, &screen.spans,
    );
    for (mut text, mut text_font, mut color, slot) in &mut texts {
        text.0 = if matches!(slot, StoryText::Body) && !screen.spans.is_empty() {
            String::new()
        } else {
            screen.part(slot).to_owned()
        };
        let style = match slot {
            StoryText::Speaker => &theme.speaker,
            StoryText::Body if screen.choices => &theme.choices,
            StoryText::Body => &theme.dialogue,
            StoryText::Hint => &theme.hint,
        };
        text_font.font = font.clone();
        text_font.font_size = FontSize::Px(style.size);
        *color = text_color(style.color);
    }
    info!("Vesper hot reload applied");
}
