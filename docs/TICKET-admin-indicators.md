# Ticket — admin tuning drawer + activity indicators (shipped 2026-09-28)

Two features, one wave, over the `GameRules` data layer F4 shipped.
Version 0.1.04.58-dev. Everything below is implemented as described.

## Part 1 — admin tuning drawer

**Never a separate server** — an in-client DOM drawer (`?admin=1` on load,
F4 toggles) generated entirely from core data:

- Core surface (wasm + native, `Sim` in `sim/mod.rs` + `lib.rs`):
  - `rules_get()` → `{"digest": "<16 hex>", "rules": {…}}` — serde
    whole-object JSON; unknown fields and wrong types are **parse errors**
    (`deny_unknown_fields`) — a typo can never silently drop a field.
  - `rules_set(json)` → atomic commit: parse + `validate()` (every
    violation listed at once) + apply; or all errors back, nothing applied.
    Live application re-derives entity-copied stats (`Ant::speed`,
    `Combat`, `Predator`) so speed/hp/dmg changes reach existing entities;
    **hp fraction is preserved** across max-hp changes.
  - `rules_default()`, `rules_digest()`, `new_founding_rules(seed, team,
    json)` (throws with the validation errors — no silent default fallback).
  - `GameRules::meta_json()` — the field registry: ordered groups, every
    leaf's scope (`live` / `new_game`), unit-stats and brood shape hints,
    the food-kind list. The panel renders exactly this; there is no
    client-side field table to drift.
- Client (`client/src/admin.ts`): Tweakpane 4 form (LMB-only — IAB-safe),
  12 grouped sections, nested folders for unit stats / brood castes /
  map sources, `◇ng` label suffix on new-game-only fields, craving cycle
  as an editable comma list, a JSON view (paste + load), digest + dirty
  indicator, and the error list under the buttons.
- Commit paths:
  - **Apply (live)** — commits to the running sim; live fields apply now.
  - **Apply & new game** — validates first, then restarts the founding
    game under the rules **keeping the seed** (worldgen tweaks are
    apples-to-apples).
  - Successful commits are logged as `dev-rules` commands (the full JSON)
    and **replay byte-identically** — the native determinism runner
    applies the same bytes, and the rules digest is folded into the canon
    (since F4), so a replay can never silently run under different rules.
- Scope split (from `meta_json`): worldgen-only fields (width/height,
  scatter chances, patches, source placement, collectible counts,
  start_food, sources) are `new_game`; everything else is live — read
  per-tick or re-derived on commit.

## Part 2 — activity indicators

The floating information layer: every player-colony ant carries a
pixel-art icon above its label, always on, never smaller than 16 screen
px. (The first design gated icons to zoom ≤ 0.75 — the rigs carry detail
up close; the user reversed their own rule after live play: icons stay on
at every zoom.)

- **Core** (`sim/snapshot.rs`): a pure projection, no new sim state,
  not part of the canonical digest.
  - `Intent` — off / dig / haul-dirt / haul-home / fight / medic / feeder /
    follow / produce (honey) / **forage(kind)**. Derivation precedence:
    body state (digging/fighting/harvesting) → role jobs (feeder, medic —
    a feeder carrying the craved unit to the queen stays the crown, not a
    haul icon) → mandibles (dirt out, food/eggs/wood/wool home) → other
    jobs → honey-caste produce → off. Downed ants carry no intent (the
    red DOWN label is their story).
  - `Blocked` — the "!" reason, only where the AI genuinely cannot
    proceed: `EmptyPantry` (feeder, craving unsatisfiable),
    `NeedsWater` (medic parked at its patient), `PantryFull` (loaded
    carrier, no pantry cell, nothing soft left to dig), `QueenStarving`
    (the fuse burns and nothing feeds her).
  - Wire **v5**, stride 15: p8 = intent code (forage = 20 + food code —
    the per-resource tint), p9 = blocked code. `snapshot_spec()` exports
    the new groups; the client's boot assertion pins them.
- **Client**:
  - `client/src/art/icons.ts` — the atlas as an embedded art spec: 16×16
    hand-authored pixel grids + palettes, Canvas2D → nearest-neighbour
    textures. One white harvest droplet, tinted per resource at draw.
  - `render.ts` — per-ant icon sprite in the entity container (above the
    label), visible at zoom ≤ 0.75, scale recomputed per frame for the
    16px minimum; the blocked "!" beats the activity icon.
  - **Confirm pulse** — every successful order (move/dig/attack/drop/
    pick-egg/entrance/land/found/brood/follow) draws an expanding gold
    ring and pops the icon in for 0.45s regardless of zoom.
  - Command feedback twin: a refused dig by a non-digger flashes the
    reason ("soldiers can't dig — only workers excavate"; the queen only
    digs while founding) instead of silently walking into the wall.
  - The HUD "Controlling" line carries the intent + blocked reason.

## Verification

- Core: 87 tests (16 new — `tests/rules.rs`, `tests/indicators.rs`):
  JSON round-trip digest stability, atomic refusal, unknown-field
  rejection, live stat re-sync (canonical evidence), custom-rules
  worldgen, meta completeness, replay determinism of commits; intent and
  blocked projections for dig/haul/produce/medic/feeder+empty-pantry+
  queen-starving/needs-water/follow-release/forage-kind/downed.
- e2e (`e2e/smoke.mjs`): drawer auto-open + F4 toggle + pane built +
  digest sync; live commit changes the digest; invalid commit refused
  with both violations listed and nothing applied; `dev-rules` in the
  input log and in the regenerated determinism fixture — native vs WASM
  canon still byte-identical; rules restart resizes the map (64×64,
  spider count applied) keeping the seed; icon layer zoom-gated both
  ways with glyphs assigned; order lands a pulse.
- Visual QC (GLM-Vision loop): icon legibility (pixel-level: the Zz glyph
  paints at 16×16 after the min-size fix), drawer form (hex digest,
  expanded first folder, ◇ng suffixes correct, opaque panel).

## Notes / decisions

- "Unreachable" was considered as a blocked reason but dropped: the AI's
  retry counter is a generic wait primitive (the feeder uses it to stand
  by), so it cannot honestly signal route failure. The four shipped
  reasons are all honestly derivable from live state.
- Tweakpane 4.0.5 publishes broken type resolution (imports
  `@tweakpane/core` without declaring it) — installing `@tweakpane/core`
  explicitly fixes it.
- The drawer is fully opaque: a translucent panel lets world labels read
  through at amplified contrast, which reads as a theming bug.
