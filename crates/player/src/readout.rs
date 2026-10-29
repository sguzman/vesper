//! Runtime text pacing. Only the bounded grapheme boundary table is built
//! when a new dialogue arrives; the frame loop uses a prefix and clock.
use super::*;
use crate::text_reveal::{RevealBuffer, RevealSpeed};

#[derive(Component)]
pub struct ContinueCue;

#[derive(Clone, Debug)]
struct StyledSegment {
    from: usize,
    to: usize,
    style: Option<String>,
}

#[derive(Component)]
pub struct InlineUiSpan {
    line_id: u64,
    index: usize,
}

#[derive(Resource, Default)]
pub struct ReadingState {
    buffer: Option<RevealBuffer>,
    showing_dialogue: bool,
    auto_elapsed: f32,
    pub autoplay_ready: bool,
    dirty: bool,
    /// A single bounded segment plan is built when the story line changes.
    segments: Vec<StyledSegment>,
    line_id: u64,
}

pub fn speed(session: &Session) -> RevealSpeed {
    if session.preferences.reduced_motion.unwrap_or(false) {
        RevealSpeed::Instant
    } else {
        session.preferences.reveal_speed.unwrap_or(session.game.theme.reveal_speed)
    }
}

impl ReadingState {
    /// A new story line starts its own progression; no execution of .vn is
    /// performed here. Existing save schema remains untouched.
    pub fn begin(&mut self, body: String, dialogue: bool, speed: RevealSpeed) -> &str {
        self.segments.clear();
        self.line_id = self.line_id.wrapping_add(1);
        self.showing_dialogue = dialogue;
        self.buffer = Some(RevealBuffer::new(
            body, if dialogue { speed } else { RevealSpeed::Instant }
        ));
        self.auto_elapsed = 0.0;
        self.autoplay_ready = false;
        self.dirty = true;
        self.buffer.as_ref().map_or("", RevealBuffer::visible_text)
    }

    pub fn begin_styled(
        &mut self, body: String, dialogue: bool, speed: RevealSpeed,
        spans: &[InlineSpan],
    ) -> &str {
        // All author offsets have already passed UTF-8/grapheme validation.
        // Build the complete presentation plan once, never per frame.
        let mut cursor = 0;
        let mut plan = Vec::with_capacity(spans.len() * 2 + 1);
        for span in spans {
            if cursor < span.start {
                plan.push(StyledSegment {
                    from: cursor, to: span.start, style: None,
                });
            }
            plan.push(StyledSegment {
                from: span.start, to: span.end, style: Some(span.style.clone()),
            });
            cursor = span.end;
        }
        if !spans.is_empty() && cursor < body.len() {
            plan.push(StyledSegment { from: cursor, to: body.len(), style: None });
        }
        self.begin(body, dialogue, speed);
        self.segments = plan;
        self.buffer.as_ref().map_or("", RevealBuffer::visible_text)
    }

    pub fn restore_completed_styled(
        &mut self, body: String, dialogue: bool, spans: &[InlineSpan],
    ) {
        self.begin_styled(body, dialogue, RevealSpeed::Instant, spans);
    }

    /// A reloaded/loaded scene is already visible: never replay partial text.
    pub fn restore_completed(&mut self, body: String, dialogue: bool) {
        self.begin(body, dialogue, RevealSpeed::Instant);
    }

    /// Enter/Space while revealing finishes only this line; the story machine
    /// is intentionally not advanced until a subsequent separate press.
    pub fn complete_on_input(&mut self) -> bool {
        let was_partial = self.buffer.as_mut().is_some_and(RevealBuffer::complete_now);
        if was_partial {
            self.dirty = true;
            self.auto_elapsed = 0.0;
            self.autoplay_ready = false;
            info!("Vesper reveal completed by reader");
        }
        was_partial
    }

    pub fn full_body(&self) -> Option<&str> {
        self.buffer.as_ref().map(RevealBuffer::visible_text)
    }
}

/// Keep the same single UI text block for wrapping while assigning each
/// authored section its own TextSpan color/font. No fonts or markup are parsed
/// during this update system; at most 129 already-validated sections exist.
pub fn sync_spans(
    mut commands: Commands,
    session: Res<Session>,
    state: Res<ReadingState>,
    assets: Res<AssetServer>,
    mut body: Query<(Entity, &StoryText, &mut Text)>,
    mut children: Query<(Entity, &InlineUiSpan, &mut TextSpan, &mut TextFont, &mut TextColor)>,
    mut previous: Local<Option<(u64, usize, bool)>>,
) {
    let text = state.full_body().unwrap_or("");
    let color_mode = session.preferences.high_contrast.unwrap_or(false);
    let marker = (state.line_id, text.len(), color_mode);
    if *previous == Some(marker) && !session.is_changed() { return; }
    let new_line = previous.is_none_or(|old| old.0 != state.line_id);
    let theme = effective_theme(&session);
    let root = body.iter_mut().find_map(|(id, slot, mut text)| {
        if matches!(slot, StoryText::Body) {
            if !state.segments.is_empty() && !text.0.is_empty() { text.0.clear(); }
            Some(id)
        } else { None }
    });
    let Some(root) = root else { return; };
    if new_line {
        for (entity, _, _, _, _) in &mut children {
            commands.entity(entity).despawn();
        }
        if !state.segments.is_empty() {
            let default_font = text_font(&theme, &assets);
            commands.entity(root).with_children(|parent| {
                for (index, segment) in state.segments.iter().enumerate() {
                    let upto = text.len().min(segment.to);
                    let visible = if upto > segment.from { &text[segment.from..upto] } else { "" };
                    let styled = segment.style.as_ref()
                        .and_then(|name| theme.inline_styles.get(name));
                    let palette = if color_mode { theme.dialogue.color }
                        else { styled.map_or(theme.dialogue.color, |style| style.color) };
                    let font = if !color_mode && session.preferences.font_family.is_none() {
                        styled.and_then(|style| style.font_family.as_ref())
                            .map_or_else(|| default_font.clone(),
                                |family| FontSource::Family(family.clone().into()))
                    } else { default_font.clone() };
                    parent.spawn((
                        InlineUiSpan { line_id: state.line_id, index },
                        TextSpan::new(visible),
                        TextFont { font, font_size: FontSize::Px(theme.dialogue.size), ..default() },
                        text_color(palette),
                    ));
                }
            });
            info!("Vesper inline text spans mounted: {}", state.segments.len());
        }
    } else {
        for (_, node, mut span, mut font, mut color) in &mut children {
            if node.line_id != state.line_id { continue; }
            let Some(segment) = state.segments.get(node.index) else { continue };
            let upto = text.len().min(segment.to);
            let visible = if upto > segment.from { &text[segment.from..upto] } else { "" };
            if span.0 != visible { span.0.clear(); span.0.push_str(visible); }
            if session.is_changed() {
                let styled = segment.style.as_ref()
                    .and_then(|name| theme.inline_styles.get(name));
                let palette = if color_mode { theme.dialogue.color }
                    else { styled.map_or(theme.dialogue.color, |style| style.color) };
                *color = text_color(palette);
                let default_font = text_font(&theme, &assets);
                font.font = if !color_mode && session.preferences.font_family.is_none() {
                    styled.and_then(|style| style.font_family.as_ref())
                        .map_or(default_font, |family| FontSource::Family(family.clone().into()))
                } else { default_font };
                font.font_size = FontSize::Px(theme.dialogue.size);
            }
        }
    }
    *previous = Some(marker);
}

/// Guarded by preference validation; deterministic for simulated-clock tests.
fn auto_delay_elapsed(elapsed: f32, delay: f32) -> bool {
    elapsed >= delay && delay.is_finite() && (0.5..=20.0).contains(&delay)
}

pub fn tick(
    time: Res<Time>,
    session: Res<Session>,
    settings: Res<settings::SettingsState>,
    saves: Res<save_slots::SaveUi>,
    mut state: ResMut<ReadingState>,
    mut previous_delay: Local<Option<f32>>,
    mut bodies: Query<(&StoryText, &mut Text), Without<ContinueCue>>,
    mut cues: Query<(&mut Node, &mut TextColor, &mut TextFont), With<ContinueCue>>,
    assets: Res<AssetServer>,
) {
    let active = !session.paused && session.menu_overlay.is_none()
        && settings.overlay.is_none() && !saves.visible() && !session.history.visible()
        && !session.finished && !session.choosing;
    let chosen_speed = speed(&session);
    let mut changed = false;
    let mut complete = false;
    let showing_dialogue = state.showing_dialogue;
    if let Some(buffer) = &mut state.buffer {
        let was_complete = buffer.is_complete();
        buffer.set_speed(chosen_speed);
        if !was_complete && buffer.is_complete() { changed = true; }
        if active { changed |= buffer.advance(time.delta_secs()); }
        complete = buffer.is_complete();
    }

    if changed || state.dirty {
        state.dirty = false;
        if let Some(text) = state.full_body().map(str::to_owned) {
            for (slot, mut body) in &mut bodies {
                if matches!(slot, StoryText::Body) {
                    let replacement = if state.segments.is_empty() { text.as_str() } else { "" };
                    if body.0 != replacement {
                        body.0.clear();
                        body.0.push_str(replacement);
                    }
                }
            }
        }
    }

    let cue_active = active && showing_dialogue && complete;
    let delay = session.preferences.auto_advance_delay.unwrap_or(1.5);
    // A changed wait period always begins a fresh countdown rather than
    // unexpectedly skipping the current line when a reader shortens it.
    if *previous_delay != Some(delay) {
        *previous_delay = Some(delay);
        state.auto_elapsed = 0.0;
        state.autoplay_ready = false;
    }
    if cue_active && session.preferences.auto_advance.unwrap_or(false) {
        state.auto_elapsed = (state.auto_elapsed + time.delta_secs()).min(60.0);
        if auto_delay_elapsed(state.auto_elapsed, delay) && !state.autoplay_ready {
            state.autoplay_ready = true;
            info!("Vesper auto-advance ready");
        }
    } else {
        state.auto_elapsed = 0.0;
        state.autoplay_ready = false;
    }
    let theme = effective_theme(&session);
    // The cue uses the authored panel accent, separate from the keyboard hint.
    let base_alpha = theme.panel.accent[3];
    let pulse = if session.preferences.reduced_motion.unwrap_or(false)
        || chosen_speed == RevealSpeed::Instant {
        1.0
    } else {
        0.70 + 0.30 * (time.elapsed_secs() * 2.5).sin().abs()
    };
    for (mut node, mut color, mut font) in &mut cues {
        let display = if cue_active { Display::Flex } else { Display::None };
        if node.display != display { node.display = display; }
        if cue_active {
            let [r,g,b,_] = theme.panel.accent;
            let next = Color::srgba(r,g,b,base_alpha * pulse);
            if color.0 != next { color.0 = next; }
            if font.font_size != FontSize::Px(theme.hint.size) {
                font.font_size = FontSize::Px(theme.hint.size);
            }
            // Only rescan a font source after a player preference or theme edit.
            if session.is_changed() { font.font = text_font(&theme, &assets); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configurable_auto_delay_exact_boundary_and_legacy_default() {
        assert!(!auto_delay_elapsed(1.49, 1.5));
        assert!(auto_delay_elapsed(1.5, 1.5));
        assert!(!auto_delay_elapsed(4.49, 4.5));
        assert!(auto_delay_elapsed(4.5, 4.5));
        assert!(auto_delay_elapsed(20.0, 20.0));
        assert!(!auto_delay_elapsed(2.0, f32::NAN));
        assert!(!auto_delay_elapsed(3.0, 0.0));
    }
    #[test]
    fn restore_is_immediate_and_skip_does_not_execute_narrative() {
        let mut state = ReadingState::default();
        assert_eq!(state.begin("👩‍💻 Stars".to_owned(), true, RevealSpeed::Slow), "");
        assert!(state.complete_on_input());
        assert!(!state.complete_on_input());
        assert_eq!(state.full_body(), Some("👩‍💻 Stars"));
        state.restore_completed("restored ☀️".into(), true);
        assert_eq!(state.full_body(), Some("restored ☀️"));
        assert!(!state.complete_on_input());
    }
}
