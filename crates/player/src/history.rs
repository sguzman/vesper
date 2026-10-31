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
    pub just_closed: bool,
    /// A fresh overlay starts at the most recent page only once the layout
    /// has resolved the true scroll extent (never a fake 100000px offset).
    snap_to_latest: bool,
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
        if let Some(Output::Dialogue { speaker, text }
            | Output::RichDialogue { speaker, text, .. }) = last {
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
                ScrollPosition(Vec2::ZERO),
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
    history.snap_to_latest = true;
    info!("Vesper reading history opened: {} entries",history.entries.len());
}

pub fn close(history: &mut HistoryLog, commands: &mut Commands) {
    if let Some(overlay)=history.overlay.take() {
        commands.entity(overlay).despawn();
        history.just_closed = true;
        history.snap_to_latest = false;
        info!("Vesper reading history closed");
    }
}

/// Bevy stores the requested scroll offset, which can exceed the rendered
/// content extent. Clamp against the actual laid-out scroll region instead
/// of saturating a made-up large offset.
fn scroll_max(node: &ComputedNode) -> f32 {
    ((node.content_size().y - node.size().y).max(0.0)
        * node.inverse_scale_factor).max(0.0)
}

fn scroll_by(
    delta: f32,
    scroll: &mut Query<(&mut ScrollPosition, &ComputedNode), With<HistoryScroll>>,
) {
    for (mut position, computed) in scroll.iter_mut() {
        let limit = scroll_max(computed);
        let next = (position.0.y + delta).clamp(0.0, limit);
        if (next - position.0.y).abs() >= 0.1 {
            position.0.y = next;
            info!("Vesper reading history scrolled: {:.0} / {:.0}", next, limit);
        }
    }
}

/// Initialize after Bevy measures the actual text and clips its viewport.
/// The initial offset must never get stuck at an unreachable 100000 pixels.
fn snap_latest(
    history: &mut HistoryLog,
    scroll: &mut Query<(&mut ScrollPosition, &ComputedNode), With<HistoryScroll>>,
) {
    if !history.snap_to_latest { return; }
    for (mut position, computed) in scroll.iter_mut() {
        if computed.size().y <= 0.0 || computed.content_size().y <= 0.0 { continue; }
        position.0.y = scroll_max(computed);
        history.snap_to_latest = false;
    }
}

pub fn input(
    key: &ButtonInput<KeyCode>,
    history: &mut HistoryLog,
    clicks: &mut HistoryClick,
    scroll: &mut Query<(&mut ScrollPosition, &ComputedNode), With<HistoryScroll>>,
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
    snap_latest(history, scroll);
    let delta=if key.just_pressed(KeyCode::PageUp)
        || matches!(pointer,Some(HistoryAction::Older)) { -320.0 }
        else if key.just_pressed(KeyCode::PageDown)
        || matches!(pointer,Some(HistoryAction::Newer)) { 320.0 }
        else if key.just_pressed(KeyCode::ArrowUp) { -70.0 }
        else if key.just_pressed(KeyCode::ArrowDown) { 70.0 }
        else if key.just_pressed(KeyCode::Home) { -f32::MAX }
        else if key.just_pressed(KeyCode::End) { f32::MAX }
        else { 0.0 };
    if delta != 0.0 { scroll_by(delta, scroll); }
}

/// Ordinary mouse wheels and trackpads control history directly; no pointer
/// hovering a particular text row or owning keyboard focus is required.
pub fn wheel(
    mut events: MessageReader<bevy::input::mouse::MouseWheel>,
    session: Res<Session>,
    mut scroll: Query<(&mut ScrollPosition, &ComputedNode), With<HistoryScroll>>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    let mut delta = 0.0;
    for event in events.read() {
        delta -= event.y * match event.unit {
            MouseScrollUnit::Line => 65.0,
            MouseScrollUnit::Pixel => 1.0,
        };
    }
    if session.history.visible() && delta != 0.0 {
        scroll_by(delta, &mut scroll);
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
    fn history_scroll_offset_tracks_real_content_bounds() {
        let mut node = ComputedNode::default();
        node.size = Vec2::new(800.0, 340.0);
        node.content_size = Vec2::new(800.0, 1410.0);
        node.inverse_scale_factor = 1.0;
        assert_eq!(scroll_max(&node), 1070.0);
        node.inverse_scale_factor = 0.5;
        assert_eq!(scroll_max(&node), 535.0);
        node.content_size.y = 240.0;
        assert_eq!(scroll_max(&node), 0.0);
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
