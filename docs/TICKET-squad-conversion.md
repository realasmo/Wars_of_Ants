# Ticket — Squad conversion: the group mode persists at your action

Wave **S** (squad conversion), 2026-09-27 — user refinement of the F3
group mode, shipped after the worker-priorities wave (P).

## User spec (verbatim intent)

Once joined, they follow me. If I start farming, they start farming as
well and **won't stop if I stop** — to get them back, issue another
"all visible ants join" request. The same for attacking: if I attack,
they all attack the one I did, and then they **keep attacking the
remaining hostile creatures**.

## Design

- A squad leader who starts an activity **converts** every follower to it
  (`squad_convert`, end of `worker_ai`):
  - **Farming** — leader enters `Harvesting` → followers (workers AND
    soldiers) become `Fetch` on the leader's source. They keep farming
    after the leader stops: ordinary foraging life continues (next source
    by craving priority, pantry runs) until re-recruited.
  - **Attacking** — leader gains an attack order (click or chase) → the
    whole squad becomes `Job::Hunt(target)` — workers included (supersedes
    the earlier "workers never fight" idea). `Hunt` engages the target and,
    when it dies, acquires the nearest living predator anywhere on the map
    (rampage); with no hostiles left they stand down to normal life.
- Conversion replaces the follower's saved resume — converted ants are
  farmers/hunters now; a later recruit-release cycle starts fresh.
- Re-recruit is the existing X-menu 1 (all in sight): they must be near
  you again, which is the "come back" gesture.
- The queen's **feeder role outranks conversion**: the designation still
  reclaims the lowest-id free worker, so with a very small colony one
  convert may be pulled back to feeding (intended — "always one feeder").
- Combat no longer wipes a hunter's job on target death (it used to force
  `Idle`), which is what makes the rampage continue.

## Acceptance

- [x] Leader harvests → workers + soldier convert to `Fetch(leader source)`,
      persist after the leader walks away, re-recruitable.
- [x] Leader attacks → squad converts to `Hunt`, damages the target,
      reacquires the next hostile after a kill, stands down when none
      remain (worldgen spiders count).
- [x] 62 core tests green (2 new), smoke ×1 green + determinism byte-match
      (fixture regenerates per run), tsc clean.
- [x] Version 0.1.02.53-dev in the same commit.

## Not in this ticket (still F4)

- Retaliation: "if I'm being attacked, soldiers attack the attacker" —
  defensive trigger, comes with the F4 squad work.
- Honey/Medic brood effects on squads.
