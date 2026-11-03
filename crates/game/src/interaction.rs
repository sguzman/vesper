//! Pure focus selection for optional 1D authored interaction zones.
//!
//! Focus is deterministic when two candidates overlap: nearest interaction,
//! then stable NPC-before-prop / lexical ID order. There is no pixel hunting.
use std::collections::BTreeMap;
use vesper_story::Value;
use crate::{experience::WorldPosition, exploration::Location};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind { Npc, Prop }

#[derive(Clone, Debug, PartialEq)]
pub struct Target<'a> {
    pub kind: TargetKind,
    pub id: &'a str,
    pub scene: &'a str,
    pub x: f32,
    pub distance: f32,
}

fn consider<'a>(best: &mut Option<Target<'a>>, kind: TargetKind,
    id: &'a str, x: f32, radius: f32, scene: &'a str, position_x: f32) {
    let distance = (position_x - x).abs();
    if !distance.is_finite() || distance > radius { return; }
    if best.as_ref().is_none_or(|prior| distance < prior.distance) {
        *best = Some(Target { kind, id, scene, x, distance });
    }
}

/// Stable tie-breaks use BTreeMap order, and NPCs precede props.
pub fn nearest<'a>(area: &'a Location, position: &WorldPosition) -> Option<Target<'a>> {
    nearest_with_state(area, position, &BTreeMap::new())
}

pub fn nearest_with_state<'a>(area: &'a Location, position: &WorldPosition,
    variables: &BTreeMap<String, Value>) -> Option<Target<'a>> {
    let mut best: Option<Target<'a>> = None;
    for (id, npc) in &area.npcs {
        if npc.visible_if.as_ref().is_some_and(|gate| !gate.matches(variables)) {
            continue;
        }
        consider(&mut best, TargetKind::Npc, id, npc.x,
            npc.interaction_radius, &npc.interact_scene, position.x);
    }
    for (id, prop) in &area.props {
        if prop.visible_if.as_ref().is_some_and(|gate| !gate.matches(variables)) {
            continue;
        }
        if let Some(scene) = &prop.interact_scene {
            consider(&mut best, TargetKind::Prop, id, prop.x,
                prop.interaction_radius, scene, position.x);
        }
    }
    best
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExitTarget<'a> {
    pub id: &'a str,
    pub to_location: &'a str,
    pub to_spawn: &'a str,
    pub distance: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Focus<'a> {
    Scene(Target<'a>),
    Exit(ExitTarget<'a>),
}

impl Focus<'_> {
    pub fn id(&self) -> &str {
        match self {
            Self::Scene(target) => target.id,
            Self::Exit(target) => target.id,
        }
    }
    pub fn distance(&self) -> f32 {
        match self {
            Self::Scene(target) => target.distance,
            Self::Exit(target) => target.distance,
        }
    }
}

/// Nearest interaction, with scene interactions winning exact ties.
/// Exits never cause automatic traversal: E explicitly selects this focus.
pub fn focus<'a>(area: &'a Location, position: &WorldPosition,
    variables: &BTreeMap<String, Value>) -> Option<Focus<'a>> {
    let mut best = nearest_with_state(area, position, variables).map(Focus::Scene);
    for (id, exit) in &area.exits {
        let distance = (position.x - exit.x).abs();
        if !distance.is_finite() || distance > exit.interaction_radius {
            continue;
        }
        if best.as_ref().is_none_or(|current| distance < current.distance()) {
            best = Some(Focus::Exit(ExitTarget {
                id, to_location: &exit.to_location, to_spawn: &exit.to_spawn,
                distance,
            }));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{experience::Facing, exploration::{Npc, Prop}};
    use std::collections::BTreeMap;

    fn area() -> Location {
        Location {
            background: "street.png".into(),
            width: 1280.0,
            height: 720.0,
            camera: Default::default(),
            spawns: BTreeMap::new(),
            npcs: BTreeMap::from([
                ("amy".into(), Npc { x: 410.0, interact_scene: "amy_scene".into(),
                    image: None, interaction_radius: 90.0, visible_if: None }),
                ("zoe".into(), Npc { x: 440.0, interact_scene: "zoe_scene".into(),
                    image: None, interaction_radius: 90.0, visible_if: None }),
            ]),
            props: BTreeMap::from([
                ("billboard".into(), Prop { x: 410.0, interact_scene: Some("read".into()),
                    image: None, interaction_radius: 90.0, visible_if: None }),
                ("decoration".into(), Prop { x: 415.0, interact_scene: None,
                    image: None, interaction_radius: 90.0, visible_if: None }),
            ]),
            exits: BTreeMap::new(),
            layers: BTreeMap::new(),
        }
    }
    fn at(x: f32) -> WorldPosition {
        WorldPosition::new("street", x, Facing::Right).unwrap()
    }

    #[test]
    fn no_remote_interactions_or_decorative_burden() {
        let area = area();
        assert!(nearest(&area, &at(100.0)).is_none());
        assert!(nearest(&area, &at(1000.0)).is_none());
        assert_eq!(nearest(&area, &at(410.0)).unwrap().id, "amy");
    }

    #[test]
    fn nearest_wins_overlap_with_stable_tie_breaks() {
        let area = area();
        let focus = nearest(&area, &at(435.0)).unwrap();
        assert_eq!(focus.id, "zoe");
        assert_eq!(focus.kind, TargetKind::Npc);
        let focus = nearest(&area, &at(410.0)).unwrap();
        assert_eq!(focus.id, "amy"); // NPC before prop, lexical within NPCs
    }

    #[test]
    fn focus_prefers_closer_exit_and_never_triggers_outside_radius() {
        let mut area = area();
        area.exits.insert("coffee".into(), crate::exploration::Exit {
            x: 600.0, to_location: "cafe".into(),
            to_spawn: "door".into(), interaction_radius: 80.0,
        });
        let vars = BTreeMap::new();
        assert!(matches!(focus(&area, &at(600.0), &vars), Some(Focus::Exit(ref exit))
            if exit.id == "coffee" && exit.to_location == "cafe"));
        assert!(focus(&area, &at(950.0), &vars).is_none());
        // NPC wins when tied at exactly the same interaction distance.
        area.exits.get_mut("coffee").unwrap().x = 410.0;
        assert!(matches!(focus(&area, &at(410.0), &vars),
            Some(Focus::Scene(Target { id: "amy", .. }))));
    }

    #[test]
    fn conditional_npc_requires_actual_story_variable_and_its_value() {
        let mut area = area();
        area.npcs.get_mut("zoe").unwrap().visible_if =
            Some(crate::exploration::VisibilityCondition {
                variable: "met".into(),
                equals: crate::exploration::ConditionValue::Boolean(true),
            });
        let mut vars = BTreeMap::new();
        assert_eq!(focus(&area, &at(440.0), &vars).unwrap().id(), "amy");
        vars.insert("met".into(), Value::Boolean(true));
        assert_eq!(focus(&area, &at(440.0), &vars).unwrap().id(), "zoe");
        vars.insert("met".into(), Value::Boolean(false));
        assert_eq!(focus(&area, &at(440.0), &vars).unwrap().id(), "amy");
    }

    #[test]
    fn focused_scene_is_a_validated_external_reference() {
        let area = area();
        let focus = nearest(&area, &at(475.0)).unwrap();
        assert_eq!(focus.scene, "zoe_scene");
        assert!(focus.distance <= 90.0);
    }
}
