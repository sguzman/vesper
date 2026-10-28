//! Theme-owned focus transitions and transient confirmation feedback shared by
//! engine-owned menu and choice widgets. No sounds or assets are mandatory.
use super::*;

#[derive(Clone, Copy, Debug)]
pub enum FocusTarget {
    Menu(MenuKind, usize),
    Choice(usize),
}

#[derive(Component)]
pub struct FocusFeedback {
    target: FocusTarget,
    amount: f32,
}

impl FocusFeedback {
    pub fn menu(kind: MenuKind, index: usize, selected: bool) -> Self {
        Self { target: FocusTarget::Menu(kind, index), amount: f32::from(selected) }
    }
    pub fn choice(index: usize, selected: bool) -> Self {
        Self { target: FocusTarget::Choice(index), amount: f32::from(selected) }
    }
}

#[derive(Component)]
struct ConfirmationCue;

#[derive(Resource, Default)]
pub struct FeedbackState {
    remaining: f32,
    cue: Option<Entity>,
}

/// Called only by an explicit keyboard or pointer activation, never by an
/// update-system effect or narrative progression.
pub fn signal(state: &mut FeedbackState, session: &Session) {
    let duration = session.game.theme.feedback.confirm_seconds;
    if !session.preferences.reduced_motion.unwrap_or(false) && duration > 0.0 {
        state.remaining = duration;
    }
}

fn focused(target: FocusTarget, session: &Session) -> bool {
    match target {
        FocusTarget::Menu(MenuKind::Main, index) =>
            session.menu_overlay.is_some() && session.menu_selection == index,
        FocusTarget::Menu(MenuKind::Pause, index) =>
            session.paused && session.pause_selection == index,
        FocusTarget::Menu(MenuKind::Ending, index) =>
            session.finished && session.ending_selection == index,
        FocusTarget::Choice(index) =>
            session.choosing && !session.finished
                && session.presentation.selected_choice == index,
    }
}

fn ease(x: f32) -> f32 {
    let p = x.clamp(0.0, 1.0);
    p * p * (3.0 - 2.0 * p)
}

fn row_color(target: FocusTarget, amount: f32, accent: [f32;4]) -> Color {
    if amount <= 0.0 { return Color::NONE; }
    let (factor, alpha) = match target {
        FocusTarget::Menu(MenuKind::Ending, _) => (0.27, 0.50),
        FocusTarget::Menu(_, _) => (0.27, 0.94),
        FocusTarget::Choice(_) => (0.28, 0.94),
    };
    Color::srgba(
        accent[0] * factor,
        accent[1] * factor,
        accent[2] * factor,
        alpha * ease(amount),
    )
}

pub fn tick(
    time: Res<Time>,
    session: Res<Session>,
    mut state: ResMut<FeedbackState>,
    mut commands: Commands,
    mut rows: Query<(&mut BackgroundColor, &mut FocusFeedback), Without<ConfirmationCue>>,
    mut cues: Query<&mut BackgroundColor, (With<ConfirmationCue>, Without<FocusFeedback>)>,
) {
    let reduced = session.preferences.reduced_motion.unwrap_or(false);
    let theme = &session.game.theme.feedback;
    let dt = time.delta_secs().clamp(0.0, 0.1);
    for (mut bg, mut focus) in &mut rows {
        let selected = focused(focus.target, &session);
        let target = if selected { 1.0 } else { 0.0 };
        let next = if reduced || theme.focus_seconds <= 0.0 {
            target
        } else if selected {
            (focus.amount + dt / theme.focus_seconds).min(1.0)
        } else {
            (focus.amount - dt / theme.focus_seconds).max(0.0)
        };
        focus.amount = next;
        let color = row_color(focus.target, next, session.game.theme.panel.accent);
        if bg.0 != color { bg.0 = color; }
    }

    if reduced || theme.confirm_seconds <= 0.0 || state.remaining <= 0.0 {
        state.remaining = 0.0;
        if let Some(entity) = state.cue.take() {
            commands.entity(entity).despawn();
        }
        return;
    }
    if state.cue.is_none() {
        let entity = commands.spawn((
            Name::new("VX07 confirmation cue"),
            ConfirmationCue,
            Node {
                position_type: PositionType::Absolute,
                top: px(0.0), left: px(0.0),
                height: px(3.0), width: percent(100.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        )).id();
        state.cue = Some(entity);
    }
    let fraction = (state.remaining / theme.confirm_seconds).clamp(0.0, 1.0);
    let rgba = theme.cue_color;
    let color = BackgroundColor(Color::srgba(
        rgba[0], rgba[1], rgba[2],
        rgba[3] * theme.cue_opacity * fraction,
    ));
    if let Some(entity) = state.cue {
        if let Ok(mut background) = cues.get_mut(entity) {
            if *background != color { *background = color; }
        }
    }
    state.remaining = (state.remaining - dt).max(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_ends_are_stable_and_transitions_bounded() {
        let accent = [0.7, 0.9, 1.0, 1.0];
        let none = row_color(FocusTarget::Choice(0), 0.0, accent);
        let full = row_color(FocusTarget::Choice(0), 1.0, accent);
        assert_eq!(none, Color::NONE);
        assert!(full.alpha() > 0.0);
        for fraction in [-100.0, 0.0, 0.1, 0.6, 1.0, 100.0] {
            assert!((0.0..=1.0).contains(&ease(fraction)));
        }
    }

    #[test]
    fn focus_state_starts_from_explicit_selection() {
        assert_eq!(FocusFeedback::menu(MenuKind::Pause, 0, false).amount, 0.0);
        assert_eq!(FocusFeedback::choice(1, true).amount, 1.0);
    }
}
