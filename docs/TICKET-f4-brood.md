# F4 — queen X-menu brood production (shipped 2026-09-28)

Wave ticket: design, decisions, and verification record. Costs and mechanics
live in `docs/WORLD-DESIGN.md` (the content bible) and `core/src/rules.rs`
(the data); this file records the F4 implementation decisions.

## Shipped

1. **GameRules foundation** (`core/src/rules.rs`, commit `6286617`): every
   tunable number — the old `balance.rs` consts, `Config`, and the stray
   worldgen literals (rock 0.08, patch min-dist 20, near ring 7–14, spider
   distance 25–45) — graduated into one typed, validated ruleset carried by
   `Sim`. `validate()` returns every invariant violation at once; `digest()`
   (FNV-1a over the Debug render) is folded into `canonical_state` so a
   replay can never silently run under different rules than it was recorded
   with. Behavior-identical graduation (62 tests unchanged). This is the
   substrate of the upcoming admin tuning panel; F4's brood-cost table was
   its first data-driven rows.
2. **Queen X-menu** (`Command::Brood`): X as the queen opens the BROOD menu
   (1–4 or click). Costs are paid from the **physical pantry** (stored
   piles; the ledger moves with them — atomic all-kinds-or-nothing
   withdrawal). Every refusal returns a human reason which the client
   flashes in the help bar (the no-silent-refusals rule). **Founding
   softlock fixed**: 0 eggs + 0 workers is no longer forever — order
   workers for 2p+1w once any protein exists.
3. **Caste costs** (settled 2026-09-27): Worker 2p+1w · Soldier 6p+3w and
   **consumes one living worker** (it despawns into the soldier brood —
   metamorphosis, not duplication) · Honey 1p+8h · Medic 4p+3c. All are
   rules rows (`GameRules.brood`), not code.
4. **Honeydew economy**: `FoodKind::Honeydew` is the 4th colony resource
   (pantry rules apply — silver keeps it, cap per cell, ledger synced).
   **Nettle** map sources (src 7: 15–22 units, 10s/unit, 4 scattered,
   unknown until sighted) until Wave D's aphids. **Honey ants** secrete
   1 honeydew per 240s as a physical unit where they stand: on silver
   pantry soil it banks instantly; anywhere else it drops as spoiling loot
   workers must haul home.
5. **Medics + fallen ants**: combat deaths of caste ants (queen excepted)
   **down** them instead of killing — `Fallen { bleed_t, heal_t,
   carried_by }`, 60s bleed-out (dying for real at the end). Medics
   auto-rescue: nearest unattended casualty → carry home (the casualty
   rides the medic like an egg) → heal for **2 stored water** (the
   long-awaited water sink) over 10s → revive at full hp. Medics never
   farm — unoccupied medics stand by near the queen (`Job::Hold` polls for
   casualties); they join squads as travelers/fighters but farm-conversion
   skips them. Spiders ignore downed ants (not prey). Downed ants render
   with red `DOWN` labels and red HP bars.
6. **Retaliation**: when a squad leader takes a spider hit, every follower
   gets the attacker as `attack_after` — the same shared-target assist
   semantics as the leader's own attack orders (wave S).
7. **Wire layout v4** (stride 13): kinds +honey/medic, carry +fallen, food
   +honeydew, egg records carry a caste code (was a soldier flag), ant
   p7 = downed bleed fraction. `brood_spec()` exports the menu's data from
   the rules (the client renders costs, never hardcodes them).

## Verification

- 71 core tests (9 new: nettle count, brood order pays physically +
  refusal reasons + cooldown, soldier conversion consumes the worker,
  honey secretion, bleed-out without a medic, medic rescue end-to-end,
  softlock recovery, retaliation, rules validation). clippy
  `-D warnings` clean (also caught the toolchain's 10 lint drift since the
  last wave), cargo fmt.
- e2e smoke: new F4 section — cycle to the queen, brood menu hidden before
  X (the squadmenu CSS lesson regression-asserted), opens with core-sourced
  costs, empty-pantry refusal flashed, paid order lays an egg with exact
  physical payment. 3× green; cross-platform determinism byte-match
  (native dump vs WASM, ~6.5k chars, rules digest included).

## Open for playtest

- Brood pacing: is 3s lay cooldown + 45s incubation + 2p+1w per worker the
  right recovery curve after a spider wipe? (Knobs: `lay_cooldown`,
  `egg_time`, brood costs.)
- Medic latency: they wait in the nest — should they patrol with foragers
  instead? Bleed time 60s vs map size is the tension.
- Honey value: 8 honeydew per Honey ant vs 1/4min secretion — the
  compounding loop's slope (nettle funds ~2 eggs each).
- Near-nest protein wall (pre-existing): if no protein source is ever
  sighted, brood orders can't be paid — the request display is the
  pressure valve; revisit source placement.
- Soldiers can't dig (defender design); Honey/Medic likewise non-diggers —
  deliberate role separation, surface if it feels wrong in play.
