//! Current-session, interpreter-fed dialogue history. No future scene text is
//! scanned, no history enters version-1 saves, and no file I/O occurs here.
use super::*;
use std::collections::VecDeque;
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_ENTRIES: usize = 80;
pub const MAX_GRAPHEMES: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryEntry {
    Scene(String),
    Dialogue { speaker: String, text: String },
}

#[derive(Default)]
pub struct HistoryLog {
    pub entries: VecDeque<HistoryEntry>,
    last_scene: Option<String>,
    pub overlay: Option<Entity>,
}

fn bound(text: &str) -> String {
    let mut graphemes = text.graphemes(true);
    let prefix: String = graphemes.by_ref().take(MAX_GRAPHEMES).collect();
    if graphemes.next().is_some() { format!("{prefix}…") } else { prefix }
}

impl HistoryLog {
    pub fn record(&mut self, scene: &str, speaker: &str, text: &str) {
        if self.last_scene.as_deref() != Some(scene) {
            self.entries.push_back(HistoryEntry::Scene(bound(scene)));
            self.last_scene = Some(scene.to_owned());
        }
        self.entries.push_back(HistoryEntry::Dialogue {
            speaker: bound(speaker),
            text: bound(text),
        });
        while self.entries.len() > MAX_ENTRIES {
            self.entries.pop_front();
        }
    }

    pub fn reset(&mut self) {
        self.entries.clear();
        self.last_scene = None;
    }

    pub fn reset_to_current(&mut self, scene: &str, last: Option<&Output>) {
        // Loading a prior branch must never retain later dialogue in history.
        self.reset();
        if let Some(Output::Dialogue { speaker, text }) = last {
            self.record(scene, speaker.as_deref().unwrap_or(""), text);
        }
    }

    pub fn visible(&self) -> bool { self.overlay.is_some() }
}

#[derive(Component)]
pub struct HistoryScroll;

#[derive(Clone, Copy)]
pub enum HistoryAction { Older, Newer, Close }

#[derive(Resource, Default)]
pub struct HistoryClick(pub Option<HistoryAction>);

fn control(
    parent: &mut ChildSpawnerCommands, label: &str, action: HistoryAction,
    font: &FontSource, accent: [f32; 4],
) {
    parent.spawn((
        Name::new(format!("History control: {label}")),
        Button,
        Hovered::default(),
        observe(move |_activate: On<Activate>, mut clicks: ResMut<HistoryClick>| {
            clicks.0 = Some(action);
        }),
        Text::new(label),
        TextFont { font: font.clone(), font_size: FontSize::Px(18.0), ..default() },
        TextColor(Color::WHITE),
        Node { padding: px(9.0).all(), ..default() },
        BackgroundColor(Color::srgba(accent[0] * 0.22,
            accent[1] * 0.22, accent[2] * 0.22, 0.9)),
    ));
}

pub fn open(
    history: &mut HistoryLog,
    commands: &mut Commands,
    assets: &AssetServer,
    theme: &vesper_game::Theme,
) {
    if history.overlay.is_some() { return; }
    let font = text_font(theme, assets);
    let accent = theme.panel.accent;
    let overlay=commands.spawn((
        Name::new("VX08 reading history overlay"),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.0), height: percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.01, 0.015, 0.035, 0.90)),
    )).with_children(|screen| {
        screen.spawn((
            Name::new("Reading history panel"),
            Node {
                width: percent(90.0), max_width: px(900.0),
                height: percent(86.0),
                padding: px(16.0).all(),
                row_gap: px(10.0), flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.045, 0.065, 0.11, 0.98)),
        )).with_children(|panel| {
            panel.spawn((
                Name::new("History title"),
                Text::new("Reading history"),
                TextFont { font: font.clone(), font_size: FontSize::Px(26.0),
                    ..default() },
                TextColor(Color::srgba(accent[0], accent[1], accent[2], 1.0)),
            ));
            panel.spawn((
                Name::new("History scroll"),
                HistoryScroll,
                ScrollPosition(Vec2::new(0.0, 100000.0)),
                Node {
                    width: percent(100.0), flex_grow: 1.0, min_height: px(0.0),
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(7.0),
                    ..default()
                },
            )).with_children(|list| {
                if history.entries.is_empty() {
                    list.spawn((
                        Text::new("No dialogue has been read in this session."),
                        TextFont { font: font.clone(), font_size: FontSize::Px(18.0),
                            ..default() }, TextColor(Color::WHITE),
                    ));
                }
                for entry in &history.entries {
                    match entry {
                        HistoryEntry::Scene(scene) => {
                            list.spawn((
                                Name::new("History scene boundary"),
                                Text::new(format!("◆ {scene}")),
                                TextFont { font: font.clone(),
                                    font_size: FontSize::Px(17.0), ..default() },
                                TextColor(Color::srgba(accent[0],accent[1],accent[2],1.0)),
                            ));
                        }
                        HistoryEntry::Dialogue {speaker,text} => {
                            let label=if speaker.is_empty() {
                                text.clone()
                            } else {
                                format!("{speaker}: {text}")
                            };
                            list.spawn((
                                Name::new("History dialogue line"),
                                Text::new(label),
                                TextFont { font: font.clone(),
                                    font_size: FontSize::Px(theme.dialogue.size.min(25.0)),
                                    ..default() },
                                TextColor(Color::WHITE),
                                TextLayout::new(Justify::Left,LineBreak::WordOrCharacter),
                                Node {width:percent(100.0),flex_shrink:0.0,..default()},
                            ));
                        }
                    }
                }
            });
            panel.spawn((
                Node {
                    width: percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(10.0),
                    ..default()
                },
            )).with_children(|controls| {
                control(controls, "↑ Older", HistoryAction::Older, &font, accent);
                control(controls, "↓ Newer", HistoryAction::Newer, &font, accent);
                control(controls, "Close (H / Esc)", HistoryAction::Close, &font, accent);
            });
        });
    }).id();
    history.overlay=Some(overlay);
    info!("Vesper reading history opened: {} entries",history.entries.len());
}

pub fn close(history: &mut HistoryLog, commands: &mut Commands) {
    if let Some(overlay)=history.overlay.take() {
        commands.entity(overlay).despawn();
        info!("Vesper reading history closed");
    }
}

pub fn input(
    key: &ButtonInput<KeyCode>,
    history: &mut HistoryLog,
    clicks: &mut HistoryClick,
    scroll: &mut Query<&mut ScrollPosition, With<HistoryScroll>>,
    commands: &mut Commands,
) {
    let pointer=clicks.0.take();
    if key.just_pressed(KeyCode::Escape)
        || key.just_pressed(KeyCode::KeyH)
        || matches!(pointer,Some(HistoryAction::Close))
    {
        close(history,commands);
        return;
    }
    let delta=if key.just_pressed(KeyCode::PageUp)
        || matches!(pointer,Some(HistoryAction::Older)) { -320.0 }
        else if key.just_pressed(KeyCode::PageDown)
        || matches!(pointer,Some(HistoryAction::Newer)) { 320.0 }
        else if key.just_pressed(KeyCode::ArrowUp) { -70.0 }
        else if key.just_pressed(KeyCode::ArrowDown) { 70.0 }
        else { 0.0 };
    if delta != 0.0 {
        for mut position in scroll.iter_mut() {
            position.0.y = (position.0.y + delta).clamp(0.0, 100000.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_records_only_emitted_dialogue_with_scene_boundaries() {
        let mut log=HistoryLog::default();
        log.record("opening","Alice","Welcome.");
        log.record("opening","Bob","Hello.");
        log.record("other","","Choices were made.");
        assert_eq!(log.entries.len(),5);
        assert_eq!(log.entries.front(),Some(&HistoryEntry::Scene("opening".into())));
        assert_eq!(log.entries.back(),Some(&HistoryEntry::Dialogue {
            speaker:"".into(),text:"Choices were made.".into()
        }));
    }

    #[test]
    fn history_is_bounded_by_entries_and_grapheme_count() {
        let mut log=HistoryLog::default();
        for i in 0..300 {
            log.record("long","", &format!("{i}"));
        }
        assert_eq!(log.entries.len(),MAX_ENTRIES);
        let s="👩‍💻".repeat(MAX_GRAPHEMES+20);
        assert_eq!(bound(&s).graphemes(true).count(),MAX_GRAPHEMES+1);
    }

    #[test]
    fn save_restore_clears_unread_future_branch_lines() {
        let mut log=HistoryLog::default();
        log.record("future","Alex","Future line");
        log.reset_to_current("past",Some(&Output::Dialogue {
            speaker:None,text:"Earlier.".into()
        }));
        assert_eq!(log.entries.len(),2);
        assert!(!format!("{:?}",log.entries).contains("Future"));
    }
}
