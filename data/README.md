# data/ — the game's numbers, in files

Every tunable value in Wars of Ants lives in these four files. The game
does not have a separate copy in code: they are embedded into the core at
build time and are the single source of truth for the shipped balance —
the browser client, native tests, and replays all read the same bytes.

| File | Owns |
|---|---|
| `ants.jsonc` | caste stats (hp, damage, speed, dig capacity), egg/brood costs + incubation, specialists, queen economy |
| `resources.jsonc` | map sources (amounts, harvest speed, counts), storage caps, spoil, spider drops, legacy costs |
| `world.jsonc` | map size, world generation scatter (incl. how often digging reveals storage/nursery soil), spider stats |
| `colony.jsonc` | founding ritual timings, dig speed, nest logistics, loiter/rest |

## How to tune

1. Edit the files. Comments (`// …`, `/* … */`) are allowed and welcome —
   the loader strips them, and the F4 admin drawer's JSON box accepts
   commented documents too. Leave yourself notes like `// too fast at 22`.
2. Rebuild the core and restart the dev server:
   `npm run wasm` (in `client/`), restart vite, hard-reload the page.
   (Or just tell the agent the values — same result.)
3. Quick experiments don't need any of this: open the F4 admin drawer
   (`?admin=1`), tune live, and when you like the values, copy them from
   the drawer's JSON box into these files to make them permanent.

Every edit here is a visible git commit, and the rules digest (shown in
the drawer and folded into the deterministic canon) changes with it —
old replays stay honest because they carry their own rules.

## Rules of the road

- **Keys may live in any file** — the split is thematic for humans; the
  loader merges top-level keys by name. But **each key in exactly one
  file**: a duplicate is a load error, not a silent override.
- **Unknown or missing keys are load errors**, listed with the file name.
  No silent fallbacks — a typo fails loudly at boot.
- **◇ng** in a comment marks keys that only take effect on the *next*
  generated world (map shape, scatter, source counts). Everything else
  applies live — but for files, since they are baked in at build time,
  "live" still means: new game after rebuild.
- Units: times in seconds, distances in tiles, tick counts are sim ticks
  (60/s). A key's comment next to the value is its glossary entry.
- `dirt_capacity` sits inside each caste's stats. Only workers and the
  founding queen dig; the others carry `0` because they are refused at
  the dig order itself (that refusal is game code, not data).
- Validation guards live in `core/src/rules.rs::validate()` (e.g. worker
  and queen `dirt_capacity` must be ≥ 1, brood must cost something).

## What is NOT in these files

Behavior is code, not data: which castes may dig, the queen's dig-speed
special case, food carrying (always 1 unit per trip), AI roles (honey
secretion, medic rescue), worldgen placement attempts. Adding a whole
new ant type is a code ticket — its *numbers* then live here next to the
others.
