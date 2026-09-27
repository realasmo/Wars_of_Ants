# Ant proportions intake — Camponotus set (re-skin of the procedural-ants rig)

Vision-model extraction (GLM-Vision, 2026-09-27) from the reference photos in
`Ants-Photos/camponotus-test/`. Draft→locked values for `client/src/art/ants.ts`
(the Camponotus set replaces the Carebara values as the live config; the
Carebara consensus remains in `docs/ant-proportions-intake.md` and git history).

Ratios normalized: thorax length = 1.0 (length rows), thorax width = 1.0
(width rows). "Body length" = head + thorax + gaster.

## CRITICAL ID flag

`soldier1.png` / `soldier2.png` are **Myrmecia (bull ants), not Camponotus**
(enormous bulging eyes, long linear serrated mandibles; Camponotus majors have
small eyes and short broad triangular mandibles). Structure values below are
Camponotus-corrected (per the user's written spec: large reddish-orange head,
short legs); the photos remain palette/gloss references only. If real major
photos arrive, re-extract and re-lock.

## Consensus (the numbers in the config)

| field | worker (minor) | soldier (major) | queen (gyne) |
|---|---|---|---|
| head:thorax:gaster length | 0.4 : 1.0 : 1.1 | 0.9 : 1.0 : 0.9 | 0.8 : 1.0 : 1.0 |
| head:thorax:gaster width | 1.2 : 1.0 : 1.7 | 1.5 : 1.0 : 1.4 | 1.4 : 1.0 : 1.4 |
| waist | ONE node, 0.5 | ONE node, 0.5 | ONE node, 0.4 |
| leg length / body length | ~1.0 (tempered 0.85 in-game) | 0.55 | 0.55 |
| splay front/mid/rear (°) | 40 / 90 / 30 | 50 / 75 / 40 | 45 / 80 / 35 |
| mandible / head width | 0.2 | 0.35 broad triangular | 0.25 |
| antennae (scape→body; rig uses full antenna) | scape 0.3, full ~0.6 → 0.5 | 0.45 | 0.4 |
| bodyLenTiles | 1.0 | 1.7 | 2.4 |

## Bicolor pattern per caste (team 0 = natural)

- **Queen:** orange-red head; LARGE BLACK muscular mesosoma with wing scars;
  black banded gaster (pale gray pubescent bands), silvery hair fringe.
- **Soldier:** red-orange head (broad RECTANGULAR, straight sides) + red
  mesosoma + red legs; GLOSSY BLACK gaster with strong warm specular.
- **Worker:** dark head; orange mesosoma + legs + antennae; dark glossy
  gaster. Long-scaped antennae, long legs (hind legs trail past the gaster
  tip) — the caste's strongest identifier.

## Structural notes for the builder

- **One-node waist on every caste** — never draw the two-node Carebara waist.
- Soldier head is rectangular (straight parallel sides), not cordate; queen
  head is a wide rounded-rect; worker head a small oval with prominent eyes.
- Queen wing scars: small lateral notches ~45% along the mesosoma (placement
  UNRELIABLE from photos — drawn as short dark strokes on the thorax sides).
- Queen legs/limbs near-black in photos (unreadable at play zoom) — in-game
  limbs lightened to dark brown for leg visibility (readability deviation).
- Real sizes 6–24 mm; in-game scale worker 1.0 / soldier 1.7 / queen 2.4
  tiles (the 2.5–4× size gap is the species' gameplay readability feature).

## Mandible state spec (user-provided, dev-ready)

1. Idle: closed, relaxed, forward (≈0° baseline).
2. Alert: flared wide (+45–90°) — combat wind-up.
3. Bite: high-speed snap open→locked; damage lands at full close.
4. Clamp: locked around a target + body shake (needs CC gameplay + snapshot
   field — deferred until a sim wave adds pinning).
5. Carrying: partially open, static, adjusted to the carried item's width
   (dirt < food unit < egg); combat animation suppressed while carrying.

Client-only implementation notes: alert/snap run on a periodic cycle while
`activity === 'fighting'` (the snapshot has no aggro radius or attack-timer
field, so the flare cannot trigger on enemy proximity — that plus
damage-synced snaps and clamp needs one small core addition).
