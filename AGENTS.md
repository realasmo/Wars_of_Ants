# Project instructions — Wars of Ants

## Quality above anything else (user directive, 2026-09-27)

When planning any code feature, optimization, or bugfix: **ALWAYS select the
best option, not the cheap one.** Cheap options may be less complicated and
faster to ship, but the quality of the work in this project matters ABOVE
ANYTHING ELSE.

In practice this means:

- Prefer the right long-term design over the minimal patch, even when the
  good option is a bigger ripple (the typed snapshot layer over magic-number
  constants was exactly this call).
- When presenting options, lead with the recommended best one; cheap
  shortcuts only when the user explicitly picks them.
- No risky shortcuts that survive on luck (unchecked indexing, silent
  fallbacks, load-bearing numeric codes scattered across files).
