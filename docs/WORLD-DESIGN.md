# World design: creatures, resources, map

User's design draft (2026-09-26), captured verbatim as the content bible.
Balance numbers are placeholders — they now live as data in `core/src/rules.rs`
(`GameRules`: one typed, validated ruleset per sim; the admin tuning panel
edits it live in a later wave). Decisions get promoted to README.md once
confirmed in playtesting.

## Creatures

### Player-controlled (F4 SHIPPED 2026-09-28, costs rebalanced 2026-10-03 to
protein + sugars — all paid from the physical pantry via the queen's X-menu;
refusals explain themselves)
- **Worker** — 2× protein, 1× carbs to spawn. Idle: feeds the queen, otherwise
  farms resources. Very weak.
- **Soldier** — 6× protein, 3× carbs to convert from worker — the order
  CONSUMES one living worker (it spins into the soldier brood). Muscles;
  attacks neutral/hostile creatures.
- **Honey** — 1× protein, 8× aphid honeydew. Produces 1× aphid honeydew / 4 min
  as a physical unit where it stands (banks on silver pantry soil, spoils as
  loose loot anywhere else — workers haul it home).
- **Medic** — 4× protein, 3× carbohydrates. Auto-rescues fallen ants: carries
  them to the nest and heals them for **2 stored water** (the water sink);
  medics never farm — they stand by in the nest between casualties.
- **Queen** — spawns eggs. Combat deaths of caste ants down them instead of
  killing: 60s bleed-out unless a medic arrives; spiders ignore downed ants.

### Neutral, with perks
- **Earthworm** — drops small treasure when killed.
- **Snail** — slows passing ants by 60%.
- **Rove beetle** — can be given food to invite it to the nest; releases
  beneficial chemicals (buff) while resident.

### Hostile
- Bosses (todo): *Mucha Garbata*, *bombardier beetle*.

## Resources

Colony resources:
- **Protein** — spawning ants.
- **Carbohydrates** — keeps the colony alive and fit; below threshold ants slow down.
- **Water** — heals injured ants.
- **Aphid honeydew** — required to spawn Honey ants. Settled 2026-09-27:
  until aphid farming exists (Wave D), honeydew comes from **nettle** — a
  map source like the others (15–22 units, 10s/unit, 4 scattered). The
  queen's feeding is physical since the worker-priorities wave: rotating
  cravings (carbs→protein→carbs→water), one feeder worker, requests shown
  above her and in the HUD.

Map resources are **finite** (harvestable once).

From creatures:
| Creature | Protein | Honeydew | Carbs | Water |
|---|---|---|---|---|
| Earthworm | 5–8 | 2–4 | 1–3 | |
| Snail | 2–4 | | | 1–2 |
| Rove beetle | 8–12 | 3–5 | 5–7 | |

On map (amount, pickup time):
| Resource | Amount | Pickup |
|---|---|---|
| Moss | 15–20 water | 10 s |
| Mushroom | 21–26 water | 20 s |
| Raspberry | 100–110 carbs | 6 s |
| Strawberry | 40–60 carbs | 4 s |
| Cockroach | 8–12 protein | 15 s |
| Caterpillar | 25–35 protein | 13 s |

**Aphid farming** — flower buds infested with aphids, farmed for honeydew:
1. On-site: 1 honeydew / 60 s.
2. Migrated to nest: 1 / 20 s.
3. Aphids can be captured and moved to the nest — requires an **Aphid Chamber**
   with 1 soldier stationed in it.

## Map
Size: TBD — comfortable for the above. (Current: 96×96.)

## Intended flows
- Workers scout for non-creature resources after hatching; they are weak and
  must avoid hostile creatures. Finding resources should be reasonable, not easy.
- Workers may attack neutral creatures but deal no damage — the neutral flees
  in panic ~20 blocks in a random direction.

## Implementation status
- ✅ Flight right-click = fly to destination then land; grounded right-click =
  walk to location then found the nest there (was: instant land/found).
- ✅ **Resource economy (Wave A):** protein / carbs / water stores; green food
  removed from the founding map; the six finite source types scattered as
  single finds (counts/spacing/yields/harvest times in `GameRules.sources`,
  `core/src/rules.rs`); harvest takes per-unit time; scouts discover sources
  by sight (8 tiles) before foraging targets them; spiders drop protein;
  carbs below CARB_LOW slow the colony. Water/honeydew sinks (healing, Honey
  ant) come with later waves — water accumulates for now.
- ✅ Honeydew→carbs intent noted: ~20 carbs per extracted honeydew unit, once
  aphid farming lands (Wave D).
- ✅ **Admin tuning (2026-09-28):** every number in this document that ships
  as a `GameRules` row is live-tunable in-game — F4 / `?admin=1` drawer,
  atomic whole-object commits (validation errors listed), live vs
  new-game scopes, same-seed restart for worldgen tweaks, replayable
  commits (`dev-rules`, digest folded into the canon). Balance changes no
  longer require a rebuild.
- ✅ **Activity indicators (2026-09-28, always-on per the user's live-play
  call):** every player ant shows a pixel intent icon (per-resource harvest tint, dig, haul ×2,
  fight, medic, feeder crown, follow, honey production, off-duty) + a
  red "!" with an honest reason (empty pantry / needs water / pantry
  full / queen starving). Settled user rule: icons are the FAR-zoom
  layer — the rigs carry the information up close.
- ✅ **Data files (2026-10-03):** every `GameRules` number now lives in
  `data/*.jsonc` at the repo root — JSONC with comments, embedded into the
  core at build time (one source of truth for browser, native tests, and
  replays); balance edits are file edits + rebuild, live experiments stay
  in the F4 drawer.
- ✅ **Founding water quest + typed food blocks (2026-10-03):** founding
  opens with a ritual instead of a timer — the queen must store water on
  a food block (silver), then stand on an egg block (orange) to lay the
  founding brood (4 worker eggs, 4-minute incubation; all later eggs
  2-minute). Food blocks are TYPED: one 2×2 underground block holds up to
  15 units of a single food type (`food_block_cap`; surface cells stay
  mixed). New eggs cost protein + sugars (worker 2p+1c, soldier 6p+3c,
  medic 4p+3c, honey 1p+8h) — water left the brood economy and is the
  founding ritual's currency. Full design + verification record in
  `docs/TICKET-founding-rework.md`.
- Open: worker avoid-hostile behavior, panic mechanic, aphid chamber, bosses.
