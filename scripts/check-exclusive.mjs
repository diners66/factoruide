// Counts the quests only WoW Forever has (overrides/forever/exclusive_quests.toml) that a planned
// guide (routes/<name>.json, as `fg route` writes it) turns in, by the character's level band.
//   node scripts/check-exclusive.mjs <guide.json> [<guide.json>...]
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

// Paths from the repository, wherever the script is run from.
const repo = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

const files = process.argv.slice(2);
if (!files.length) {
  console.error("usage: node scripts/check-exclusive.mjs <guide.json> [<guide.json>...]");
  process.exit(2);
}
const toml = fs.readFileSync(path.join(repo, "overrides/forever/exclusive_quests.toml"), "utf8");
const exclusive = new Set([...toml.replace(/#.*$/gm, "").matchAll(/\d+/g)].map((m) => Number(m[0])));
const bands = ["0-9", "10s", "20s", "30s", "40s", "50s", "60"];

for (const file of files) {
  const guide = JSON.parse(fs.readFileSync(file, "utf8"));
  const steps = guide.steps ?? [];
  const turnins = steps.filter((s) => s.kind?.toLowerCase().replace("_", "") === "turnin" && s.quest != null);
  const counts = new Array(bands.length).fill(0);
  let total = 0;
  for (const s of turnins) {
    if (!exclusive.has(s.quest)) continue;
    counts[Math.min(Math.floor((s.level ?? 0) / 10), 6)] += 1;
    total += 1;
  }
  const by = bands.map((b, i) => `${b} ${counts[i]}`).join(" | ");
  console.log(`${file}: ${total} of ${turnins.length} quests turned in are Forever's own (${by}); ${guide.total_time_text ?? "?"}`);
}
