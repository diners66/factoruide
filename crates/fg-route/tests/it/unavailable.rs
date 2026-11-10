//! Quests the game does not give the character (the guide addon's Availability.lua, read with
//! the profile): left out of its guides, with a note.

use crate::common::repo;
use fg_route::fixture::Fixture;
use fg_route::job::{self, Overrides, PlanRequest, StartState, Unavailable};
use fg_route::xp::Edition;

fn request() -> PlanRequest {
    serde_json::from_value(serde_json::json!({
        "race": "skyborne_horde", "class": "druid", "from_level": 40, "to_level": 41,
    }))
    .unwrap()
}

#[test]
fn quests_the_game_does_not_give_are_left_out() {
    let conn = Fixture::new(Edition::Forever).file("forever_druid").open();
    let overrides = Overrides::load_edition(&repo().join("overrides"), Edition::Forever).unwrap();
    let mut req = request();
    req.start = Some(StartState {
        level: 40,
        ..StartState::default()
    });
    req.required_class_quests = None;
    let before = job::prepare(&conn, &overrides, &req, &|_| {}).unwrap();
    // Two quests the model has, one not required by anything else.
    let mut ids: Vec<i64> = before.model.quests.iter().map(|q| q.id).collect();
    ids.sort_unstable();
    let (missing, not_offered) = (ids[0], ids[1]);
    let name = |id: i64| before.model.quests[before.model.index[&id]].name.clone();

    req.start = Some(StartState {
        level: 40,
        unavailable: vec![
            Unavailable {
                id: missing,
                why: "missing".into(),
                npc: None,
                level: Some(39),
            },
            Unavailable {
                id: not_offered,
                why: "not offered".into(),
                npc: Some(3389),
                level: Some(40),
            },
        ],
        ..StartState::default()
    });
    let after = job::prepare(&conn, &overrides, &req, &|_| {}).unwrap();
    for id in [missing, not_offered] {
        assert!(
            !after.model.index.contains_key(&id),
            "{} ({id}) the game does not give is left out",
            name(id)
        );
        assert!(after.params.excluded_quests.contains(&id));
    }
    let note = after
        .notes
        .iter()
        .find(|n| n.contains("the game does not give"))
        .expect("a note says which");
    assert!(
        note.contains(&name(missing)) && note.contains("not in the game"),
        "{note}"
    );
    assert!(
        note.contains(&name(not_offered)) && note.contains("not offered"),
        "{note}"
    );
}

#[test]
fn a_profile_without_unavailable_quests_reads_as_before() {
    let start: StartState = serde_json::from_value(serde_json::json!({ "level": 12 })).unwrap();
    assert_eq!(start.unavailable, []);
    let start: StartState = serde_json::from_value(serde_json::json!({
        "level": 18,
        "unavailable": [{ "id": 9267, "why": "missing", "level": 18 }],
    }))
    .unwrap();
    assert_eq!(start.unavailable.len(), 1);
    assert_eq!(start.unavailable[0].id, 9267);
    assert_eq!(start.unavailable[0].npc, None);
}
