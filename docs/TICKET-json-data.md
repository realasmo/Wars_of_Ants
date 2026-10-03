# Ticket — data files: the game's numbers move into data/*.jsonc

**Wave:** J (data files) · **Shipped:** 2026-10-03 · **Version:**
0.1.05.60-dev · **Status:** shipped (96 core tests, e2e ×green with
native-vs-WASM determinism byte-match)

## Want

Every tunable value in editable, versioned, human-readable files
(`ants.json`, `resources.json` — the user's naming), so balance can be
tuned by hand without touching code. "The game should read from it."

## Decisions

- **Baked in at build (user-confirmed), not fetched at runtime.** The four
  files are embedded via `include_str!` and merged by `core/src/data.rs`.
  One source of truth: browser, native tests, replays, and the determinism
  fixture all parse the same embedded bytes — no dual load paths, no
  boot-time fetch failure mode. Editing cost: `npm run wasm` → restart
  vite → hard reload (or tell the agent).
- **Four thematic files** (`ants` / `resources` / `world` / `colony` +
  `data/README.md` with the workflow). The split is convention for human
  editors: the loader merges **top-level keys by name**, so a key parses
  wherever it lives — but **each key exactly once** (duplicate across
  files = loud load error; unknown/missing keys = load errors with file
  names). Tests pin full coverage: every serialized `GameRules` field
  appears in exactly one file.
- **JSONC (comments allowed).** `strip_jsonc` (string-aware `//` and
  `/* */` stripping, newlines preserved so error line numbers still match)
  runs in the file loader **and in `Sim::parse_rules`** — the admin
  drawer's JSON box and dev-rules replays accept annotated documents too.
  Comments never reach the rules digest.
- **`GameRules::default()` delegates to the loader** (`data::shipped()`,
  `LazyLock`, panics with the full error list on invalid shipped files).
  The ~200-line hand-written `Default` literal block is deleted — code and
  data can no longer drift. `rules.rs` keeps the schema, `validate()`,
  `digest()`, and the admin field registry.
- **Schema move: `dirt_capacity` global → per-caste `UnitStats`.** The
  user asked for "ant-worker: dig-blocks" per type. Worker and founding
  queen default 2; soldier/honey/medic carry an honest 0 (they're refused
  at the dig order itself — that refusal stays code). `validate()`
  requires worker/queen ≥ 1 (capacity 0 would let them dig while loaded
  and never trigger the haul-out). Four read sites updated (dig gate in
  `commands.rs`, dirt pickup in `systems.rs`, two haul-out triggers in
  `ai.rs` — all via `stats_for(caste)`); the admin drawer renders it per
  caste automatically via `unit_fields`.

## Values-unchanged proof

One-time check (recorded here): the pretty JSON of the compiled-in
defaults before the change vs the file-derived defaults after differs by
exactly the intended schema delta — six per-caste `dirt_capacity` entries
added (2/2/0/0/0/0 for queen/worker/soldier/spider/honey/medic), the one
top-level `dirt_capacity: 2` line removed — all 64 other values
byte-identical. The rules digest rotates once (new field in the Debug
rendering); nothing persisted stores digests, and the e2e fixture
regenerated cleanly with a native-vs-WASM byte-match on the new schema.

## Verification

- New: `core/tests/data.rs` (files load+validate+round-trip digest-neutral;
  every field in exactly one file; merge failure modes — duplicate,
  unknown key, non-object, broken file — each loud and named; commented
  documents parse through `parse_rules`; per-caste capacity defaults +
  validation) + four `strip_jsonc` unit tests in `core/src/data.rs`.
- Existing: all rules/founding/indicators suites green unchanged (except
  `meta_registry…`'s pinned `unit_fields` list gained `dirt_capacity`).
- `cargo fmt`/`clippy -D warnings`/`cargo test` clean; `npm run e2e` ×green
  (fixture regenerated under the new schema, determinism match=true
  6455/6455 chars, admin drawer asserts pass with the new per-caste
  leaves).

## Honest stays-code list (not in the files)

Who may dig (worker + founding queen), the queen dig-speed special case
(`queen_dig_time` is data; the *branch* is code), food carry = 1 unit per
trip, AI roles (honey secretion, medic rescue), worldgen placement
attempt counts/margins/gaps, source display names, wire codes, caste
behaviors generally — `Caste` is a closed enum, so **adding a new ant type
is a code ticket**; its *numbers* then live in `data/ants.jsonc` next to
the others.

## Known edges

- Live-down-tuning `dirt_capacity` below an ant's current load strands the
  carrier until it dumps (pre-existing behavior when the field was global;
  unchanged by the move).
- A key placed in the "wrong" thematic file still works (documented in
  `data/README.md`) — the files are conventions, the loader is key-based.
