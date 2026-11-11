# Forever Content First Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** WoW Forever guides take the quests that exist only in Forever whenever they are reasonably on the way, and fill the gaps with the older quests, instead of treating every quest alike.

**Architecture:** A generated list of Forever-exclusive quest IDs (`overrides/forever/exclusive_quests.toml`, made by a script that compares the three editions' databases) is loaded into `Overrides`, carried to the model like `quest_overrides`, and marks `Quest::exclusive_content`. The score then charges `Params::exclusive_quest_weight` seconds for each such quest the route leaves out, the way `profession_quest_weight` does for profession quests. Classic and TBC have no list, so nothing changes for them.

**Tech Stack:** Rust 1.99 (`crates/fg-route`, `crates/fg`), Node 24 (`node:sqlite`) for the generator script, React + TypeScript (`app/src`) for the setting.

**Spec (approved in chat, 2026-11-11):**
- "Forever content first": plan with every Forever quest, but strongly prefer the quests exclusive to WoW Forever and use older quests only to fill gaps. The user chose this over "exclusive only", which cannot level past 40: there are 41 exclusive quests in the 40s and 2 in the 50s.
- **Exclusive** means a quest of the Forever data (`m_quest` status `available` or `incomplete`) whose ID is in neither the Classic nor the TBC data (`m_quest`), nor in their clients' quest tables (`client_questv2`). On today's data that is about 1,150 quests (886 available, 265 incomplete), almost all with IDs 86000–99999, plus Goblin Lockpicks (78270).
- The client's quest table is not used to *exclude* quests: it leaves out many real vanilla quests (all 562 available quests missing from the Forever client are missing from the Classic and TBC clients too). The addon's server check (Availability.lua) stays the guard against quests the server lacks.

## Global Constraints

- Default `exclusive_quest_weight = 300.0` seconds per exclusive quest left out (the same scale as `profession_quest_weight`); 0 turns the preference off. Task 5 measures 300 against 600 and may change the default with a ledgered ruling.
- Only quests the character could still use count: `level > xp::Rules::gray_level(profile.from_level)` and `min_level <= profile.to_level`, not `mandatory` (already forced), once per group of exclusive alternatives.
- New fields deserialize from older saved requests and profiles (`#[serde(default)]`, as the other `Params` fields).
- Classic and TBC guides are unchanged: no `exclusive_quests.toml` in `overrides/classic` or `overrides/tbc`.
- `cargo test -p fg-route`, `cargo clippy --all-targets`, `cargo fmt --all -- --check` stay green; work on `main` of our fork as before (the user merges there).

## Review Focus

1. A quest exclusive to Forever but gray at the guide's start, or above its target level: not counted (no constant penalty that steers nothing). Test in Task 3.
2. Exclusive alternatives (the same quest for each faction or race, `Quest::exclusive`): counted once, not once per alternative. Test in Task 3.
3. A saved request or guide from before this change (no `exclusive_quest_weight` key) loads with the default and no error. Test in Task 3.
4. Classic and TBC editions load no exclusive quests and plan as before. Test in Task 1.
5. The weight does not turn a route into a tour of every exclusive quest: the end-to-end check reports the time the guide gives up for them, and the default is the value that keeps that time reasonable (Task 5).

---

### Task 1: The list of exclusive quests

**Files:**
- Create: `scripts/forever-exclusive.mjs`
- Create: `overrides/forever/exclusive_quests.toml` (generated)
- Modify: `crates/fg-route/src/job/overrides.rs` (field on `Overrides`, parsed in `parse` next to `quests.toml`, ~line 262)
- Test: `crates/fg-route/src/job/overrides.rs` (existing `#[cfg(test)]` module, ~line 274)

**Interfaces:**
- Produces: `Overrides::exclusive_quests: Vec<i64>`, sorted, from `exclusive_quests.toml` shaped as `quests = [ ... ]`, with a header comment saying how it was made and when.
- Produces: `node scripts/forever-exclusive.mjs`: reads `data/forever.sqlite`, `data/classic.sqlite` and `data/tbc.sqlite` (unpacked by any `fg --edition <e>` command) with `node:sqlite`, applies the spec's definition, and writes the TOML with 12 IDs per line.

- [ ] **Step 1: Write the failing test**

`forever_has_its_exclusive_quests`: `load(Edition::Forever).exclusive_quests` contains 78270 (Goblin Lockpicks) and 91723 (Delicate Instruments), does not contain 783 (A Threat Within, in every edition), and is sorted. `load(Edition::Classic)` and `load(Edition::Tbc)` have it empty.

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p fg-route forever_has_its_exclusive_quests`
Expected: FAIL to compile (`exclusive_quests` unknown).

- [ ] **Step 3: Write the script, generate the file, add the field**

Run: `target/release/fg.exe --edition classic travel "Elwynn Forest,40,60" "Elwynn Forest,42,62"` and the same with `--edition tbc` if `data/classic.sqlite` / `data/tbc.sqlite` are missing, then `node scripts/forever-exclusive.mjs`.
Expected: the script prints the count (about 1,150) and writes the file.

- [ ] **Step 4: Run the test and the crate**

Run: `cargo test -p fg-route`
Expected: all PASS.

- [ ] **Step 5: Commit**

`git commit -m "feat(data): the quests only WoW Forever has, listed from the editions' data"`

---

### Task 2: The model knows them

**Files:**
- Modify: `crates/fg-route/src/model/types.rs` (`Quest`, next to `zone_forced` ~line 577)
- Modify: `crates/fg-route/src/job/character.rs:147` (carry the list like `quest_overrides`)
- Modify: `crates/fg-route/src/model/quest.rs:166` area (where `profile.quest_overrides` is applied: set the flag there)
- Modify: the `Profile` struct that holds `quest_overrides` (add the list next to it)
- Test: `crates/fg-route/tests/it/unavailable.rs` (same fixture and request helper)

**Interfaces:**
- Consumes: `Overrides::exclusive_quests` (Task 1).
- Produces: `Profile::exclusive_quests: Vec<i64>`, `Quest::exclusive_content: bool` (true when the quest's ID is in the list).

- [ ] **Step 1: Write the failing test**

`forever_quests_are_marked_in_the_model`: with the `forever_druid` fixture, build `Overrides` as the test file does, then set `overrides.exclusive_quests` to the IDs of the first two quests of a first `job::prepare`. Prepare again: exactly those two quests have `exclusive_content`, and no other quest has it.

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p fg-route --test it forever_quests_are_marked`
Expected: FAIL to compile (`exclusive_content` unknown).

- [ ] **Step 3: Add `Profile::exclusive_quests`, carry it in `character.rs`, set `Quest::exclusive_content` where the quest overrides are applied**

- [ ] **Step 4: Run the crate's tests**

Run: `cargo test -p fg-route`
Expected: all PASS.

- [ ] **Step 5: Commit**

`git commit -m "feat(route): the model marks the quests only WoW Forever has"`

---

### Task 3: The score prefers them

**Files:**
- Modify: `crates/fg-route/src/params.rs` (field next to `profession_quest_weight` ~line 395, default ~line 664)
- Modify: `crates/fg-route/src/plan/state.rs:571` (`Breakdown`, next to `missing_profession_quests`)
- Modify: `crates/fg-route/src/plan/step.rs:1047-1067` (count and score), new method next to `missing_profession_quests` (~line 1097)
- Modify: `crates/fg/src/route.rs:136-143` (print the count)
- Test: `crates/fg-route/src/plan/tests.rs` (next to `profession_quests_are_kept_once_gray`, ~line 532)

**Interfaces:**
- Consumes: `Quest::exclusive_content` (Task 2).
- Produces: `Params::exclusive_quest_weight: f64` (default 300.0), `Breakdown::missing_exclusive_quests: u32`, `Simulator::missing_exclusive_quests(&self, s: &State) -> usize`, which counts as the Global Constraints say. The score adds `missing_exclusive_quests * exclusive_quest_weight`. The CLI line gains `| Forever quests missing {n}`.

- [ ] **Step 1: Write the failing tests**

`forever_quests_left_out_cost_the_score`, built on `quests_at_start(3)` like the profession test (`p.profile.to_level = 20`):
- quests 0 and 1 `exclusive_content = true`, with `exclusive_quest_weight = 300.0`;
- a route doing both → `missing_exclusive_quests == 0`; without quest 1 → 1, and that score is more than 270 s worse;
- quest 0 and quest 2 made exclusive alternatives of each other (`exclusive` lists), quest 2 also `exclusive_content`, neither done → counted once;
- quest 1 at level 1 with `p.profile.from_level = 20` (gray at the start) → not counted; quest 1 with `min_level = 25` → not counted;
- `exclusive_quest_weight = 0.0` → the two scores are equal.

`old_requests_keep_the_forever_weight`: `serde_json::from_str::<Params>("{}")` has `exclusive_quest_weight == 300.0`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p fg-route forever_quests_left_out old_requests_keep_the_forever`
Expected: FAIL to compile.

- [ ] **Step 3: Implement the field, the count, the score and the CLI line**

- [ ] **Step 4: Run the crate's tests and clippy**

Run: `cargo test -p fg-route && cargo clippy --all-targets`
Expected: all PASS, no warnings. Upstream tests whose route changes because of the weight set `exclusive_quest_weight = 0.0` (list them in the commit message), as was done for `strict_levels`.

- [ ] **Step 5: Commit**

`git commit -m "feat(route): guides take WoW Forever's own quests first (exclusive_quest_weight)"`

---

### Task 4: The app setting

**Files:**
- Modify: `app/src/api.ts` (type `Params`: `exclusive_quest_weight: number;`)
- Modify: `app/src/paramSchema.ts` (next to `profession_quest_weight` if listed, else next to `return_weight`)
- Check: the app's i18n (`npm run check:i18n`)

**Interfaces:**
- Consumes: `Params::exclusive_quest_weight` (Task 3), serialized under that name.

- [ ] **Step 1: Add the entry**

Kind number, label "Quêtes propres à WoW Forever d'abord", unit "secondes par quête laissée", min 0, max 1800, step 60, help "Chaque quête qui n'existe que dans WoW Forever et que le guide laisse lui coûte ce temps : il les fait dès qu'elles sont sur le chemin, les anciennes quêtes comblent les trous. 0 : aucune préférence." Shown only for the Forever edition if the schema has a per-edition filter; otherwise always (it does nothing elsewhere).

- [ ] **Step 2: Verify**

Run (in `app/`): `npm run build && npm run lint && npm run check:i18n && npm test`
Expected: no errors; add the translations `check:i18n` asks for.

- [ ] **Step 3: Commit**

`git commit -m "feat(app): setting for Forever's own quests first"`

---

### Task 5: Real guide, before and after

**Files:**
- Create: `scripts/check-exclusive.mjs`: reads a guide JSON and `overrides/forever/exclusive_quests.toml`, prints the exclusive quests turned in, by level band (0-9, 10s ... 60).

- [ ] **Step 1: Generate three guides from the real profile**

Run `fg route --race undead --class warlock --start <profile.json> --to-level 60 --effort 1` (the same seed each time, profile from `Factoruide_Profile.lua` as in the level-window plan), with `--param exclusive_quest_weight=0`, `=300` and `=600`, into separate `--routes` directories.

- [ ] **Step 2: Check them**

Run: `node scripts/check-exclusive.mjs <guide.json>` for each, and `node scripts/check-level-window.mjs` on the 300 guide.
Expected: more exclusive quests at 300 than at 0, and more again at 600; the level window still holds (only the exempt class quests out of it). Write a table into this plan (exclusive quests by band, total time, returns) and pick the default: the higher value unless it costs more than about 3% of the 0-weight guide's time. Ledger the choice.

- [ ] **Step 3: Install the chosen guide**

Copy the chosen guide into the addon's `Data.lua` as was done for the seed-42 guide (back up the current one first, next to it).

- [ ] **Step 4: Commit**

`git commit -m "test: Forever's own quests in a real guide, by weight"`

## Result (2026-11-11)

Zev (undead warlock) 18-60, seed 42, effort 1:

| Weight | Forever quests turned in (10s / 20s / 30s / 40s) | Usable ones left out | Time | Returns |
|---|---|---|---|---|
| 0 | 37 of 625 (6 / 12 / 18 / 1) | 35 | 92h06 | 16 |
| 300 | 41 of 612 (6 / 14 / 20 / 1) | 31 | 91h03 | 18 |
| 600 | 40 of 615 (6 / 13 / 20 / 1) | 32 | 91h28 | 17 |

The default stays 300: 600 takes no more of them. The level window holds in all three (only
the exempt warlock chain out of it). The weight moves little because this character can use
only about 72 of the 1,173 quests, and the guide already took half: the others are crafting
quests (a profession the guide does not level), other races' and classes', or in Forever's new
zones (Zephras Isle, Riverglades, Shen'dralas, Darkspear Islands), most of which the planning
log reports as having no reachable quest giver. That is data to complete (the zones' NPCs and
the 615 Forever quests with no data at all), not weighting.

The 300 guide is in the game's Factoruide_Data (the earlier Data.lua kept beside the run).

### After the review fix (the search's window estimate now exact for Forever quests)

| Weight | Forever quests turned in (10s / 20s / 30s / 40s) | Usable ones left out | Time | Returns |
|---|---|---|---|---|
| 0 | 37 of 625 (6 / 12 / 18 / 1) | 35 | 92h06 | 16 |
| 300 | 42 of 616 (6 / 14 / 21 / 1) | 30 | 92h41 | 21 |
| 600 | 42 of 614 (6 / 14 / 21 / 1) | 30 | 92h42 | 22 |

300 and 600 take the same quests: the 30 left are worth more than 10 minutes each to reach, or
never takeable in this guide (the level window gives up a quest whose mobs fell too low, which the
count does not know). The default stays 300. The guide installed in the game (91h03, 41 Forever
quests, before the fix) stays: as many Forever quests, 1h40 faster than this run's 300 guide.
