# Ticket — Squad conversion: the group mode persists at your action

Wave **S** (squad conversion), 2026-09-27 — user refinement of the F3
group mode, shipped after the worker-priorities wave (P).

## User spec (verbatim intent, refined same day)

Once joined, they follow me. If I start farming, they start farming as
well and **won't stop if I stop** — to get them back, issue another
"all visible ants join" request. For attacking — **keep it simple**: if
I attack a given enemy, all ants in my group attack the same enemy; when
it is killed, they stay near me, ready to follow. (The first draft had
them rampage the remaining hostiles; the user simplified it.)

## Design

- A squad leader who starts an activity **converts** every follower to it
  (`squad_convert`, end of `worker_ai`):
  - **Farming** — leader enters `Harvesting` → followers (workers AND
    soldiers) become `Fetch` on the leader's source. They keep farming
    after the leader stops: ordinary foraging life continues (next source
    by craving priority, pantry runs) until re-recruited.
  - **Attacking (assist-attack)** — the leader's attack order is shared:
    every follower gets the same `attack_after` target (workers included —
    supersedes the earlier "workers never fight" idea) while KEEPING the
    Follow job. When the enemy dies the intent clears and they are already
    squad members again: they walk back and stay near the leader, ready to
    follow. Clicking a new enemy retargets them.
- Farm conversion replaces the follower's saved resume — converted ants
  are farmers now; a later recruit-release cycle starts fresh.
- Re-recruit is the existing X-menu 1 (all in sight): they must be near
  you again, which is the "come back" gesture.
- The queen's **feeder role outranks conversion**: the designation still
  reclaims the lowest-id free worker, so with a very small colony one
  convert may be pulled back to feeding (intended — "always one feeder").
- Combat no longer wipes jobs on target death: squad followers keep
  following (that is what returns them to you), while player-commanded
  (Manual) ants are released back to autonomy — an attack order must not
  leave war parties as statues forever (the legacy swarm test depends on
  this release too).

## Acceptance

- [x] Leader harvests → workers + soldier convert to `Fetch(leader source)`,
      persist after the leader walks away, re-recruitable.
- [x] Leader attacks → every follower gets the same target (still squad
      members), the swarm damages it, and after the kill they follow the
      leader again — no rampage; retargeting works.
- [x] Manual ants are released to autonomy when their fight ends.
- [x] 62 core tests green (rampage test rewritten as assist-attack), smoke
      green + determinism byte-match, tsc clean.
- [x] Version 0.1.02.54-dev in the same commit.

## Not in this ticket (still F4)

- Retaliation: "if I'm being attacked, soldiers attack the attacker" —
  defensive trigger, comes with the F4 squad work.
- Honey/Medic brood effects on squads.
