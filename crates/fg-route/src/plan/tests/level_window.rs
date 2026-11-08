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
