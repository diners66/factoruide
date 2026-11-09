//! The level window (`Params::strict_levels`): never a mob above the character's level, never one
//! more than `levels_below` under it.

use crate::job::Prepared;
use crate::model::Guard;
use crate::params::{Might, Params};
use crate::plan::combat::Combat;

/// A level 10 quest killing 8 mobs of `mob` level.
fn kill_quest(mob: i64) -> Prepared {
    use crate::testing::{Q, Scenario};
    Scenario::new()
        .levels(1, 30)
        .quest(Q::new(900, "Kill").level(10).kill("Boar", mob, 8, 1, 50.0, 50.0))
        .build()
}

#[test]
fn strict_levels_never_plan_a_mob_above() {
    let mut p = kill_quest(12);
    p.params.progression_margin = Some(5.0);
    let q = &p.model.quests[0];
    let level = |params: &Params| {
        Combat {
            params,
            dungeons: &p.model.dungeons,
        }
        .level_at(q, &q.objectives[0], Might::PLAIN)
    };
    assert!(level(&p.params) >= 12, "strict: {}", level(&p.params));
    let mut loose = p.params.clone();
    loose.strict_levels = false;
    assert!(level(&loose) < 12, "loose: {}", level(&loose));
}

#[test]
fn strict_guard_needs_its_level() {
    let mut p = kill_quest(5);
    p.params.progression_margin = Some(5.0);
    let combat = Combat {
        params: &p.params,
        dungeons: &p.model.dungeons,
    };
    let guard = Guard {
        level: 14,
        count: 3,
        elite: false,
    };
    assert!(combat.guard_level(&guard, Might::PLAIN) >= 14);
}

#[test]
fn strict_levels_old_request_gets_the_new_defaults() {
    let params: Params = serde_json::from_str("{}").unwrap();
    assert!(params.strict_levels);
    assert_eq!(params.levels_below, 4);
}

/// A level 15 character and a level 12 quest killing 8 mobs of `mob` level (giver and turn-in at
/// the start).
fn at_level_15(mob: i64) -> Prepared {
    use crate::testing::{Q, Scenario};
    Scenario::new()
        .levels(15, 30)
        .quest(Q::new(900, "Kill").level(12).kill("Boar", mob, 8, 1, 50.0, 50.0))
        .build()
}

fn planner(p: &Prepared) -> crate::plan::Planner<'_> {
    crate::plan::Planner::new(&p.model, &p.world, &p.params, &p.profile)
}

fn stop(kind: crate::plan::Kind) -> crate::plan::Edit {
    crate::plan::Edit::Stop {
        stop: crate::plan::Stop::quest(0, kind),
        farm: false,
        spot: None,
    }
}

fn abandoned(report: &crate::plan::Edited) -> bool {
    report
        .trace
        .iter()
        .any(|t| matches!(t.event, crate::plan::Event::Abandon { quest: 0 }))
}

#[test]
fn too_low_quest_is_not_taken() {
    use crate::plan::{Kind, Outcome};
    // 15 - 4 = 11: mobs of level 10 are below the window, of level 11 in it.
    let report = planner(&at_level_15(10)).edit(&[stop(Kind::Accept)]);
    assert_eq!(
        report.steps[0].outcome,
        Outcome::Invalid("mobs too low for the level window")
    );
    let report = planner(&at_level_15(11)).edit(&[stop(Kind::Accept)]);
    assert_eq!(report.steps[0].outcome, Outcome::Done);
}

#[test]
fn too_low_in_the_log_is_given_up() {
    // Checked before every stop of a route (`Simulator::apply`): the imported log at the start.
    for (mob, given_up) in [(10, true), (11, false)] {
        let mut p = at_level_15(mob);
        p.model.initial.accepted = vec![0];
        let plan = planner(&p);
        let mut state = plan.sim().initial_state();
        plan.sim().abandon_hopeless(&mut state, &mut |_, _| {});
        assert_eq!(!state.accepted[0], given_up, "mobs of level {mob}");
    }
}

#[test]
fn done_objectives_keep_the_turn_in() {
    use crate::plan::{Kind, Outcome};
    let mut p = at_level_15(10);
    p.model.initial.accepted = vec![0];
    p.model.initial.progress = vec![(0, 0, 1.0)];
    let report = planner(&p).edit(&[stop(Kind::TurnIn)]);
    assert!(!abandoned(&report));
    assert_eq!(report.steps[0].outcome, Outcome::Done);
}

#[test]
fn mandatory_low_quest_stays() {
    use crate::plan::{Kind, Outcome};
    let mut p = at_level_15(10);
    p.model.quests[0].mandatory = true;
    let report = planner(&p).edit(&[stop(Kind::Accept)]);
    assert_eq!(report.steps[0].outcome, Outcome::Done);
}

#[test]
fn low_place_skipped_for_a_place_in_window() {
    use crate::plan::{Event, Kind, Outcome};
    let mut p = at_level_15(9);
    let o = &mut p.model.quests[0].objectives[0];
    // The nearest place, mobs of level 9, and one farther off of level 13.
    let mut far = o.clone();
    far.mob_level = 13;
    far.loc.pos.x += 300.0;
    let far_pos = far.loc.pos;
    o.alts = vec![far];
    let report = planner(&p).edit(&[stop(Kind::Accept), stop(Kind::Objective(0))]);
    assert_eq!(report.steps[0].outcome, Outcome::Done);
    let visited = report.trace.iter().find_map(|t| match &t.event {
        Event::Stop { stop, loc, .. } if stop.kind == Kind::Objective(0) => Some(loc.pos),
        _ => None,
    });
    assert_eq!(visited, Some(far_pos));
}

#[test]
fn farm_window_follows_strict_levels() {
    let mut p = Params {
        farm_above: 2,
        farm_below: 6,
        ..Params::default()
    };
    assert_eq!(p.farm_levels(20), 16..=20);
    p.strict_levels = false;
    assert_eq!(p.farm_levels(20), 14..=22);
}

#[test]
fn hub_mode_is_on_by_default() {
    assert!((Params::default().camp_radius - 350.0).abs() < f64::EPSILON);
}

#[test]
fn own_side_low_quest_stays() {
    use crate::plan::{Kind, Outcome};
    // A friendly NPC of level 5 to rescue: no fight, so not too low at 15.
    let mut p = at_level_15(5);
    p.model.quests[0].objectives[0].own_side = true;
    let report = planner(&p).edit(&[stop(Kind::Accept)]);
    assert_eq!(report.steps[0].outcome, Outcome::Done);
}

#[test]
fn start_drop_too_low_is_not_taken() {
    use crate::plan::{Kind, Outcome};
    // Objective mobs in the window, but the item starting it drops from a level 5 mob.
    let mut p = at_level_15(14);
    (p.model.quests[0].start_kills, p.model.quests[0].start_mob_level) = (1.0, 5);
    let report = planner(&p).edit(&[stop(Kind::Accept)]);
    assert_eq!(
        report.steps[0].outcome,
        Outcome::Invalid("mobs too low for the level window")
    );
}

#[test]
fn a_place_in_the_window_lets_the_objective_be_fought() {
    let mut p = kill_quest(16);
    let o = &mut p.model.quests[0].objectives[0];
    let mut low = o.clone();
    low.mob_level = 13;
    low.loc.pos.x += 300.0;
    o.alts = vec![low];
    let q = &p.model.quests[0];
    let combat = Combat {
        params: &p.params,
        dungeons: &p.model.dungeons,
    };
    assert!(combat.fights(q, &q.objectives[0], 14, Might::PLAIN));
    assert!(combat.level_of(q, &q.objectives[0], Might::PLAIN) < 16);
}

#[test]
fn dungeons_are_run_within_the_window() {
    let mut p = Params {
        dungeon_from: -3,
        dungeon_to: 6,
        ..Params::default()
    };
    assert_eq!(p.dungeon_levels(), (0, 4));
    p.strict_levels = false;
    assert_eq!(p.dungeon_levels(), (-3, 6));
}

#[test]
fn grinding_camps_are_never_above() {
    use crate::testing::Scenario;
    // Level 10 beside a camp of mobs of `mob` level (one group per scenario: the scenarios give
    // every group the same NPC IDs, merged as one mob listed twice).
    let camp = |mob: i64| {
        let p = Scenario::new().levels(10, 30).mobs(1, 52.0, 50.0, mob, 12).build();
        let plan = planner(&p);
        plan.sim().camp(&plan.sim().initial_state())
    };
    assert_eq!(camp(11), None, "level 11 mobs: not a camp to grind at");
    assert!(camp(9).is_some(), "level 9 mobs: ground at");
    assert!(camp(10).is_some(), "level 10 mobs: ground at");
}

/// A level 16 character whose addon gave no quest done (an emptied profile) and a level 16 quest
/// after a level 8 one: its history is unknown, not empty, so what is below the level window
/// counts as done and the chain opens. With a history, it says what was done.
#[test]
fn unknown_history_counts_the_quests_below_as_done() {
    use crate::job::{QuestId, StartState};
    use crate::plan::{Kind, Outcome};
    use crate::testing::{Q, Scenario};
    let open = |completed: Vec<QuestId>| {
        let p = Scenario::new()
            .levels(16, 30)
            .quest(Q::new(901, "Before").level(8).kill("Boar", 8, 4, 1, 50.0, 50.0))
            .quest(
                Q::new(902, "After")
                    .level(16)
                    .after(&[901])
                    .kill("Boar", 16, 4, 1, 50.0, 50.0),
            )
            .start(StartState {
                level: 16,
                completed,
                ..StartState::default()
            })
            .build();
        let i = p.model.index[&902] as u32;
        let stop = crate::plan::Edit::Stop {
            stop: crate::plan::Stop::quest(i, Kind::Accept),
            farm: false,
            spot: None,
        };
        planner(&p).edit(&[stop]).steps[0].outcome
    };
    assert_eq!(open(vec![]), Outcome::Done, "no history at level 16: unknown");
    assert_eq!(
        open(vec![QuestId(999)]),
        Outcome::Invalid("prerequisite not done"),
        "a history without it"
    );
}

/// At a hub, a quest of the route is taken as soon as it can be, however far ahead the route
/// takes it: 40 quests elsewhere between the two accepts of the hub's quests.
#[test]
fn hub_takes_a_planned_quest_as_soon_as_available() {
    use crate::plan::{Event, Kind, Stop};
    let n = 42;
    let mut p = crate::plan::tests::quests_at_start(n);
    p.params.camp_radius = 50.0;
    let mut far = p.model.start.clone();
    far.pos.x += 1000.0;
    for q in &mut p.model.quests[2..] {
        (q.starts, q.ends) = (vec![far.clone()], vec![far.clone()]);
    }
    let mut route = vec![Stop::quest(0, Kind::Accept), Stop::quest(0, Kind::TurnIn)];
    for i in 2..n as u32 {
        route.extend([Stop::quest(i, Kind::Accept), Stop::quest(i, Kind::TurnIn)]);
    }
    route.extend([Stop::quest(1, Kind::Accept), Stop::quest(1, Kind::TurnIn)]);
    let planner = planner(&p);
    let mut trace = Vec::new();
    planner.simulate(&route, Some(&mut trace)).expect("valid route");
    let accepts: Vec<usize> = trace
        .iter()
        .filter_map(|t| match t.event {
            Event::Stop { stop, .. } if stop.kind == Kind::Accept => Some(stop.index as usize),
            _ => None,
        })
        .collect();
    assert_eq!(&accepts[..2], &[0, 1], "quest 1 taken at the hub first: {accepts:?}");
}

/// A timed quest is not taken early at a hub: its time would run out.
#[test]
fn hub_keeps_a_far_timed_quest_for_later() {
    use crate::plan::{Event, Kind, Stop};
    let n = 42;
    let mut p = crate::plan::tests::quests_at_start(n);
    p.params.camp_radius = 50.0;
    p.model.quests[1].time_limit = Some(100_000.0);
    let mut far = p.model.start.clone();
    far.pos.x += 1000.0;
    for q in &mut p.model.quests[2..] {
        (q.starts, q.ends) = (vec![far.clone()], vec![far.clone()]);
    }
    let mut route = vec![Stop::quest(0, Kind::Accept), Stop::quest(0, Kind::TurnIn)];
    for i in 2..n as u32 {
        route.extend([Stop::quest(i, Kind::Accept), Stop::quest(i, Kind::TurnIn)]);
    }
    route.extend([Stop::quest(1, Kind::Accept), Stop::quest(1, Kind::TurnIn)]);
    let planner = planner(&p);
    let mut trace = Vec::new();
    planner.simulate(&route, Some(&mut trace)).expect("valid route");
    let first: Vec<usize> = trace
        .iter()
        .filter_map(|t| match t.event {
            Event::Stop { stop, .. } if stop.kind == Kind::Accept => Some(stop.index as usize),
            _ => None,
        })
        .take(2)
        .collect();
    assert_eq!(first, vec![0, 2]);
}

/// Mobs of several levels at an objective's spot (a camp of 16 to 20): the window asks for the
/// highest of them, not the level of the weakest (`Objective::mob_top`).
#[test]
fn strict_levels_wait_for_the_strongest_mob_at_the_spot() {
    let mut p = kill_quest(12);
    p.params.progression_margin = Some(5.0);
    p.model.quests[0].objectives[0].mob_top = 15;
    let q = &p.model.quests[0];
    let combat = Combat {
        params: &p.params,
        dungeons: &p.model.dungeons,
    };
    let o = &q.objectives[0];
    assert!(combat.level_at(q, o, Might::PLAIN) >= 15, "{}", combat.level_at(q, o, Might::PLAIN));
    assert!(!combat.fights(q, o, 14, Might::PLAIN));
    assert!(combat.fights(q, o, 15, Might::PLAIN));
}
