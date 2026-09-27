# Ticket — Worker priorities: feeder, physical queen feeding, drift, follow-resume

Wave **P** (worker **p**riorities), shipped before F4 per user decision
(2026-09-27, answers 1A/2A/3A/4A). One wave, playtest-gated.

## User spec (verbatim intent, 2026-09-27)

1. There is always one worker in the nest to **feed the queen**; it auto-moves
   the queen's *requested* food (displayed above her as icon + text) from the
   pantry (silver blocks) to her.
2. Extra workers go outside and auto-farm anything in the visible area; with
   nothing visible they idle. Far sources must be found by the player visiting
   with ants.
3. Ants recruited by "follow me" while busy **resume their unfinished
   activity** on release.
4. Taskless workers walk around where they were left for a little while, then
   return to the nest.

Settled decisions: queen craving is a **rotating cycle**; feeding is **fully
physical** (no automatic store drain in founding); **auto-scouting removed**
(long-range Job::Scout gone); this wave ships **before** F4.

## Design

### Queen hunger + craving (founding mode; legacy economy untouched)

- `Colony.hunger_t: f64` — seconds since last fed. She is **hungry** at
  `hunger_t >= EAT_PERIOD` (25s) and **dies** at `hunger_t >= EAT_PERIOD +
  STARVE_TIME` (115s). Grace rule (workers == 0) freezes `hunger_t` —
  unchanged solo-queen protection.
- `Colony.craving_i: usize` indexes `QUEEN_CRAVING_CYCLE =
  [Carbs, Protein, Carbs, Water]` (balance.rs). Feeding advances the index.
  Explicit index because the cycle repeats Carbs — position-by-search would
  alias.
- The abstract auto carb drain is **removed in founding mode**. The store
  counts (`carbs/protein/water`) become the **pantry ledger**: +1 at bank
  (existing), −1 at feeder withdraw. A unit in mandibles is outside the
  ledger (withdrawn); death of a carrier loses nothing from the ledger.
- Founding reserves become physical: founding the nest spawns a stored
  (never-spoiling) pile of `START_FOOD` carb units in the starter chamber,
  ledger in sync. The feeder era starts with real food on the floor.
- Queen self-feeding: an idle hungry queen carrying the craved kind eats it
  from her mandibles (no ledger change — never banked). Solo queens therefore
  survive by farming (F1 parity), and a stockpiled queen can top herself up.

### Feeder role

- `Job::Feed`. Designation step in `worker_ai` (founding only): keep
  `Colony.feeder_id` while the chosen ant is alive, a worker, and its job is
  neither `Manual` nor `Follow`; otherwise reassign to the lowest-id such
  worker (stealing it from Fetch/Deliver mid-run is fine — a carried craved
  unit gets delivered straight to the queen) and log the event. No candidate →
  `None` (the queen's display is the warning).
- Feed branch (idle-tick state machine, walk-to-act pattern):
  - dirt in hands → haul out (same gate as every job);
  - carrying the craved kind → stand adjacent to the queen (route if far) and
    feed the tick she turns hungry: consume carry, `hunger_t = 0`, advance
    craving, event. Holding the meal ready beside her is the intended look;
  - carrying a mismatched kind (craving moved on) → re-bank it on the pantry
    (`store_food` semantics undo the withdraw), then fetch the new craving;
  - empty hands → withdraw one craved unit from the nearest pantry pile of
    that kind (nearest to the queen, tie → lowest id): pile −1, ledger −1,
    `Carry::Food(kind)`. No such pile → wait beside the queen.

### Idle drift + return home (Job::Scout removed)

- Taskless workers (Job::Idle with nothing to do) enter `Job::Loiter(hops)`:
  short random hops within `LOITER_RADIUS` (4 tiles) of where they idled,
  `LOITER_HOPS` (3) hops with ~1s pauses, then `Job::GoHome` (underground,
  near the queen) and a `HOME_REST_TICKS` (15s) rest before the next loiter
  cycle. Underground-idle workers loiter inside the nest; the exit hop past
  the entrance keeps a small visible presence around the nest mouth —
  near-nest sources get sighted naturally, far ones stay player-driven.
- Discovery-by-sight (`discover()`, SIGHT_RANGE) unchanged. The old 12–35
  tile outward scouting wanders are gone.

### Follow-resume

- `Job::Follow(leader, resume: Option<Box<Job>>)` — recruitment stores the
  previous job verbatim (Manual included: a released formerly-manual ant
  returns to Manual, not autonomy); Release and leader-death restore it.
  Re-recruiting another leader's follower nests correctly. Player commands
  still leave the squad by overwriting the job (resume dropped — intended).

### Wire + canonical

- Wire layout v3, stride 11 → 12: ant `p6 = request` (food code of the
  queen's craving; 0 = not the queen). `snapshot_spec()` gains a `request`
  group; client `assertWireSpec` pins it.
- Queen `hunger` (p4) semantics: 0 while satisfied, then
  `(hunger_t − EAT_PERIOD)/STARVE_TIME` 0→1 from hungry to death (urgent =
  hunger > 0). Other ants 0.
- Canonical: colony line gains `cr hu fd` (craving kind, hunger_t, feeder_id);
  new/changed Job variants flow through the existing `j{:?}` dump.

### Client

- Queen request badge above her: resource-colored dot + short text
  ("wants carbs"), red pulsing "needs carbs!" while urgent. Reuses
  `RES_COLORS` + the entity label machinery; only visible when a craving
  exists (founding mode from nest founding onward).
- HUD Colony panel gains a "Queen wants" row. debugState/console gain
  craving/hunger/feeder (tiny wasm getter for feeder id).

## Acceptance

- [ ] Core: craving cycle advances only on real feeding; grace holds while
      workers == 0; queen dies at 115s unfed once workers exist (pantry
      empty); founding reserves spawn as a physical pile.
- [ ] Core: feeder designated (lowest-id autonomous worker), reassigned on
      death/manual/follow; withdraw decrements pile + ledger; feed resets
      hunger + advances craving + event; mismatch banked; queen self-eats
      craved carry when hungry.
- [ ] Core: no Job::Scout anywhere; idle workers loiter → home → rest;
      near-entrance loitering sights near sources; far unknown sources are
      never fetched.
- [ ] Core: busy ant → follow → release resumes the prior job (farmer back
      to farming, feeder back to feeding, manual back to manual).
- [ ] 46→N core tests green incl. rewritten scout test; legacy (Sim::new)
      eat/lay behavior untouched green.
- [ ] Client: boot spec assertion passes on layout v3; badge + HUD row
      correct in both satisfied and urgent states; WASM rebuilt.
- [ ] e2e: fixture regenerated, native-vs-WASM canonical byte-match, suite
      green ×3; smoke asserts the queen request decodes.
- [ ] Version bumped in the same commit; README/TODO synced.

## Playtest questions (not blockers)

- Feeder recruited away mid-hunger: queen display should make the risk
  obvious — is one badge enough, or does the feeder need a ring/icon?
- Loiter cadence feel (3 hops / 15s rest) and whether home-resting workers
  should hover near the entrance instead of underground.
- Craving cycle order/difficulty: Carbs→Protein→Carbs→Water.
