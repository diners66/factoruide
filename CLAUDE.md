# Factoruide fork: context from the first session (2026-11-08)

This is our fork of the original repo, cloned here. Remotes: `origin` = git@github.com:diners66/factoruide.git (our fork),
`upstream` = git@github.com:brouznouf/factoruide.git (original). Sync with `git fetch upstream`.
Live site of the original: https://factoruide.brouznouf.com/

This file is ours and is not committed yet. It holds what we found about the project before deciding
what to build. The next step is to decide: fork it as is, reuse the architecture for another
game or domain, or take only the "heavy compute in the browser, static host" pattern.

## What it is

A WoW Classic leveling-route optimizer (editions: WoW Forever, Classic Era, TBC). It computes the
fastest quest order to reach a target level, then exports a guide to an in-game Lua addon. It ships
as three products from one repo: a desktop app (Tauri), a web app (static, the same pages), and the addon.

## Tech stack

| Layer | Stack |
|---|---|
| Frontend | React 19, TypeScript ~7, Vite 8 (`@vitejs/plugin-react`), plain CSS with custom properties (no Tailwind), oxlint |
| Desktop | Tauri 2 (`app/src-tauri`), plugins dialog/process/updater |
| Engine | Rust (~2.8 MB of source, `rust-toolchain.toml`) built for `wasm32-wasip1-threads`, producing an ~8.7 MB `fg_web.wasm` |
| Rust crates | rusqlite 0.41 + libsqlite3-sys (SQLite bundled, built thread-safe), serde/serde_json, toml, zstd, miniz_oxide, quick_cache, anyhow, ahash/hashbrown |
| WASM runtime | Hand-written WASI shim in a Web Worker (`app/src/web/worker.ts`; `host.ts` starts it). `wasi thread-spawn` starts one Web Worker per Rust thread, all sharing memory through `SharedArrayBuffer`/Atomics. The worker reserves 3.75 GB of WASM memory. HTTP goes through synchronous XHR. |
| Cross-origin isolation | Cloudflare sends COOP/COEP. As a fallback, `coi.js` registers a service worker that adds the headers on any host. |
| Data | `data/<edition>.sqlite.zst` quest databases, fetched and read in place by SQLite inside the WASM |
| Storage | IndexedDB only (no backend or accounts), plus a Save/Restore backup file |
| Hosting | Cloudflare Pages, deployed by `.github/workflows/web.yml` through `scripts/build-web.sh`. That build needs wasi-sdk 34. `web/functions/v1/[[path]].js` relays `/v1/*` to the author's contribution server. |
| Tauri in the browser | `web/shims` replace the `@tauri-apps/*` modules (`vite build --mode web`) |
| Addon | Lua (`addon/Factoruide_Guide`, `addon/Factoruide_Profile`), published on CurseForge. Exchange strings are `FGP1:` (export) and `FGG1:` (import): raw DEFLATE with the LibDeflate printable alphabet (`crates/fg-web/src/fgstring.rs`). |

## Structure

```
crates/
  fg-route/     the optimizer (core IP)
    model/        quests, items, loot, NPCs, prereqs, dungeons, rewards (loaded from SQLite)
    plan/         simulation + search: alns.rs (Adaptive Large Neighbourhood Search), moves,
                  evaluate, sim, travel, combat, hearth, on_the_way, together (group play)...
    terrain/      walking paths, cliffs/fall cost, gates between zones
    job/          a planning request: character, race, group, flights, reputation, timeline
    export/       Lua guide, checkpoints, diff, vendors, grind spots, polestar (Forever packs)
    world.rs xp.rs power.rs profession.rs
  fg-web/       browser backend (in-memory FS, synced to IndexedDB by the page)
  fg-app/       desktop backend (maps.rs, studio.rs)
  fg-library/  fg-contrib/  fg/ (CLI)
app/            React + Tauri app (src/web = browser host)
web/            shims, Cloudflare function, README on how the web build runs
addon/          WoW addon (Lua)
data/           quest DBs     overrides/  hand data fixes (overrides/database/*.toml)
scripts/        build-web.sh etc.     docs/
```

How it works (details in `ARCHITECTURE.md`):
1. **Model** every quest the character can do.
2. **Simulate** a route stop by stop: travel, flight paths, hearthstone, fights, loot drop rates, XP and level-ups.
3. **Search** with ALNS for the route with the lowest simulated time.

Also worth reading: `web/README.md` (how the web build runs) and `NOTICE.md` (data sources).

## Licensing (matters for "can we do the same")

- **Code:** MIT. Fork, modify and sell are allowed; keep the copyright notice.
- **Quest databases (`data/`):** GPL-3.0. If we redistribute them, our changes must be shared under the same license.
- **Questie/QuestieDB data:** no license. Its maintainers say to treat it as "all rights reserved, use at own risk".
- **Maps, names and game data:** Blizzard's. Fine for a free fan tool; don't sell it and don't use Blizzard branding.
- **Other sources:** VMaNGOS (GPL-2.0), CMaNGOS (GPL-3.0), AllTheThings/pfQuest (MIT), EG Link (CC BY 4.0), Wowhead tooltips.

## Difficulty notes

- The value is in `fg-route` (~85 Rust files of simulation and search) and in the data pipeline (merging sources plus `overrides/`), not in the stack.
- The multithreaded WASM build needs wasi-sdk, a thread-safe SQLite build, COOP/COEP headers, and a large memory reservation, which is heavy on phones.
- The clone is about 1.6 GB, mostly `data/`.
