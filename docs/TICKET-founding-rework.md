# Ticket — founding & early-game rework: the water quest + typed food blocks (Wave K)

**Shipped:** 2026-10-03 · **Version:** 0.1.06.61-dev · **Status:** shipped
(100 core tests, e2e 3× green, native-vs-WASM determinism byte-match,
check.sh ALL GREEN)

**User spec (verbatim intent):** "The Queen must collect WATER, carry it
back to the nest, place it on a food block, and find an egg block. When she
enters the egg block, she lays 4 eggs, which hatch into workers after 4
minutes. All subsequent eggs hatch after 2 minutes. Each food block can
store up to 15 units of a single food type. New eggs require PROTEIN +
SUGARS."

## 1. The water quest replaces the founding timer

The 60-second excavation window (`founding_time`) is **gone** — the field,
the `phase_t` timer, the `phase_time()` accessor, everything. Founding is
now a ritual with no deadline (the lone-queen grace still freezes her
hunger, so she can take as long as she needs):

1. The queen harvests water at any water source on the surface (existing
   mechanic — she has farmed since F1).
2. She carries it home and it lands on a **food block** (silver soil) —
   her auto-bank while walking over silver, or a manual drop; both count.
3. Once the tally reaches `founding_quest_water` (default 1), **standing
   on an egg block** (orange soil) lays the 4 founding eggs.

Design points:

- **Detection lives in one place**: `spawn_unit_food` counts underground
  water units landing on silver during `Phase::Founding` (`Colony`
  .`quest_water_tally`, a canonical term — determinism intact).
- **The water is not consumed**: the ritual is a demonstration, not a
  payment — the stored unit remains normal pantry food. Once the tally is
  reached it stays reached (one-way).
- **The eggs prefer orange cells** (`free_egg_tile` gains an orange-first
  pass), so the founding brood hatches where it was laid without being
  carried — egg transport stays available for X-menu eggs elsewhere.
- **No hand-outs**: the silver and orange blocks are still found by
  digging (~3.5%/block) or built with wet wood / dry wool. The quest gives
  the founding dig a *purpose* — reveal the pantry and the nursery.
- Water carried before the nest exists can't count — she can't dig while
  carrying, so the nest → water → egg-block order is enforced by physics.
- Validation keeps the quest solvable: `founding_quest_water ≥ 1` and the
  source list must contain at least one water source.

**Incubation**: founding eggs 240s (`founding_egg_hatch`, was 180);
every later egg 120s (all four brood `egg_time` rows and the global
`egg_time`, were 45).

## 2. Typed food blocks — 15 of ONE kind per 2×2 block (user-confirmed reading)

`food_cell_cap` (6, mixed, per tile) is replaced by **`food_block_cap: 15`**:
underground, one 2×2 block holds at most 15 units total and **exactly one
kind** — `block_room(layer, tile, kind)` is the one room check (0 when the
block is full or foreign). Surface cells keep per-tile mixed behavior with
the same number.

Enforced at every placement site: player `Drop` (refused), queen auto-bank,
worker/feeder banking, `Deliver` arrival (re-picks a pantry cell),
`pantry_tile(kind)` (kind-aware cell choice), honeydew secretion (a foreign
or full block spills the unit — event says so), falling carriers spill.
`spawn_unit_food` returns success and `store_food` now places **before**
crediting the ledger. Dev pantry piles remain a deliberate bypass (test
setup tool). No wire format change — the block rule is derived logic; the
renderer reads the cap from the rules (cached) instead of the old `/6`.

## 3. New eggs cost protein + sugars

Worker **2p+1c** (was 2p+1w), soldier **6p+3c** (was 6p+3w), medic 4p+3c
(unchanged), honey 1p+8h (unchanged). Water leaves the brood economy
entirely — it is the founding ritual's currency and stays in the queen's
craving cycle + medic healing. New `DevSpawn::PantryCarbs` /
`spawn pantry-carbs` for testing/tuning parity.

## Client

- Phase row during founding shows quest progress
  (`founding · water 1/1`); `phaseTime` plumbing removed; the help bar
  teaches the quest steps (incl. the wood/wool alternates).
- Wrong-kind / full-block food drops flash the exact reason
  ("This food block stores carbs — each block holds one food type").
- `quest_water_tally()` wasm getter (the `feeder_id` pattern — no wire bump).

## Test & e2e notes (gotchas recorded)

- `complete_founding_quest` / `hatch_founding_colony` helpers drive the
  real flow (source by the hole → harvest → bank on painted silver → stand
  on painted orange); they replace every old "tick 1220 for the timer"
  setup. The quest helper kills worldgen spiders first (determinism).
- The e2e orange-stand click must stay **outside the 2-tile entrance
  radius** — inside it, a right-click means "cross the entrance", and the
  queen surfaces instead of walking to the egg block.
- The e2e egg-round-trip dev egg must spawn on a **scanned-empty tile**
  beside the queen — the queen often stands at the chamber edge, and a
  fixed offset lands the egg inside the dirt wall (the right-click then
  digs instead of picking).
- The F4 "empty pantry" refusal test uses the honey caste (8 honeydew is
  never present) — by F4 time the run's workers have usually farmed real
  protein, so a worker order would succeed.
- The dirt-capacity test is capacity-driven (`rules.queen.dirt_capacity`,
  user-tuned to 3) and dumps the rest of the load on the surface —
  refilling dug blocks can seal the corridor back.
- Known edge (pre-existing, now more visible): a honey ant standing on a
  foreign-typed block spills its secretion; players give honeydew its own
  block. Idle non-food carriers no longer route to Deliver (they stood
  around at the pantry pointlessly before).

## What's next

Wave C neutrals — their tuning rows go straight into `data/*.jsonc`, and
their world entries into `docs/WORLD-DESIGN.md`.
