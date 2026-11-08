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
