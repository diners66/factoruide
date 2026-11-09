// Checks a planned guide (routes/<name>.json, as `fg route` writes it) against the level window:
// every kill step's mobs (the strongest at its spot, `mob_top`) at the character's level or at
// most `below` levels under it.
//   node scripts/check-level-window.mjs <guide.json> [below=4]
// Prints the steps out of the window and exits 1 when there are any.
import fs from "node:fs";

const [file, belowArg] = process.argv.slice(2);
if (!file) {
  console.error("usage: node scripts/check-level-window.mjs <guide.json> [below=4]");
  process.exit(2);
}
const below = Number(belowArg ?? 4);
const guide = JSON.parse(fs.readFileSync(file, "utf8"));
const steps = guide.steps ?? guide.route?.steps ?? guide.plan ?? [];

// Kill objectives, and accepts started by a drop (their step carries the dropping mob's level).
const kills = steps.filter((s) => ["objective", "accept"].includes(s.kind?.toLowerCase()) && s.mob_level != null);
const top = (s) => Math.max(s.mob_level, s.mob_top ?? 0);
const out = kills.filter((s) => top(s) > s.level || s.mob_level < s.level - below);
for (const s of out) {
  const why = top(s) > s.level ? "above" : "too low";
  console.log(`level ${s.level}, mobs ${s.mob_level}-${top(s)} (${why}): ${s.quest_name ?? "?"} — ${s.text}`);
}
console.log(`${kills.length} kill steps, ${out.length} out of the window (mobs from level - ${below} to level)`);
process.exit(out.length ? 1 : 0);
