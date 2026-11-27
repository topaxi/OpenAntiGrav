# ADR-0097: A screen filter that moves pixels runs at full strength

## Status

**Accepted (2026-11-27)**, the maintainer's decision. Supersedes one item of
[ADR-0053](0053-screen-filters-are-loadable-wgsl-after-the-composite.md): that
`screen_filter_strength` "means the same for every preset". The rest of
ADR-0053 stands.

## Context

The prelude mixes a preset's output with the untouched frame at the same output
pixel. A preset that draws the picture somewhere else, as the barrel in
`crt-interlaced` and `crt-shadow-mask` does, then shows two pictures on top of
each other at any strength below 100: the curved one and the flat one. Every HUD
element is doubled.

## Decision

- A preset that moves pixels says so in its header, `fixed_strength = true`.
- The pass draws such a preset at 100, whatever is stored.
- SCREEN FILTER STRENGTH is not offered while one is selected.
- The stored value is kept and applies again to a filter that has the row.

`crt-interlaced` and `crt-shadow-mask` declare it. The contract is in
[screen-filters.md, "A preset that moves pixels"](../../rendering/screen-filters.md#a-preset-that-moves-pixels).

## Alternatives considered

**A list of built-in ids in the engine.** Rejected: filters are loadable files,
so a player's own barrel has to be able to opt out the same way.

**The row greyed, showing the stored value.** Rejected: it would show a number
the pass does not use, and there is nothing to choose.

**Leaving the mix as it was.** Rejected: the doubled picture is not a weaker
form of the filter.

## Consequences

- The strength row no longer means the same thing for every preset: for some it
  does not exist.
- The declaration is not read off the shader. A barrel without the line still
  doubles, including a copy of a built-in made before the key existed; a preset
  that declares it and moves nothing loses the row for no reason.
- A player cannot have a weaker barrel through this row. The preset's own
  tunables are the only way, and no menu row edits them.
