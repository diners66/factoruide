# Quests the game really has, and fewer returns: plan

Branch: `level-window`. Started 2026-11-10.

## Why

The zev-18-60 guide sent the player to Mupsi Shacklefridd (Ratchet, step 24) for Mending Old
Wounds (9267): a TBC quest the WoW Forever server does not have. The data lists it from Wowhead,
Questie, ATT, vmangos and pfQuest; its `in_client = 0` flag is no help (810 quests carry it,
real repeatables such as the Scourgestones too). The same chain comes back at steps 1293 (9268)
and 2036 (9266). The game itself is the only reliable judge.

Checked in game (2026-11-10): `C_QuestLog.RequestLoadQuestByID` answers
`QUEST_DATA_LOAD_RESULT(questID, success)` within a second on Forever: 9267, 9268, 9266 false;
899, 887, 5402, 2523 true.

## 1. The server is asked about the guide's quests (addon)

- New `Factoruide_Guide/Availability.lua`. After login (and when another guide is chosen) it asks
  the server about each quest the guide accepts that is neither in the log nor done, a few per
  second, only its own requests (other addons ask too).
- A quest the server does not know is asked once more later; two "no" answers make it missing.
- Missing quests: kept in `FactoruideDB.characters[key].unavailable[questID] = { why = "missing",
  level, time }`, and the guide passes their steps (skipped quests, as "Skip the dungeon" does),
  saying so in the chat.
- A "yes" later (or the quest accepted) removes it.

## 2. A giver that does not offer the quest (addon)

- When a quest window opens (gossip, greeting, or a quest's details shown at once) at the NPC or
  object of a current accept step, and the quest is not offered, not in the log and not done:
  `why = "not offered"`, with the NPC, the step passed.
- Not judged when the window lists a quest to turn in there (a turn-in may unlock it), the
  character is below the step's level, or a turn-in step at the same giver comes first.

## 3. The planner leaves them out

- `fg-contrib` profile reading adds `unavailable = [{ id, why, npc, level }]` from
  `characters[key].unavailable` to each profile.
- `StartState.unavailable`; `job::prepare` adds them to `params.excluded_quests` (a note says
  which and why). Quests needing them go too (the model drops quests whose prerequisites are
  gone).

## 4. Fewer returns to an area

- Measure: `scripts/check-returns.mjs` (23 avoidable returns on zev-18-60).
- Done in the last tidying (`plan/tidy.rs`), which only looked 40 stops back within 100 yards
  for 30 seconds: a quest's objective is looked for up to 400 stops back, within
  `return_radius` (200 yards; 300 took the town next to a camp for the camp), once its quest is
  taken or in the starting log, and moved there for at most `return_slack` (300 seconds).
  Tests: `plan/tests/returns.rs`. Before/after on the same save: `--param return_slack=-1`.

### In the search (2026-11-11)

- The simulation counts returns (`Breakdown::returns`, printed by `fg route`): an objective within
  `return_radius` of an area where one was done (`State::visits`, the last 40 areas), the route
  gone more than `return_away` (500 yards) since, its quest taken before that visit
  (`State::taken`: the stop that really took it, at hand included; the starting log counts as
  taken before). Each costs the optimizer `return_weight` (300 s; 0 = off, nothing tracked).
- The count is a proxy close to `scripts/check-returns.mjs` (20 vs 17 on one guide), not the
  same measure: the script uses 250 yards (200 here), looks back without limit (40 areas here),
  leaves out the starting log and returns whose mobs were above the character at the first visit,
  and measures "away" from the current objective. Both count objective steps, not the trips a
  player feels (2 by hand on the same guide in game): read "7 fewer returns" as a trend.
- A return is counted only for a quest taken before the earlier visit: the search may avoid one
  by taking the quest after that visit instead, which the player still walks back for. Kept as
  the plan defines it (review, 2026-11-11); worth revisiting if guides show it.
- Zev 18-60, same save, script count (sim count):

  | Seed | Off | On (300 s) |
  |---|---|---|
  | 42 | 92h45, 25 | 92h53, 20 (15) |
  | 1 | 95h50, 25 | 96h10, 23 (16) |
  | 2 | 95h41, 34 | 98h08, 19 (18) |

  About 7 fewer returns for about an hour on average; the seeds alone differ by 3 hours.

### Review fixes (2026-11-11)

- Addon: "not offered" judged only when nothing on the way may unlock it (a turn-in or an accept
  elsewhere still to do, prerequisites `pre`/`pa` not done, `min` above, a gray quest `ql`), the
  list window not forgotten on close; missing quests asked again at each check, "not offered"
  marks dropped when offered or at a level up; marks in the FGP1: export for the web app.
- Planner: in a group, "not offered" leaves a quest out only when to every player.
- Search: `State::taken` keeps only the quests in the log; the incremental evaluation compares
  returns by order (`State::same_returns`), not by stop index. Zev 18-60, effort 0.3, one run
  each, nothing else running:

  | | weight 0 | weight 300 |
  |---|---|---|
  | before | 113 s, 99h56 | 123 s, 99h11 |
  | after | 112 s, 99h56 (same guide) | 117 s, 98h21 |

  Off: unchanged. On: the search costs 4% more time instead of 9%, and finds a better route
  (the exact early stop is taken more often, so fewer moves are judged on an estimate).

## Checks

- `luajit addon/tests/smoke.lua` with stubbed `C_QuestLog.RequestLoadQuestByID` and gossip.
- `cargo test -p fg-route -p fg-contrib`.
- Regenerate zev from the current save; 9266-9268 absent; level window and returns checks.
