# Ant proportions intake — Carebara diversa (Step 0 of the procedural-ants ticket)

Vision-model extraction (GLM-Vision, 2026-09-27) from the reference photos in
`docs/Ant-Specie_Carebara_Diversai/`. Draft values for `client/src/art/ants.ts`;
the implementing session locks the config from the consensus rows below.

Ratios are normalized: thorax length = 1.0 (length rows), thorax width = 1.0
(width rows). "Body length" = head + thorax + gaster.

## Consensus (the numbers to put in the art config)

| field | worker (minor) | major | soldier (supermajor) |
|---|---|---|---|
| head:thorax:gaster length | 0.65 : 1.0 : 0.55 | 0.9 : 1.0 : 0.7 | 1.6 : 1.0 : 1.6 |
| head:thorax:gaster width | 1.4 : 1.0 : 1.2 | 2.2 : 1.0 : 1.5 | 3.0 : 1.0 : 3.0 |
| petiole / thorax width | 0.3 (postpetiole ~0.5) | 0.3 (postpetiole ~0.55) | 0.55 |
| leg length / body length | 0.5 | 0.6 | 0.5 |
| leg splay front/mid/rear (° from body axis) | 45 / 85 / 45 | 50 / 70 / 45 | 70 / 55 / 35 |
| mandible / head width (closed) | 0.4 (0.5 open) | 0.4 | 0.35 |
| antennae / body length | 0.34 | 0.6 | 0.35 |
| palette base | #8a6330 | #a7753a | #4e1d13 |
| palette dark | #3a2312 | #46311c | #20100a |
| palette highlight | #ecdcaa | #e0b060 | #d8c6b8 |
| palette limbs | #c2954e | #bc8c50 | #a85624 |
| palette outline | #21140a | #281a0e | #160a06 |

**Queen: no photos — extrapolate at config time.** Starting point: soldier's
gaster emphasis pushed further (gaster width ~3.5–4× thorax, gaster length
~2.0), worker-sized head, mandibles ~0.3, plus wing stubs (ticket spec).

## Structural notes that should shape the builder

- **The caste read is carried by head width**: 1.4× (worker) → 2.2× (major) →
  3.0× (soldier) thorax width. Major/soldier heads are cordate (heart-shaped,
  wider than long) with a median frontal furrow; soldier adds a deep posterior
  emargination. Silhouette alone must separate the castes (ticket criterion).
- **Two-node waist** on all castes: thin petiole + wider postpetiole. Don't
  merge them into one blob.
- **Bicolor limbs**: legs are amber/orange and distinctly lighter than the
  body on every caste; soldier body is dark maroon with near-black mandible
  tips, lighter orange-red limbs.
- **Small propodeal spines** on worker and major (subtle; optional at play
  zoom).
- Soldier gaster droops below the body axis when relaxed, head held slightly
  high — good posture reference for the lumbering gait.
- Real body lengths: worker ~2.3 mm, soldier ~14–15 mm (≈6×). In-game scale is
  a tuning decision, not a copy — a literal 6× makes workers invisible at play
  zoom. Suggest worker 1.0 / major ~1.6 / soldier ~2.2 / queen ~2.6 linear
  scale and revisit after first screenshots.
- Erect curved gaster hairs (all castes), dense on worker; render as 2–3 tiny
  strokes on the gaster texture at most.

## Config-author caveats (from the intake agents)

- Leg splay angles come from **mounted museum specimens** except
  `soldier/soldier.jpg` (the one natural-stance, live photo — it drives the
  soldier row; pinned specimens fold their legs and underestimate length).
- Gaster length varies with engorgement (major ranged 0.6–0.8); consensus
  takes midpoints.
- Museum flash inflates palette saturation; treat hexes as hue references and
  re-tune contrast for the game's dark underground background. The dark-gaster
  vs light-mesosoma contrast is reliable across all castes.
- Antennal length estimates are the weakest numbers (folded/cropped in most
  photos) — expect to tune by eye.

## Raw per-photo extraction

### worker/ (all same caste, ~2.3 mm; AntWeb-style macro shots with scale bars)

**worker-1.jpg** (dorsal, whole body): H:T:G len 0.6:1.0:0.5, wid 1.3:1.0:1.2;
petiole 0.3 (postpetiole ~0.5); legs 0.5, splay 50/90/45; mandible 0.3;
antennae 0.35 (hidden, corroborated by 2–3). Palette base #8f5c28, dark
#2e1a0c, highlight #e6d39a, limbs #b98a45, outline #1c1006. Dark glossy head
vs caramel mesosoma; two-node waist; long curved gaster hairs; amber
bicolored legs.

**worker-2.jpg** (frontal head-only): head L:W = 1.0:1.0; mandibles 0.5 fully
extended (apical + basal tooth visible); antennae 0.33 of body; antenna
11-segmented ending in a large dense 2-segment pale club (visually dominant);
small compound eyes mid-head; broad oval head widest across eyes. Palette base
#4a2d14, dark #26150a, highlight #f0e0ac, limbs #c08a3e, outline #180d04.

**worker-3.jpg** (side profile): H:T:G len 0.67:1.0:0.58; heights
0.55:0.45:0.63 mm (gaster tallest tagma); legs 0.55; sagittal leg angles
35/55/65 below axis; mandibles 0.35 folded (~0.5 extended); antennae 0.34.
Domed pronotum/mesonotum, small up-back propodeal spine, gaster ~1.5× mesosoma
height, bristle-fringed hind tibia/tarsus, large eye for a minor. Palette base
#a8823f, dark #5f401d, highlight #ecdfb2, limbs #c6a058, outline #2a1908.

### major/ (all same caste, sharp)

**major-1.jpg** (dorsal): H:T:G len 0.9:1.0:0.6, wid 2.2:1.0:1.8 (gaster
contracted, inflating width); petiole 0.3 (postpetiole 0.55); legs 0.6, splay
50/70/45; mandibles 0.3 (occluded, low conf.); antennae 0.6 (folded est.).
Head wider than long, coarse rugose sculpture; smooth dark gaster contrasting
light amber head/mesosoma. Palette base #a5753c, dark #4b3620, highlight
#d9a968, limbs #b98a4e, outline #2c1d10.

**major-2.jpg** (frontal head-only): head W:L 1.1:1 (cordate); mandibles 0.4,
short/massive/toothed; 11-segmented antennae, elongated 2-segment club; head
top dark reddish-brown, mandible bases bright yellow-orange; eyes far forward.
Palette base #a76f2e, dark #3e2410, highlight #e8b24a, limbs #b5762f, outline
#1f1208.

**major-3.jpg** (lateral): H:T:G len 0.8:1.0:0.8 (gaster extended); head depth
~1.2× head length; legs 0.65, splay 45/65/45; mandibles 0.4; antennae 0.65.
Distinct small propodeal spines; gaster hangs downward, dark with lighter
apical band; long golden hairs; semi-translucent amber legs. Palette base
#a97b3f, dark #46311c, highlight #e4b75c, limbs #c89a55, outline #241608.

### soldier/ (supermajor)

**soldier.jpg** (dorsal, LIVE natural stance — best leg reference): H:T:G len
1.2:1.0:1.8, wid 2.2:1.0:2.6; petiole 0.6; legs 0.5, splay 70/55/35 (only
natural splay in the whole set); mandibles 0.6 OPEN (~0.35 closed); antennae
0.3. Glossy dark-maroon; gaster engorged/bulbous (distended — inflates its
ratios); huge cordate head with deep posterior emargination; short antennae
with small club; near-black mandible tips. Palette base #5a1f14, dark
#24100c, highlight #e0d0c4, limbs #a8481f, outline #1a0b06.

**soldier-1.jpg** (frontal head-only, 1 mm bar): mandibles 0.3 closed (head
~4.6 mm, mandibles ~1.4 mm); scape ~2.3 mm pale orange-tan vs dark head;
cordate head, deep posterior median notch, dense fine longitudinal striation,
tiny eye far back, 3 ocelli at vertex; glossy black broad-toothed mandibles.
Excluded from body ratios. Palette base #4a1a12, dark #1c0c08, highlight
#d0bcb0, limbs #b07438, outline #120804.

**soldier-2.jpg** (dorsal, pinned, 2 mm bar): H:T:G len 1.8:1.0:1.5, wid
3.9:1.0:3.3; petiole 0.6; legs 0.3 UNRELIABLE (folded); mandibles 0.3;
antennae 0.4. Dusty specimen, dull colors; head ~4× mesosoma width. Palette
base #3a2018, dark #201008, highlight #9a7a60, limbs #5a3422, outline
#140a06.

**soldier-3.jpg** (lateral, pinned): H:T:G len 1.8:1.0:1.6; depths
2.1:1.0:2.1; petiole 0.4 (side view, dorsal more trustworthy); legs 0.3
UNRELIABLE; mandibles 0.35; antennae 0.3. Gaster droops below body axis when
relaxed, head slightly elevated; head nearly as deep as gaster is tall;
bristles on legs/head margins; gaster segment banding. Palette base #3c1812,
dark #241008, highlight #c09878, limbs #6a3220, outline #120804.
