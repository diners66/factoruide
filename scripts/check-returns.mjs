// Finds the guide's returns to an area: an objective done near (within `near` yards) one done
// earlier, with a trip away (further than `away` yards) between them, for a quest already in
// the log at the first visit and not above the character's level then (it could have been
// done then).
//   node scripts/check-returns.mjs <guide.json> [near=250] [away=500]
import fs from "node:fs";

const [file, nearArg, awayArg] = process.argv.slice(2);
if (!file) {
  console.error("usage: node scripts/check-returns.mjs <guide.json> [near=250] [away=500]");
  process.exit(2);
}
const near = Number(nearArg ?? 250), away = Number(awayArg ?? 500);
const steps = JSON.parse(fs.readFileSync(file, "utf8")).steps;
const dist = (a, b) => (a[0] === b[0] ? Math.hypot(a[1] - b[1], a[2] - b[2]) : Infinity);
const acceptedAt = new Map();
let returns = 0;
steps.forEach((s, i) => {
  if (s.kind === "accept" && !acceptedAt.has(s.quest)) acceptedAt.set(s.quest, i);
  if (s.kind !== "objective" || !s.world) return;
  // The latest earlier objective near here, and whether the route went away since.
  for (let j = i - 1; j >= 0; j--) {
    const o = steps[j];
    if (!o.world) continue;
    if (o.kind === "objective" && dist(o.world, s.world) <= near) {
      const between = steps.slice(j + 1, i).filter((x) => x.world);
      const left = between.some((x) => dist(x.world, s.world) > away);
      const inLog = (acceptedAt.get(s.quest) ?? Infinity) < j;
      // Its mobs were not above the character at the first visit.
      const fit = Math.max(s.mob_level ?? 0, s.mob_top ?? 0) <= o.level;
      if (left && inLog && fit) {
        returns++;
        console.log(`step ${i + 1} (${s.quest_name}: ${s.text}) back near step ${j + 1} (${o.quest_name}), in the log since step ${acceptedAt.get(s.quest) + 1}, level ${o.level} then`);
      }
      break;
    }
  }
});
console.log(`${returns} returns to an area for a quest already in the log`);
