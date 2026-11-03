//! Pure focus selection for optional 1D authored interaction zones.
//!
//! Focus is deterministic when two candidates overlap: nearest interaction,
//! then stable NPC-before-prop / lexical ID order. There is no pixel hunting.
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
    let mut best: Option<Target<'a>> = None;
    for (id, npc) in &area.npcs {
        consider(&mut best, TargetKind::Npc, id, npc.x,
            npc.interaction_radius, &npc.interact_scene, position.x);
    }
    for (id, prop) in &area.props {
        if let Some(scene) = &prop.interact_scene {
            consider(&mut best, TargetKind::Prop, id, prop.x,
                prop.interaction_radius, scene, position.x);
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
                    image: None, interaction_radius: 90.0 }),
                ("zoe".into(), Npc { x: 440.0, interact_scene: "zoe_scene".into(),
                    image: None, interaction_radius: 90.0 }),
            ]),
            props: BTreeMap::from([
                ("billboard".into(), Prop { x: 410.0, interact_scene: Some("read".into()),
                    image: None, interaction_radius: 90.0 }),
                ("decoration".into(), Prop { x: 415.0, interact_scene: None,
                    image: None, interaction_radius: 90.0 }),
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
    fn focused_scene_is_a_validated_external_reference() {
        let area = area();
        let focus = nearest(&area, &at(475.0)).unwrap();
        assert_eq!(focus.scene, "zoe_scene");
        assert!(focus.distance <= 90.0);
    }
}
