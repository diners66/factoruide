# Level Window and Quest Hubs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Routes never send the character after mobs above its level or more than 4 levels below it, and clear a quest hub (pick up everything first) before moving on, on by default in our fork.

**Architecture:** Two new `Params` fields gate the engine (`fg-route`): `strict_levels` raises the level an objective or a guarded place needs to at least its mobs' level, and makes quests whose mobs fell more than `levels_below` under the character unacceptable and abandoned (the same path as gray dead ends). The existing hub mode (`camp_radius`) gets a default of 40 yards. The app shows the two new settings through `paramSchema.ts`, and takes its defaults from `Params::default()`.

**Tech Stack:** Rust 1.99 (`crates/fg-route`), React + TypeScript (`app/src`), Tauri 2 desktop build.

**Spec (approved in chat, 2026-11-08):**
- Kill objectives are only planned while every mob of the objective is **between the character's level and 4 levels below**, at the level the character has when the route gets there. Never above. Elites and mobs guarding a quest's places follow the same rule. Objectives with nothing to kill are unaffected.
- A quest too low (mobs more than 4 below) or gray is **not allowed**: not taken, and given up if it falls out of the window in the log. Exception (our call, to be told to the user): mandatory quests (class quests such as warlock pet quests, quests the player asked for).
- Farming on the way follows the window; grinding is already at the character's level.
- At a hub (quest NPCs within ~40 yards of the stop), the route takes every planned quest there first, then does them in the order the optimizer finds fastest, turning in on return.
- Both rules on by default, with a switch in the app's settings.
- Plans start from the character profile exported by `Factoruide_Profile` (existing feature); guides reach the addon through the desktop app's `Factoruide_Data` or the `FGG1:` paste box (existing features). Nothing is sent to the author's server unless the user chooses to.

## Global Constraints

- Defaults: `strict_levels = true`, `levels_below = 4`, `camp_radius = 40.0` (yards, the engine's distance unit).
- Mob level above character level: never allowed when `strict_levels` is on, whatever gear, class or progression profile.
- New params must deserialize from old saved requests (`#[serde(default)]` behaviour like the other fields of `Params`).
- `cargo clippy --all-targets` and `cargo test -p fg-route` stay green; existing tests that assume the upstream behaviour set `strict_levels = false` rather than being deleted.
- Commit on branch `level-window`, never on `main`.

## Review Focus

1. A quest whose objective has several places (`Objective::alts`) at different mob levels: allowed if **any** place is in the window, and the place chosen must be in the window. Test in Task 2.
2. A quest already in the imported character's log whose mobs are now too low (the level-16 profile has a log): it is abandoned at the start, not kept forever. Test in Task 2.
3. A quest whose kill objectives are done but whose turn-in is still pending: it is not abandoned when the mobs go too low. Test in Task 2.
4. Mandatory quests (class quests) with low mobs stay in the route. Test in Task 2.
5. A saved guide request from before this change (no `strict_levels` key) loads with the new defaults and no error. Test in Task 1.

---

### Task 1: Params and the upper bound

**Files:**
- Modify: `crates/fg-route/src/params.rs` (fields next to `progression`, ~line 51; defaults ~line 504; new method next to `pack_extra`, ~line 776)
- Modify: `crates/fg-route/src/plan/combat.rs:66-98` (`guard_level`, `level_at`)
- Test: `crates/fg-route/src/plan/combat.rs` (new `#[cfg(test)] mod tests` at the end, or the existing one if any)

**Interfaces:**
- Produces: `Params::strict_levels: bool`, `Params::levels_below: i64`, `Params::level_window(&self, level: i64) -> Option<std::ops::RangeInclusive<i64>>` (None when `strict_levels` is off, else `level - levels_below..=level`).
- Produces: `Combat::level_at` and `Combat::guard_level` return at least the mob level (`o.mob_level` for an objective with `kills > 0`, `g.level` for a guard with `count > 0`) when `strict_levels` is on.

- [ ] **Step 1: Write the failing tests**

`strict_levels_never_plan_a_mob_above` builds a `Params::default()` (strict on) with `progression_margin = Some(5.0)` (generous margin), a quest of level 10 and an objective with `kills = 8.0, mob_level = 12`, and asserts `Combat::level_at(..) >= 12`; with `strict_levels = false` it asserts `< 12` (the margin lets it through). `strict_guard_needs_its_level`: a `Guard { level: 14, count: 3, .. }` gives `guard_level >= 14`. `old_request_gets_the_new_defaults`: `serde_json::from_str::<Params>("{}")` (or the request type the app sends, whichever `Params` derives from) has `strict_levels == true` and `levels_below == 4`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p fg-route strict_ -- --nocapture`
Expected: FAIL to compile (`strict_levels` unknown).

- [ ] **Step 3: Add the fields, defaults, `level_window`, and the `max` in `level_at` / `guard_level`**

Doc comments in the file's style (what, then why). In `level_at`, apply the `max` only when the objective has kills; leave the `need <= 0.0 => 0` early return for non-kill objectives untouched.

- [ ] **Step 4: Run the new tests and the whole crate**

Run: `cargo test -p fg-route`
Expected: the three new tests PASS. Upstream tests that break because they relied on fighting above level: set `p.params.strict_levels = false` in those tests (list them in the commit message).

- [ ] **Step 5: Commit**

`git commit -m "feat(route): never plan mobs above the character's level (strict_levels)"`

---

### Task 2: The lower bound — too-low quests are not taken and are given up

**Files:**
- Modify: `crates/fg-route/src/plan/availability.rs:108-155` (`can_accept`, new `too_low` next to `gray_dead_end`)
- Modify: `crates/fg-route/src/plan/step.rs:195-215` (`abandon_hopeless`)
- Modify: `crates/fg-route/src/plan/evaluate.rs:449` (`why_invalid`: new reason)
- Modify: `crates/fg-route/src/plan/spot.rs` (the place chosen for an objective: skip places out of the window)
- Test: `crates/fg-route/src/plan/tests.rs`

**Interfaces:**
- Consumes: `Params::level_window` (Task 1).
- Produces: `Availability::too_low(&self, s: &State, q: &Quest, i: u32) -> bool` — true when the window is on, the quest is not `mandatory`, not `replaying`, and some kill objective not yet done (`s.objectives[i] & (1 << k) == 0`) has **every** place (`o.places()`) with `mob_level < window.start()`.
- Produces: `why_invalid` reason string `"mobs too low for the level window"`.

- [ ] **Step 1: Write the failing tests** (build on `quests_at_start`)

- `too_low_quest_is_not_taken`: character level 15, quest with mobs level 10 → `can_accept` false; mobs level 11 → true.
- `too_low_in_the_log_is_given_up`: the quest is in the initial log, level 15, mobs level 10 → after the first stop the quest is abandoned (`s.turned[i]` true, `s.accepted[i]` false, no XP for it).
- `done_objectives_keep_the_turn_in`: same, with the kill objective already done → not abandoned, turned in.
- `mandatory_low_quest_stays`: same as the first with `mandatory = true` → `can_accept` true.
- `low_place_skipped_for_a_place_in_window`: one objective with two places at mob levels 9 and 13, character 15 → accepted, and the stop goes to the level-13 place.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p fg-route too_low low_place done_objectives mandatory_low`
Expected: FAIL (no `too_low`).

- [ ] **Step 3: Implement `too_low`, use it in `can_accept`, `abandon_hopeless` and `why_invalid`, and filter the places in `spot.rs`**

- [ ] **Step 4: Run the crate's tests**

Run: `cargo test -p fg-route`
Expected: all PASS.

- [ ] **Step 5: Commit**

`git commit -m "feat(route): quests with mobs more than levels_below under the character are left out"`

---

### Task 3: Farming on the way, grind spots, hub default

**Files:**
- Modify: `crates/fg-route/src/plan/on_the_way.rs:229`, `crates/fg-route/src/export/grind_spots.rs:182` (use the window)
- Modify: `crates/fg-route/src/params.rs` default `camp_radius: 40.0`
- Test: `crates/fg-route/src/plan/tests.rs`

**Interfaces:**
- Consumes: `Params::level_window` (Task 1).
- Produces: `Params::farm_levels(&self, level: i64) -> RangeInclusive<i64>` — `level - farm_below ..= level + farm_above`, intersected with `level_window` when on. Both call sites use it.

- [ ] **Step 1: Write the failing tests**

`farm_window_follows_strict_levels`: defaults, `farm_above = 2`, `farm_below = 6`, level 20 → `farm_levels(20) == 16..=20`; with `strict_levels = false` → `14..=22`. `hub_mode_is_on_by_default`: `Params::default().camp_radius == 40.0`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p fg-route farm_window hub_mode`
Expected: FAIL.

- [ ] **Step 3: Implement `farm_levels`, use it at both call sites, change the `camp_radius` default**

- [ ] **Step 4: Run the crate's tests**

Run: `cargo test -p fg-route`
Expected: all PASS. Tests needing `camp_radius = 0` already set it (`quests_at_start`); any other that breaks sets it explicitly.

- [ ] **Step 5: Commit**

`git commit -m "feat(route): farming follows the level window; quest hubs on by default"`

---

### Task 4: App settings

**Files:**
- Modify: `app/src/api.ts:68` (type `Params`: `strict_levels: boolean; levels_below: number;`)
- Modify: `app/src/paramSchema.ts` (next to the `camp_radius` entry, ~line 88)
- Check: the app's i18n (`npm run check:i18n`) for the new labels

**Interfaces:**
- Consumes: the two `Params` fields (Task 1), serialized under the same names.

- [ ] **Step 1: Add the two entries**

`strict_levels`: kind boolean (as other boolean entries in the file), label "Jamais de mobs au-dessus de mon niveau", help "Les quêtes ne vous envoient que contre des mobs de votre niveau ou jusqu'à N niveaux en dessous ; plus bas ou grises, elles sont laissées." `levels_below`: kind number, label "Mobs au plus N niveaux en dessous", unit "niveaux", min 0, max 10, step 1. French labels like the rest of the file (the i18n step translates).

- [ ] **Step 2: Verify**

Run (in `app/`): `npm run build && npm run lint && npm run check:i18n`
Expected: no errors; add the translations `check:i18n` asks for.

- [ ] **Step 3: Commit**

`git commit -m "feat(app): settings for the level window"`

---

### Task 5: End-to-end on the real character, desktop build

**Files:**
- Create: `scripts/check-level-window.mjs` (or a Rust test in `crates/fg/`, whichever reads the exported guide more simply) — reads a guide JSON and lists each kill step's character level and mob level.

- [ ] **Step 1: Generate a guide from the real profile**

Run the `fg` CLI on `data/forever.sqlite.zst` for the user's character (Undead Warlock 16, The Crossroads, from `WTF/Account/DINERS66/SavedVariables/Factoruide_Profile.lua`; see `fg route --help` for the profile/start option) to level 30.

- [ ] **Step 2: Check it**

Run: `node scripts/check-level-window.mjs <guide.json>`
Expected: 0 kill steps with mob level above the character's level or more than 4 below; the first steps at The Crossroads take every planned Crossroads quest before leaving.

- [ ] **Step 3: Build the desktop app**

Run (in `app/`): `npm run tauri build`
Expected: an installer under `target/release/bundle/nsis/`. Confirm in the app's settings that automatic contributions are off.

- [ ] **Step 4: Commit**

`git commit -m "test: check the level window on a generated guide"`
