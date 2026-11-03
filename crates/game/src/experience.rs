//! Optional narrative-adventure state, independent of dialogue presentation.
//!
//! These are pure, engine-owned transition semantics. The player will wire
//! them to authored exploration locations and UI in later milestones. Nothing
//! in this module changes the legacy story machine or existing save format.

use serde::{Deserialize, Serialize};

/// Speech bubbles do not imply that the player can walk. Either presentation
/// is valid in a static visual novel or an explorable conversation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueSurface {
    #[default]
    Panel,
    Bubble,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    #[default]
    Right,
}

/// Logical game-world position; location bounds are validated against the
/// external map by the eventual exploration loader, not by the story parser.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPosition {
    pub location: String,
    pub x: f32,
    pub facing: Facing,
}

fn valid_id(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

impl WorldPosition {
    pub fn new(location: impl Into<String>, x: f32, facing: Facing) -> Result<Self, String> {
        let position = Self { location: location.into(), x, facing };
        if !valid_id(&position.location) || !position.x.is_finite() || position.x < 0.0 {
            return Err("exploration position requires a location ID and finite nonnegative x".into());
        }
        Ok(position)
    }
}

/// Conversation is a temporary modal child of exploration, not a forced walk.
/// Pure visual novels never need to instantiate an exploration position.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceMode {
    #[default]
    VisualNovel,
    Exploring(WorldPosition),
    Conversation { scene: String, return_to: WorldPosition },
}

/// The two axes deliberately remain separate. A game can choose bubbles
/// without exploration or use full illustrated panels while walking.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExperienceState {
    pub mode: ExperienceMode,
    pub dialogue_surface: DialogueSurface,
}

impl ExperienceState {
    /// Explicit authored transition; no movement is synthesized.
    pub fn enter_exploration(&mut self, position: WorldPosition) {
        self.mode = ExperienceMode::Exploring(position);
    }

    /// Only an explorable location can open a returnable interaction.
    pub fn begin_interaction(&mut self, scene: impl Into<String>) -> Result<(), String> {
        let scene = scene.into();
        if !valid_id(&scene) {
            return Err("interaction requires a valid authored scene ID".into());
        }
        let ExperienceMode::Exploring(return_to) = &self.mode else {
            return Err("interaction requires an active exploration location".into());
        };
        self.mode = ExperienceMode::Conversation {
            scene,
            return_to: return_to.clone(),
        };
        Ok(())
    }

    /// Explicitly resume the exact authored world position after dialogue.
    /// A general VN scene never returns to exploration on its own.
    pub fn resume_exploration(&mut self) -> Result<(), String> {
        let ExperienceMode::Conversation { return_to, .. } = &self.mode else {
            return Err("no exploration conversation is awaiting return".into());
        };
        self.mode = ExperienceMode::Exploring(return_to.clone());
        Ok(())
    }

    /// Explicit cinematic VN transition drops any pending exploration return.
    pub fn enter_visual_novel(&mut self) {
        self.mode = ExperienceMode::VisualNovel;
    }

    pub fn set_dialogue_surface(&mut self, surface: DialogueSurface) {
        self.dialogue_surface = surface;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_visual_novels_default_to_panel_and_never_require_world_state() {
        let state = ExperienceState::default();
        assert_eq!(state.mode, ExperienceMode::VisualNovel);
        assert_eq!(state.dialogue_surface, DialogueSurface::Panel);
        assert!(serde_json::from_str::<ExperienceState>("{}").is_ok());
        assert_eq!(
            serde_json::from_str::<ExperienceState>("{}").unwrap(),
            state
        );
    }

    #[test]
    fn bubble_and_world_modes_are_independent_and_interactions_return_exactly() {
        let mut state = ExperienceState::default();
        state.set_dialogue_surface(DialogueSurface::Bubble);
        assert_eq!(state.mode, ExperienceMode::VisualNovel);

        let original = WorldPosition::new("cafe_street", 144.25, Facing::Left).unwrap();
        state.enter_exploration(original.clone());
        state.begin_interaction("neighbor_talk").unwrap();
        assert_eq!(state.dialogue_surface, DialogueSurface::Bubble);
        assert_eq!(
            state.mode,
            ExperienceMode::Conversation {
                scene: "neighbor_talk".into(),
                return_to: original.clone(),
            }
        );
        state.set_dialogue_surface(DialogueSurface::Panel);
        state.resume_exploration().unwrap();
        assert_eq!(state.mode, ExperienceMode::Exploring(original));
        assert_eq!(state.dialogue_surface, DialogueSurface::Panel);
    }

    #[test]
    fn cinematic_transfer_does_not_implicitly_restore_location() {
        let mut state = ExperienceState::default();
        state.enter_exploration(WorldPosition::new("station", 32.0, Facing::Right).unwrap());
        state.begin_interaction("important_scene").unwrap();
        state.enter_visual_novel();
        assert_eq!(state.mode, ExperienceMode::VisualNovel);
        assert!(state.resume_exploration().is_err());
        assert!(state.begin_interaction("another_scene").is_err());
    }

    #[test]
    fn invalid_state_transitions_and_coordinates_fail_without_mutating_state() {
        assert!(WorldPosition::new("", 1.0, Facing::Right).is_err());
        assert!(WorldPosition::new("../escape", 1.0, Facing::Right).is_err());
        assert!(WorldPosition::new("valid", f32::INFINITY, Facing::Left).is_err());
        assert!(WorldPosition::new("valid", f32::NAN, Facing::Left).is_err());
        assert!(WorldPosition::new("valid", -1.0, Facing::Left).is_err());

        let mut state = ExperienceState::default();
        assert!(state.begin_interaction("test").is_err());
        assert!(state.resume_exploration().is_err());
        state.enter_exploration(WorldPosition::new("street", 0.0, Facing::Right).unwrap());
        let before = state.clone();
        assert!(state.begin_interaction("bad scene").is_err());
        assert_eq!(state, before);
        state.begin_interaction("hello").unwrap();
        assert!(state.begin_interaction("nested").is_err());
        assert_eq!(state.dialogue_surface, DialogueSurface::Panel);
    }

    #[test]
    fn mode_roundtrip_preserves_return_point_and_surface() {
        let mut state = ExperienceState::default();
        state.enter_exploration(WorldPosition::new("street", 199.5, Facing::Left).unwrap());
        state.begin_interaction("chat").unwrap();
        state.set_dialogue_surface(DialogueSurface::Bubble);
        let encoded = serde_json::to_string(&state).unwrap();
        let decoded: ExperienceState = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, state);
    }
}
