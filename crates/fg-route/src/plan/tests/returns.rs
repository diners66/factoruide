//! Returns to an area (`Planner::tidy`): an objective of a quest already in the log, near a place
//! the route went to long before, is done at that first visit rather than on a trip back.

use crate::job::Prepared;
use crate::plan::{Kind, Planner, Stop};

/// Errands at a second town 400 yards east, between the two visits of the boars.
const ERRANDS: i64 = 25;

/// A town (50, 50), boars 300 yards south (50, 80) for Ribs, and more 150 yards further
/// (50, 95) for Tusks, both taken in town. `ERRANDS` delivery quests at a town east (90, 50).
fn town_boars_and_errands() -> Prepared {
    use crate::testing::{Q, Scenario};
    let mut s = Scenario::new()
        .levels(1, 10)
        .quest(
            Q::new(1, "Ribs")
                .level(1)
                .xp(50)
                .giver(1, 50.0, 50.0)
                .turn_in_at(1, 50.0, 50.0)
                .kill("Boar", 1, 6, 1, 50.0, 80.0),
        )
        .quest(
            Q::new(2, "Tusks")
                .level(1)
                .xp(50)
                .giver(1, 51.0, 50.0)
                .turn_in_at(1, 51.0, 50.0)
                .kill("Boar", 1, 4, 1, 50.0, 95.0),
        );
    for k in 0..ERRANDS {
        s = s.quest(
            Q::new(100 + k, "Errand")
                .level(1)
                .xp(5)
                .giver(1, 90.0, 50.0)
                .turn_in_at(1, 90.0, 51.0),
        );
    }
    // The explicit takes and turn-ins this tidying moves: no hub mode (`camp_radius`).
    s.params(|p| p.camp_radius = 0.0).build()
}

/// Both quests taken, Ribs' boars, Ribs turned in, the errands, then back to the boars for
/// Tusks (`accepted_late`: Tusks taken after the errands instead).
fn route_back_to_the_boars(accepted_late: bool) -> Vec<Stop> {
    let tusks = Stop::quest(1, Kind::Accept);
    let mut route = vec![Stop::quest(0, Kind::Accept)];
    if !accepted_late {
        route.push(tusks);
    }
    route.extend([Stop::quest(0, Kind::Objective(0)), Stop::quest(0, Kind::TurnIn)]);
    for k in 0..u32::try_from(ERRANDS).unwrap() {
        route.extend([Stop::quest(2 + k, Kind::Accept), Stop::quest(2 + k, Kind::TurnIn)]);
    }
    if accepted_late {
        route.push(tusks);
    }
    route.extend([Stop::quest(1, Kind::Objective(0)), Stop::quest(1, Kind::TurnIn)]);
    route
}

fn tidied(p: &Prepared, route: &[Stop]) -> Vec<Stop> {
    Planner::new(&p.model, &p.world, &p.params, &p.profile)
        .tidy(route)
        .map_or(route.to_vec(), |(r, _)| r)
}

/// Tusks' boars, 150 yards from Ribs', are killed at the first visit, 50 stops before the trip
/// back: no return.
#[test]
fn an_objective_near_an_earlier_visit_is_done_then() {
    let p = town_boars_and_errands();
    let route = route_back_to_the_boars(false);
    let after = tidied(&p, &route);
    let at = |stop: Stop| after.iter().position(|&s| s == stop).unwrap();
    let (ribs, tusks) = (Stop::quest(0, Kind::Objective(0)), Stop::quest(1, Kind::Objective(0)));
    assert_eq!(at(tusks), at(ribs) + 1, "Tusks' boars right after Ribs': {after:?}");
}

/// Not before its quest is taken, nor beyond `return_radius`.
#[test]
fn an_objective_is_not_pulled_before_its_quest_nor_from_afar() {
    let p = town_boars_and_errands();
    let route = route_back_to_the_boars(true);
    assert_eq!(tidied(&p, &route), route, "Tusks taken after the errands");
    let mut p = town_boars_and_errands();
    p.params.return_radius = 100.0;
    let route = route_back_to_the_boars(false);
    assert_eq!(tidied(&p, &route), route, "150 yards is not the same place then");
}

/// Tusks in the log from the start (no accept in the route): its boars at Ribs' visit too.
#[test]
fn an_objective_of_a_quest_in_the_starting_log_is_done_at_the_first_visit() {
    let mut p = town_boars_and_errands();
    p.model.initial.accepted.push(1);
    let route: Vec<Stop> = route_back_to_the_boars(false)
        .into_iter()
        .filter(|&s| s != Stop::quest(1, Kind::Accept))
        .collect();
    let after = tidied(&p, &route);
    let at = |stop: Stop| after.iter().position(|&s| s == stop).unwrap();
    let (ribs, tusks) = (Stop::quest(0, Kind::Objective(0)), Stop::quest(1, Kind::Objective(0)));
    assert_eq!(at(tusks), at(ribs) + 1, "{after:?}");
}

/// Returns counted by the score (`Params::return_weight`), so that the search weighs them.
fn returns_of(p: &Prepared, route: &[Stop]) -> u32 {
    Planner::new(&p.model, &p.world, &p.params, &p.profile)
        .simulate_full(route, None)
        .expect("a valid route")
        .1
        .returns
}

/// The trip back to the boars for Tusks, in the log at Ribs' visit, is a return; not once
/// Tusks' boars are done at that visit, nor when Tusks is taken after it.
#[test]
fn the_score_counts_a_return_for_a_quest_in_the_log_at_the_first_visit() {
    let mut p = town_boars_and_errands();
    // The errands' town is 500 yards from Ribs' boars.
    p.params.return_away = 300.0;
    let route = route_back_to_the_boars(false);
    assert_eq!(returns_of(&p, &route), 1);
    p.params.return_slack = 300.0;
    assert_eq!(returns_of(&p, &tidied(&p, &route)), 0, "done at the first visit");
    assert_eq!(
        returns_of(&p, &route_back_to_the_boars(true)),
        0,
        "Tusks taken after it"
    );
    // A return costs the optimizer `return_weight` on top of the trip.
    let planner = Planner::new(&p.model, &p.world, &p.params, &p.profile);
    let with = planner.simulate(&route, None).unwrap();
    p.params.return_weight = 0.0;
    let planner = Planner::new(&p.model, &p.world, &p.params, &p.profile);
    let without = planner.simulate(&route, None).unwrap();
    assert!((with - without - 300.0).abs() < 1e-6, "{with} - {without}");
    assert_eq!(returns_of(&p, &route), 0, "off: not counted");
}

/// The state each stop of `route` starts from (`Planner::run`).
fn states_of(p: &Prepared, route: &[Stop]) -> Vec<crate::plan::state::State> {
    Planner::new(&p.model, &p.world, &p.params, &p.profile)
        .run(route)
        .expect("a valid route")
        .states
}

/// What returns are counted from keeps only the quests still in the log: the 25 errands taken
/// and turned in, and Ribs, are not carried along the rest of the route.
#[test]
fn only_the_quests_in_the_log_are_kept_for_returns() {
    let p = town_boars_and_errands();
    let route = route_back_to_the_boars(false);
    let states = states_of(&p, &route);
    let last = states.last().unwrap();
    let kept: Vec<u32> = last.taken.iter().map(|&(q, _)| q).collect();
    assert!(kept.len() <= 2, "Tusks and at most the last errand: {kept:?}");
    assert!(kept.contains(&1), "Tusks, in the log: {kept:?}");
}

/// Two errands swapped before the trip back move the stop indexes the returns are counted
/// with, not their order: the states after the swap count the same returns, so the
/// incremental evaluation can stop there (`State::same_returns`). Tusks taken after Ribs'
/// boars instead changes what is counted.
#[test]
fn reordered_stops_keep_the_same_returns() {
    let mut p = town_boars_and_errands();
    p.params.return_away = 300.0;
    let route = route_back_to_the_boars(false);
    // [Ribs, Tusks, Ribs' boars, Ribs in, (errand 0 taken, in), (errand 1 taken, in), ...]
    let mut swapped = route.clone();
    swapped[4..8].rotate_left(2);
    let (a, b) = (states_of(&p, &route), states_of(&p, &swapped));
    assert!(
        a[8].same_returns(&b[8]),
        "errands swapped: {:?} / {:?}",
        a[8].taken,
        b[8].taken
    );
    // The cost evaluated from the earlier run, stopped after the swap, is the full replay's.
    let planner = Planner::new(&p.model, &p.world, &p.params, &p.profile);
    let run = planner.run(&route).unwrap();
    let incremental = planner.evaluate(&run, &swapped, 4, 7, true).unwrap();
    let full = planner.simulate(&swapped, None).unwrap();
    assert!((incremental - full).abs() < 1e-6, "{incremental} vs {full}");
    let mut late = route;
    late.swap(1, 2); // Tusks taken after Ribs' boars
    let c = states_of(&p, &late);
    assert!(
        !a[3].same_returns(&c[3]),
        "Tusks taken after the visit: {:?} / {:?}",
        a[3].taken,
        c[3].taken
    );
}
