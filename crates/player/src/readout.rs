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

/// Bevy's UI root Text must carry the leading unstyled section. The
/// remaining styled sections live as child TextSpans, retaining a single
/// wrapping/shaping block. A zero-width prefix supports a styled first word.
fn rich_root_prefix<'a>(visible: &'a str, segments: &[StyledSegment]) -> &'a str {
    if let Some(first) = segments.first()
        && first.style.is_none() && first.from == 0
    {
        return &visible[..visible.len().min(first.to)];
    }
    // Bevy measures child TextSpans only after its parent establishes a text
    // block. A zero-width anchor ensures a leading styled word is measurable
    // without inserting visible whitespace or changing story/history text.
    if !segments.is_empty() { return "\u{200B}"; }
    ""
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
            if !state.segments.is_empty() {
                let prefix = rich_root_prefix(state.full_body().unwrap_or(""), &state.segments);
                if text.0 != prefix {
                    text.0.clear();
                    text.0.push_str(prefix);
                }
            }
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
                // The leading unstyled segment is held by the root Text,
                // not duplicated as a child of that same text block.
                let first_is_root = state.segments.first()
                    .is_some_and(|seg| seg.style.is_none() && seg.from == 0);
                for (index, segment) in state.segments.iter().enumerate()
                    .skip(usize::from(first_is_root))
                {
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
            info!("Vesper inline text spans mounted: {} (root prefix bytes: {})",
                state.segments.len(), rich_root_prefix(text, &state.segments).len());
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

/// A fixed-cost color-only flourish. It never changes opacity of the panel,
/// glyph geometry, the readable source string, or span/grapheme boundaries.
/// Disabled for high contrast; reduced motion freezes the effect at t=0.
pub fn effect_palette(
    base: [f32; 4], effect: vesper_game::InlineEffect,
    amplitude: f32, rate_hz: f32, seconds: f32, index: usize,
) -> [f32; 4] {
    use std::f32::consts::TAU;
    use vesper_game::InlineEffect;
    if effect == InlineEffect::Static || amplitude == 0.0 || rate_hz == 0.0 {
        return base;
    }
    let phase = seconds.max(0.0) * rate_hz * TAU + index as f32 * 1.618;
    let (a, b) = (phase.sin(), (phase * 2.7 + 0.83).sin());
    let mut color = base;
    match effect {
        InlineEffect::Ember => {
            // Warm, nonrandom double-frequency flame shimmer.
            let flicker = (a * 0.72 + b * 0.28) * amplitude;
            color[0] = (base[0] * (1.0 + flicker)).clamp(0.0, 1.0);
            color[1] = (base[1] * (1.0 + flicker * 0.75)).clamp(0.0, 1.0);
            color[2] = (base[2] * (1.0 - flicker * 0.35)).clamp(0.0, 1.0);
        }
        InlineEffect::Shadow => {
            // Gentle dimming, not invisible text. Preserve authored alpha.
            let dim = amplitude * (0.5 + a * 0.5);
            for channel in color.iter_mut().take(3) {
                *channel = (*channel * (1.0 - dim)).clamp(0.0, 1.0);
            }
        }
        InlineEffect::Shimmer => {
            // Hue-tinted periodic ripple along a word; no text movement/reflow.
            let sweep = amplitude * a;
            color[0] = (base[0] * (1.0 + sweep * 0.45)).clamp(0.0, 1.0);
            color[1] = (base[1] * (1.0 - sweep * 0.35)).clamp(0.0, 1.0);
            color[2] = (base[2] * (1.0 + sweep)).clamp(0.0, 1.0);
        }
        InlineEffect::Static => {},
    }
    color
}

/// Only authored dynamic TextColor values are updated. No reads from disk,
/// no allocations or string shaping and no changes to Text/TextFont/Node.
/// At most 64 authored style spans are traversed per player update.
pub fn animate_inline_effects(
    time: Res<Time>,
    session: Res<Session>,
    state: Res<ReadingState>,
    mut spans: Query<(&InlineUiSpan, &mut TextColor)>,
) {
    let styles = &session.game.theme.inline_styles;
    if state.segments.is_empty()
        || !styles.values().any(|style| style.effect != vesper_game::InlineEffect::Static)
    {
        return;
    }
    // Reduced motion freezes both the periodic phase and the text's appearance.
    // High contrast always takes precedence and restores the normal text color
    // in sync_spans whenever the setting changes.
    if session.preferences.high_contrast.unwrap_or(false) { return; }
    let elapsed = if session.preferences.reduced_motion.unwrap_or(false) {
        0.0
    } else {
        time.elapsed_secs()
    };
    for (node, mut tint) in &mut spans {
        if node.line_id != state.line_id { continue; }
        let Some(segment) = state.segments.get(node.index) else { continue };
        let Some(style) = segment.style.as_ref().and_then(|name| styles.get(name)) else { continue };
        if style.effect == vesper_game::InlineEffect::Static { continue; }
        let value = effect_palette(style.color, style.effect,
            style.effect_amplitude, style.effect_rate_hz, elapsed, node.index);
        let target = text_color(value);
        if *tint != target { *tint = target; }
    }
}

#[cfg(test)]
mod effect_tests {
    use super::*;
    use vesper_game::InlineEffect;

    #[test]
    fn dynamic_colors_change_with_time_and_stay_bounded_without_reflow() {
        let base = [0.71, 0.58, 0.32, 0.8];
        for effect in [InlineEffect::Ember, InlineEffect::Shadow, InlineEffect::Shimmer] {
            let first = effect_palette(base, effect, 0.25, 1.0, 0.0, 0);
            let next = effect_palette(base, effect, 0.25, 1.0, 0.23, 0);
            assert_ne!(first, next, "{effect:?} never animated");
            for color in [first, next] {
                assert!(color.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
                assert_eq!(color[3], base[3], "effect must preserve text opacity");
            }
            assert_eq!(effect_palette(base, effect, 0.25, 1.0, 0.0, 0),
                first, "same frame and author settings must be deterministic");
        }
    }

    #[test]
    fn zero_amplitude_zero_rate_and_static_preserve_author_colors() {
        let base = [0.6, 0.7, 0.8, 0.9];
        for effect in [InlineEffect::Static, InlineEffect::Ember, InlineEffect::Shadow,
                       InlineEffect::Shimmer] {
            assert_eq!(effect_palette(base, effect, 0.0, 4.0, 7.0, 4), base);
            assert_eq!(effect_palette(base, effect, 0.2, 0.0, 7.0, 4), base);
        }
        assert_eq!(effect_palette(base, InlineEffect::Static, 0.25, 4.0, 7.0, 4), base);
    }
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
                    let replacement = if state.segments.is_empty() {
                        text.as_str()
                    } else {
                        rich_root_prefix(&text, &state.segments)
                    };
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
