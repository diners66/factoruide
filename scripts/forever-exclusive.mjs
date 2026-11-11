// Lists the quests only WoW Forever has into overrides/forever/exclusive_quests.toml: quests of
// the Forever data (available or incomplete) whose ID neither the Classic nor the TBC data has,
// nor their clients' quest tables. Guides take them first (Params::exclusive_quest_weight).
//   node scripts/forever-exclusive.mjs
// Needs data/forever.sqlite, data/classic.sqlite and data/tbc.sqlite: any `fg --edition <e>`
// command unpacks them (fg --edition classic travel "Elwynn Forest,40,60" "Elwynn Forest,42,62").
import fs from "node:fs";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { fileURLToPath } from "node:url";

// Paths from the repository, wherever the script is run from.
const repo = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

const open = (edition) => {
  const file = path.join(repo, "data", `${edition}.sqlite`);
  if (!fs.existsSync(file)) {
    console.error(`${file} missing: run any fg command with --edition ${edition} first`);
    process.exit(2);
  }
  return new DatabaseSync(file, { readOnly: true });
};
const ids = (db, sql) => new Set(db.prepare(sql).all().map((r) => Object.values(r)[0]));

const forever = open("forever");
const others = new Set();
for (const edition of ["classic", "tbc"]) {
  const db = open(edition);
  for (const id of ids(db, "SELECT id FROM m_quest")) others.add(id);
  for (const id of ids(db, "SELECT ID FROM client_questv2")) others.add(id);
}
const exclusive = [...ids(forever, "SELECT id FROM m_quest WHERE status IN ('available', 'incomplete')")]
  .filter((id) => !others.has(id))
  .sort((a, b) => a - b);

const lines = [];
for (let i = 0; i < exclusive.length; i += 12) lines.push("    " + exclusive.slice(i, i + 12).join(", ") + ",");
const text = `# Quests only WoW Forever has: in its data (available or incomplete), in neither the Classic
# nor the TBC data nor their clients' quest tables. Guides take them first
# (Params::exclusive_quest_weight). Made by scripts/forever-exclusive.mjs (${exclusive.length} quests,
# ${new Date().toISOString().slice(0, 10)}): run it again after the databases change.
quests = [
${lines.join("\n")}
]
`;
fs.writeFileSync(path.join(repo, "overrides/forever/exclusive_quests.toml"), text);
console.log(`${exclusive.length} quests only WoW Forever has: overrides/forever/exclusive_quests.toml`);
