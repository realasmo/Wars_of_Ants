# World design: creatures, resources, map

User's design draft (2026-09-26), captured verbatim as the content bible.
Balance numbers are placeholders — tune in `core/src/balance.rs` once playable.
Decisions get promoted to README.md once confirmed in playtesting.

## Creatures

### Player-controlled
- **Worker** — 2× protein, 1× water to spawn. Idle: feeds the queen, otherwise
  farms resources. Very weak.
- **Soldier** — 6× protein, 3× water to convert from worker. Muscles; attacks
  neutral/hostile creatures.
- **Honey** — 1× protein, 8× aphid honeydew. Produces 1× aphid honeydew / 4 min.
- **Medic** — 4× protein, 3× carbohydrates. Transports fallen ants back to the
  nest to heal.
- **Queen** — spawns eggs.

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
- **Aphid honeydew** — required to spawn Honey ants.

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
- Open mapping decisions (see README "Undecided"): how protein/carbs/water map
  onto (or replace) the current green/super food; worker scouting AI; panic
  mechanic; aphid chamber building; bosses.
