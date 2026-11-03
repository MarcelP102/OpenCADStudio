# Icon design guidelines: 16 px and 32 px

How to draw Open CAD Studio tool icons so they look as sharp as possible on screen. These rules are for the pixel-native masters: one drawing at **16 × 16** (small ribbon tools, Quick Access Toolbar, menus, search results) and one at **32 × 32** (large ribbon tools).

They build on what we learned from the 19-unit line-icon set (Phase 3):
- A line icon is sharp only where its lines sit exactly on pixels.
- Lines of 1 px or less are drawn as hairlines: they get end caps but no corner joins.
- Diagonals and curves are always spread over two pixel rows, so they look lighter than straight lines.

The fewer of those the drawing needs, the crisper it looks.

---

## 1. Two drawings, not one scaled drawing

- **Draw the 16 px and the 32 px icon separately.** The 32 px icon is a redraw, not the 16 px icon scaled up.
- **Keep the same metaphor in both:** the same main shape, in the same place and the same orientation, so users recognise the tool at both sizes.
- **16 px: say one thing.** It shows one main shape plus at most one modifier badge (see §7). Anything that does not help someone recognise the tool at a glance is left out.
- **32 px: say the same thing with more detail.** It may add secondary information: grips, dimension ticks, inner lines, a second object, a pale fill, a short label. The main shape must still dominate.
- **Other sizes are generated from these two.**
  - 20 and 24 px (small icons at 125 % and 150 %) come from the 16 px master.
  - 40, 48, 56 and 64 px (large icons at 125–200 %) come from the 32 px master.
  - 28 px (small at 175 %) comes from the 32 px master.
  - The pipeline scales and re-snaps these. Lines that are on pixels in the master stay on pixels after scaling.

## 2. Canvas and live area

| | 16 px | 32 px |
| --- | --- | --- |
| Canvas (viewBox) | `0 0 16 16` | `0 0 32 32` |
| Live area (keep shapes inside) | 14 × 14, from 1 to 15 | 28 × 28, from 2 to 30 |
| May touch the canvas edge | badges, arrowheads, overshoot of round shapes | the same |

**Keyline shapes:** use these so icons in a row look the same size.

| Shape | 16 px | 32 px |
| --- | --- | --- |
| Square | 12 × 12 | 24 × 24 |
| Circle (diameter) | 13 | 26 |
| Landscape rectangle | 14 × 10 | 28 × 20 |
| Portrait rectangle (sheet, document) | 10 × 14 | 20 × 28 |

Optical centre: a symmetric shape is centred on the canvas; a heavy shape with a badge shifts up and left by at most 1 px (16) or 2 px (32).

## 3. Line weight and pixel positions

| | 16 px | 32 px |
| --- | --- | --- |
| Main lines | 1 px | 2 px |
| Secondary detail lines | — (none at 16 px) | 1 px |
| Line centre coordinates | **x.5** (pixel centre) | **whole numbers** for 2 px lines (pixel edge); x.5 for 1 px detail lines |

**Why:**
- A 1 px line centred on x.5 covers exactly one pixel column.
- A 2 px line centred on a whole number covers exactly two.
- Anything else is spread over an extra, half-lit column.

**Rules:**
- **One weight per size.** Don't mix 1 px and 2 px main lines in one 32 px icon; detail lines are the exception.
- **Shape lengths from line centre to line centre.** A 1 px square that is 12 px on the outside has its corners at 2.5 and 13.5.
- **Line ends:** `stroke-linecap="square"` for straight lines, so the end sits on a pixel edge. Use `round` only for curves and free ends that should look soft.
- **Corners:** sharp (90°) on straight shapes. No rounded corners at 16 px. At 32 px a corner radius of at least 3 px is allowed when the metaphor needs it (a rounded device, a tab).

## 4. Angles: prefer horizontal and vertical

Horizontal and vertical lines are perfectly sharp. Every other direction is softer, so:

- **Horizontal / vertical first.** Express the metaphor with straight lines wherever you can.
- **Allowed diagonals:** 45° (1:1), and 2:1 or 1:2 (≈ 26.6° / 63.4°). They form a regular pixel staircase:

```
45° (1:1)        2:1               1:2
#.......         ##......          #.......
.#......         ..##....          #.......
..#.....         ....##..          .#......
...#....         ......##          .#......
                                   ..#.....
                                   ..#.....
```

- **Other angles:** avoid them. An arbitrary angle makes an uneven staircase, which looks jagged with anti-aliasing off and blurred with it on.
- **Endpoints of a diagonal:** both on pixel centres (16 px) or both on whole coordinates (32 px), with the run and rise exactly in the allowed ratio (e.g. 6 across and 6 down, never 6 and 5).
- **Number of diagonals:** at most two at 16 px.
- **Diagonal through a corner:** let it start and end on a vertex shared with a straight line. That way the straight lines keep their crisp ends.

## 5. Curves and circles

- **Circles:** centre on a pixel centre (x.5, 16 px) or a pixel corner (whole number, 32 px), with a whole-number radius. The top, bottom, left and right of the circle then fall on full pixels; only the four diagonal stretches are soft.
- **Size:** the smallest circle at 16 px is 7 px across (radius 3). Smaller circles become blobs; draw a small point as a filled 2 × 2 or 3 × 3 square instead (see §6).
- **Arcs:** start and end on pixel centres, preferably at 0°, 90°, 180° or 270°, so the ends are crisp.
- **No small radii at 16 px:** a quarter circle under 3 px radius turns into a grey smudge. Use a straight corner, or a single 45° step.
- **Ellipses:** align their axes horizontally/vertically, with axis lengths that make both ends land on pixel centres.

## 6. Spacing, minimum sizes and mass

| Item | 16 px | 32 px |
| --- | --- | --- |
| Gap between two parallel lines | ≥ 1 px, preferably 2 px | ≥ 2 px |
| Point, grip or node (filled square) | 2 × 2 (or 3 × 3) | 3 × 3 or 4 × 4 |
| Smallest closed shape (inside) | 3 × 3 | 4 × 4 |
| Arrowhead | 3 px wide, 45° sides | 5–7 px wide |
| Text / letters | avoid; one bold letter, ≥ 5 px high, if it is the metaphor | up to 3 characters, ≥ 7 px high |
| Hatch / dash pattern | ≥ 2 px between strokes, ≤ 3 strokes | ≥ 2 px between strokes |

**Mass matters as much as sharpness.** Thin outlines alone read faint next to AutoCAD's icons.
- **Target ink:** about 25–40 % of the live area should carry ink: lines plus fills.
- **Pale fills:** closed shapes that stand for an object (a sheet, a block, a layer, a window) may get a pale fill of the accent colour at 15–25 % opacity, plus a full-strength outline. At 16 px, fill at most one shape.
- **Solid fills:** small, meaningful parts (a grip, the "current" layer, a selected item) may be solid accent fills. They are the sharpest thing an icon can have, because a filled square on whole pixels has no anti-aliasing at all.

## 7. Colour

The icons are recoloured to the theme. Use exactly these source colours:

| Source colour | Becomes | Use for |
| --- | --- | --- |
| `#ffffff` | theme text colour | base lines: the object |
| `#4a9eff` | theme primary | the accent: what the tool creates, changes or points at |
| `#e05050` | theme danger | remove, delete, warning |

- **Never `currentColor`:** resvg draws it black.
- **Accent share:** at most about one third of the ink. Typically it marks the result of the tool (the new line, the moved copy, the dimension).
- **Thin accent lines at 16 px:** a 1 px accent line on a dark background is weaker than a 1 px base line. Make accent parts larger (a fill, a 2 × 2 grip, a badge) rather than relying on a thin accent line.
- **Light and dark themes:** an icon must work on both. Check it on both before delivering.

## 8. Badges (modifiers)

A badge says *add*, *remove*, *edit* or *settings* on top of a base object.

| | 16 px | 32 px |
| --- | --- | --- |
| Badge area | bottom-right 7 × 7 | bottom-right 12 × 12 |
| Clear gap around the badge | 1 px cut out of the base shape | 2 px |

- **Shapes:** `+` for add/new, `×` in danger red for remove, an arrow for import/export, a pencil for edit, a gear for settings.
- **One badge per icon.** Use the same badge for the same meaning across the whole set.

## 9. Families and consistency

- **Shared drawings:** tools in one family share a base drawing and differ only in the part that changes. Examples: the Zoom options (one magnifier, different contents), Draw order (the same two squares, a different one in front), DIMTEDIT justify (the same dimension, text in a different place).
- **Same object, same drawing:** draw a sheet, a block or a layer the same way everywhere it appears.
- **Light comes from nowhere:** no shadows, gradients or 3D effects other than simple isometric line drawings (blocks, views).

## 10. File rules (what the pipeline accepts)

- **File names:** one SVG per icon and size, `assets/icons/v2/16/<key>.svg` and `assets/icons/v2/32/<key>.svg`, using the same key in both.
- **Root element:**

  ```svg
  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"
       fill="none" stroke="#ffffff" stroke-width="1" stroke-linecap="square" stroke-linejoin="miter">
  ```

  (32 px: `width="32" height="32" viewBox="0 0 32 32" stroke-width="2"`.)
- **Elements:** only `<path>` elements with `d`, `fill`, `stroke`, `stroke-width`, `stroke-linecap`, `stroke-linejoin`, and `fill-opacity` for pale fills.
  - No `<g>`, no transforms, no `<circle>` / `<rect>` (convert them to paths), no masks, filters or text.
  - Path commands: `M L H V A C S Q Z`, absolute or relative, with coordinates written out (no exponents).
- **Coordinates:** follow §3. A line coordinate is `.5` at 16 px and whole at 32 px; a fill coordinate is whole at both sizes.
- **Licence:** own designs only, GPL-3.0 like OCS.

## 11. Checklist before delivering an icon

1. **Size:** viewed at 100 % on a real screen, not zoomed in the design tool.
2. **Themes:** checked on the dark and the light theme.
3. **Squint test:** the icon is recognisable when slightly out of focus, and distinct from its neighbours in the same panel.
4. **Grid:** every straight line is on the grid. Zoomed in, there is no half-lit row or column next to a straight line, and no light pixel in a corner.
5. **Diagonals:** only 1:1, 2:1 or 1:2, at most two at 16 px.
6. **Minimum sizes:** no gap, point or shape smaller than §6 allows.
7. **Mass:** the icon does not look lighter than its neighbours (compare it in a row of 5–10 icons).
8. **Two sizes:** the 16 px and 32 px versions read as the same tool, and the 32 px version is not just the 16 px version scaled up.
9. **File:** inside the subset of §10.

## Pipeline changes this needs (for the developers)

The current pipeline (`src/ui/icon_snap.rs`) handles the 19-unit frame icons. To use these masters it needs to:
- Accept `viewBox="0 0 16 16"` and `"0 0 32 32"` as line icons.
- Pick the master by physical size: the 16 px master below 28 px, the 32 px master from 28 px.
- Scale it to `N` and snap as today. At exactly 16 or 32 px the master is already on the grid, so snapping changes nothing.
- Use the master's own line weight instead of the computed one at 16 and 32 px.
- Allow `fill-opacity`, `stroke-linecap` and `stroke-linejoin` on a `<path>` (not accepted today).
- Keep the 19-unit icons as the fallback for keys without a master.
