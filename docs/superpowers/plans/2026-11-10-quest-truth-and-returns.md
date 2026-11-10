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

## Checks

- `luajit addon/tests/smoke.lua` with stubbed `C_QuestLog.RequestLoadQuestByID` and gossip.
- `cargo test -p fg-route -p fg-contrib`.
- Regenerate zev from the current save; 9266-9268 absent; level window and returns checks.
