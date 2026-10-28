//! Runtime text pacing. Only the bounded grapheme boundary table is built
//! when a new dialogue arrives; the frame loop uses a prefix and clock.
use super::*;
use crate::text_reveal::{RevealBuffer, RevealSpeed};

#[derive(Component)]
pub struct ContinueCue;

#[derive(Resource, Default)]
pub struct ReadingState {
    buffer: Option<RevealBuffer>,
    showing_dialogue: bool,
    auto_elapsed: f32,
    pub autoplay_ready: bool,
    dirty: bool,
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
        self.showing_dialogue = dialogue;
        self.buffer = Some(RevealBuffer::new(
            body, if dialogue { speed } else { RevealSpeed::Instant }
        ));
        self.auto_elapsed = 0.0;
        self.autoplay_ready = false;
        self.dirty = true;
        self.buffer.as_ref().map_or("", RevealBuffer::visible_text)
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

pub fn tick(
    time: Res<Time>,
    session: Res<Session>,
    settings: Res<settings::SettingsState>,
    mut state: ResMut<ReadingState>,
    mut bodies: Query<(&StoryText, &mut Text), Without<ContinueCue>>,
    mut cues: Query<(&mut Node, &mut TextColor, &mut TextFont), With<ContinueCue>>,
    assets: Res<AssetServer>,
) {
    let active = !session.paused && session.menu_overlay.is_none()
        && settings.overlay.is_none() && !session.finished && !session.choosing;
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
                if matches!(slot, StoryText::Body) && body.0 != text {
                    body.0.clone_from(&text);
                }
            }
        }
    }

    let cue_active = active && showing_dialogue && complete;
    if cue_active && session.preferences.auto_advance.unwrap_or(false) {
        state.auto_elapsed = (state.auto_elapsed + time.delta_secs()).min(60.0);
        if state.auto_elapsed >= 1.5 && !state.autoplay_ready {
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
