# Task: Procedural ants — built from parts, animated like living creatures

Replace the current placeholder geometry ants with procedurally built and
procedurally animated ants that read as *alive* at a glance: a purposeful
walk with planted feet, idle micro-motion, and clearly readable carrying.
This is client-side rendering only — zero changes to the sim, snapshot
format, or gameplay.

## Step 0 — proportions intake (from photos)
The user provides ant photos (worker, soldier, queen). A vision model
(GLM-Vision subagent) extracts measurements using the intake prompt in the
Appendix. Take the returned numbers as draft values and put them into the
art config (below). If the extracted notes are not in this conversation
yet, ask the user for them before locking the config.

## Art config
Create `client/src/art/ants.ts` holding ALL visual constants: per-caste
proportions, per-team palettes, leg geometry, gait parameters. Render logic
reads the config and nothing else — changing the look means editing the
config, never the render code.

## Build: ants from parts
One builder, `buildAnt(caste, team)`, returns a Pixi container:
- Segments front→back: head (2 mandibles + 2 antennae), thorax, gaster,
  connected with slight overlap at a narrow waist (petiole).
- 6 legs, two segments each (femur + tibia), 3 per side, attached to the
  thorax.
- Castes driven by config: worker (slim), soldier (oversized head, heavy
  mandibles), queen (huge gaster, small wing stubs, slower gait).
- Team palette from config; dark outline on every part; one subtle
  highlight per segment.
- Bake every part type to a RenderTexture once (per caste × team). Per-ant
  rendering is sprites + transforms only — no per-frame Graphics rebuilds.

## Animation system — the heart of this task
1. **Planted feet, no sliding.** Each foot has a rest position in body
   space. While walking, a planted foot stays fixed in WORLD space; when
   its distance from the ideal rest point exceeds the stride threshold AND
   its tripod group may step, it swings in an arc to a predicted point
   ahead (tiny fake lift: slight scale/shadow dip). Legs step in
   alternating tripods (L1,R2,L3 vs R1,L2,R3) — a group may step only
   while the other group is fully planted. A planted foot must NEVER slide.
   This is the single most important detail. A sinusoid phase cycle is an
   acceptable temporary fallback only if planted feet misbehave — flag it
   clearly if you ship the fallback.
2. **Distance-driven phase, not time-driven.** Gait advances with distance
   traveled. Standing ants are motionless except idle motion (point 5).
   Speed naturally changes stride and step rate.
3. **Two-segment legs with IK.** Solve the knee with 2-bone IK; the knee
   bends outward/backward (away from the head), like real ant legs.
4. **Body motion.** Follow-through: head and gaster lag a few degrees
   behind the thorax when turning (thorax leads, head/gaster ease after
   it). Subtle body sway synced to steps. Body rotation smoothly
   interpolates toward the movement direction.
5. **Idle micro-motion.** Standing ants occasionally wave their antennae
   (each antenna independently, ease in/out), rarely turn the head
   slightly, and "breathe" with the gaster (very subtle scale pulse;
   pronounced on the queen). Randomized per ant, seeded by entity id with
   a fixed client-local RNG — never the sim RNG.
6. **Carrying.** The carried item (dirt block / egg / food unit, from the
   client's existing carrying state) is drawn between the mandible tips;
   mandibles rest slightly open while carrying. Pickup plays a quick
   open→snap-close (~100 ms); drop plays a brief release. The item
   inherits the head transform with a small positional lag so it sways on
   turns.
7. **Caste gait personality (config values).** Worker: brisk, small
   steps. Soldier: heavier, slower stride, wider splay. Queen: slow,
   lumbering, pronounced gaster sway.

## Constraints
- Rendering only: nothing under `core/` changes; no new snapshot fields;
  consume the interpolated positions/rotation/carrying state the client
  already renders from.
- Determinism guardrail: all cosmetic randomness is client-side, seeded
  per entity id; never touch the sim RNG.
- Performance: fixed per-ant part count; target 60 fps with 100 ants on an
  integrated GPU. Verify with the `?perf=1` overlay and report
  before/after worst-frame ms. If needed, skip limb updates for
  off-screen ants.
- Zoom: the animation must read at typical play zoom (ant ≈ 25–40 px on
  screen). Tune limb thickness and contrast so legs are visible but not
  noisy.

## Acceptance criteria (verify all before finishing)
- [ ] Walking ants show no foot sliding (step e2e ticks and check feet
      stick in world space between steps)
- [ ] Standing ants look alive within 5 s (antennae/twitch) but calm
- [ ] Tripod alternation is visible at play zoom
- [ ] Carrying dirt / egg / food reads clearly; snap animation on pickup
- [ ] Worker / soldier / queen distinguishable by silhouette alone
- [ ] Teams distinguishable by color only
- [ ] Queen unmistakable at a glance
- [ ] No per-frame Graphics rebuilds introduced; ?perf=1 worst-frame ms
      not worse than the pre-change baseline
- [ ] e2e suite still passes; replays unaffected

## QC loop
After implementing, capture close-up screenshots (walking, idle, carrying)
per caste via ?e2e=1 and run them through the GLM-Vision subagent for
critique against the proportions from Step 0. Iterate on art-config values
until the vision critique reports no structural complaints. Summarize what
changed in each iteration.

## Out of scope
Death/hatch animations, attack effects, pheromones — later waves.

## Appendix — photo intake prompt (run per photo via GLM-Vision)
"Analyze this ant photo (dorsal/top view). Extract quantitative
proportions as structured data, not prose: head:thorax:gaster length and
width ratios; petiole width relative to thorax; leg length and splay angle
relative to body length; mandible length relative to head width; antennae
length relative to body. Body color as a 5-shade hex palette (base, dark,
highlight, limbs, outline). Numbers are estimates — give one decimal."