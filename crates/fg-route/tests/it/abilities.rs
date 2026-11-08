//! Class presets use the selected edition's quest and spell data: a short guide (effort 0) is
//! generated on the real data of each edition, for characters that differ in what the data holds
//! (the classes' spells, the editions' races).

use crate::common::repo;
use fg_route::fixture::Fixture;
use fg_route::job::{self, Overrides, PlanRequest};
use fg_route::plan::Planner;
use fg_route::xp::Edition;

fn generates_a_guide(edition: Edition, race: &str, class: &str, start: &str) {
    let conn = Fixture::new(edition).file(start).open();
    let overrides = Overrides::load_edition(&repo().join("overrides"), edition).unwrap();
    let request: PlanRequest = serde_json::from_value(serde_json::json!({
        "race":race,"class":class,"from_level":1,"to_level":5,
        // The editions' data, not the level window (under it the first mobs, above level 1,
        // are ground to first and the target reached by the last turn-ins).
        "params":{"strict_levels":false}
    }))
    .unwrap();
    let p = job::prepare(&conn, &overrides, &request, &|_| {}).unwrap();
    let planner = Planner::new(&p.model, &p.world, &p.params, &p.profile);
    let route = planner.construct(|_| {});
    assert!(planner.simulate(&route, None).is_some(), "the generated route replays");
    let guide = fg_route::export::build(&planner, &p.profile, &route);
    // The level the character reaches (the guide's steps say the level each one starts at: the
    // last turn-ins, taken at hand in a quest hub, may bring the target with no step after them).
    let mut trace = Vec::new();
    planner.simulate_full(&route, Some(&mut trace));
    let level = trace.iter().map(|t| t.level).max().unwrap_or(0);
    assert!(level >= 5, "{edition:?} {race} {class}: reaches the target");
    assert!(guide.quests > 0, "{edition:?} {race} {class}: follows quests");
}

macro_rules! preset {
    ($name:ident, $edition:expr, $race:expr, $class:expr, $start:expr) => {
        #[test]
        fn $name() {
            generates_a_guide($edition, $race, $class, $start);
        }
    };
}

preset!(
    classic_human_warrior,
    Edition::Classic,
    "human",
    "warrior",
    "classic_northshire"
);
preset!(classic_orc_shaman, Edition::Classic, "orc", "shaman", "classic_valley");
preset!(
    tbc_bloodelf_paladin,
    Edition::Tbc,
    "bloodelf",
    "paladin",
    "tbc_sunstrider"
);
preset!(tbc_draenei_shaman, Edition::Tbc, "draenei", "shaman", "tbc_ammen");
preset!(
    forever_skyborne_druid,
    Edition::Forever,
    "skyborne",
    "druid",
    "forever_zephras"
);
preset!(
    forever_skyborne_horde_druid,
    Edition::Forever,
    "skyborne_horde",
    "druid",
    "forever_zephras"
);

/// The level window (on by default) on the draenei's start: the route still reaches its target,
/// and a friendly survivor to rescue ("Kill Draenei Survivor", a kill credit on the character's
/// side) is no fight: no grinding to its level first.
#[test]
fn tbc_draenei_shaman_in_the_level_window() {
    use fg_route::export::StepKind;
    let conn = Fixture::new(Edition::Tbc).file("tbc_ammen").open();
    let overrides = Overrides::load_edition(&repo().join("overrides"), Edition::Tbc).unwrap();
    let request: PlanRequest = serde_json::from_value(serde_json::json!({
        "race":"draenei","class":"shaman","from_level":1,"to_level":5
    }))
    .unwrap();
    let p = job::prepare(&conn, &overrides, &request, &|_| {}).unwrap();
    assert!(p.params.strict_levels);
    let planner = Planner::new(&p.model, &p.world, &p.params, &p.profile);
    let route = planner.construct(|_| {});
    let guide = fg_route::export::build(&planner, &p.profile, &route);
    let mut trace = Vec::new();
    planner
        .simulate_full(&route, Some(&mut trace))
        .expect("the route replays");
    let reached = trace.iter().map(|t| t.level).max().unwrap_or(0);
    assert!(reached >= 5, "reaches the target: level {reached}");
    let rescue = (guide.steps.iter())
        .position(|s| s.kind == StepKind::Objective && s.quest_name.as_deref() == Some("Rescue the Survivors!"))
        .expect("the rescue");
    let before = &guide.steps[rescue - 1];
    assert!(before.kind != StepKind::Grind, "no grinding before the rescue");
    assert!(
        guide.steps[rescue].level < 3,
        "rescued at level {}",
        guide.steps[rescue].level
    );
}
