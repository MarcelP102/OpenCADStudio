# Fill by layer

A closed shape drawn on a layer gets the fill style of that layer, the way
classes work in Vectorworks. The fill is an ordinary associative hatch, so the
drawing stays a plain DWG. Only the fill style per layer is OpenCADStudio data.

Status: proof of concept on a feature branch. Off by default (`FILLBYLAYER 0`).

## Using it

| Command | What it does |
| --- | --- |
| `FILLBYLAYER 1` | Turn filling by layer on for this drawing (`0` turns it off). |
| `LAYERFILL` | Report the fill style of the current layer. |
| `LAYERFILL <layer> None` | Remove a layer's fill style. |
| `LAYERFILL <layer> Solid` | Fill shapes on the layer solid. |
| `LAYERFILL <layer> <pattern> [scale] [angle] [transparency]` | Fill with a hatch pattern, such as `ANSI31 2 45 30`. |
| `FILLBYLAYERAPPLY [layer ...\|*]` | Fill closed shapes already in the drawing (current space) on these layers; `*` is every layer with a fill style. |
| `FILLBYLAYERRESET` | Give the selected shapes their layer's fill again. |

The Layer Manager has a Fill column (None, Solid or a pattern) and a Fill
color column (ByLayer, the standard colours or Select Color...). A change of a
layer's style, in the column or with `LAYERFILL`, reaches every fill that
follows the layer in the same undo step; a layer set to None loses those
fills. A fill changed in the Graphic Attributes palette is marked as changed
(`OCS_FILL = 2`) and keeps its own style until `FILLBYLAYERRESET`.

A layer name with spaces goes in double quotes. `.` is the current layer.
`color=<colour>` gives the fill its own colour, apart from the layer's line
colour: `ByLayer`, an index 1–255 or `r,g,b`, for example
`LAYERFILL Walls Solid color=200,180,120`.

## Where the data lives

| What | Where | Format |
| --- | --- | --- |
| Fill style of a layer | XRecord `OCS_LAYER_FILL` in the extension dictionary of the layer table record | Group codes 70 version, 71 kind (0 none, 1 solid, 2 pattern), 2 pattern name, 62/420 colour, 63/421 background, 440 transparency (0–90 %), 41 scale, 50 angle (degrees) |
| FILLBYLAYER | Drawing variable in the `AcDbVariableDictionary` of the named object dictionary | `DICTIONARYVAR`, value `0` or `1` |
| Fill made from a layer style | XDATA `OCS_FILL` on the hatch | Integer16: 1 follows the layer, 2 changed by the user |

FILLBYLAYER is stored as a drawing variable rather than in an XRecord
`OCS_SETTINGS`: the variable dictionary is where AutoCAD keeps its own per
drawing variables, and the code already reads and writes it
(`crate::io::drawing_variable`, as `HPLAYER` does).

The fill and its shape need no extra link: the hatch is associative, so its
boundary handles point at the shape.

## Commit routes (phase 0)

The new logic is in `src/app/layer_fill/`. Existing files only get calls:

| Route | Function | Filled |
| --- | --- | --- |
| A drawing command ends with one shape (`RECTANG`, `POLYGON`, `CIRCLE`, `ELLIPSE`, `SPLINE`) | `handle_commit_and_exit` | yes |
| A repeating command commits one shape | `handle_commit_entity` | yes |
| A command commits several shapes | `handle_commit_entities`, `handle_commit_entities_and_exit` | yes, when the command is a drawing command |
| `PLINE` (built live, vertex by vertex) | `handle_update_live_entity` (finish), `handle_finalize_live_entity` | yes, as its own undo step |
| Paste, insert, import, xref, PDF import, COUNT, hyperlinks, node graph, plugins | `commit_entity_handle*` called directly, or a non-drawing command | no |

Only the drawing commands in `apply::DRAW_COMMANDS` fill. That keeps paste,
import, blocks, xrefs and plugins out, as the plan asks. Plugins that draw
through a drawing command name would be filled; plugin commands have their own
names, so they are not.

A step that may add a fill is undone from a structure snapshot rather than an
entity delta: the fill changes the sort table (draw order behind the shape) and
may register the `OCS_FILL` application. Steps that add no fill keep the delta.

`replace_graphic_fills` in the Graphic Attributes palette builds its hatch from
the whole scene's boundary sources and knows the palette's options. The fill by
layer uses the same `HatchCommand::finish_selected` and `add_hatch` route, with
the source of the one shape (`entity_boundary_source_on_plane`), so drawing
stays fast on large drawings. The palette code is unchanged.

## Known limits of the proof of concept

- `PLINE` builds its polyline live and closes its undo step when it ends. Its
  fill is a second undo step: the first undo removes the fill, the second the
  polyline.
- The Layer Manager's Fill and Fill color columns pick the pattern and the
  fill colour. Background, scale, angle and transparency are set with
  `LAYERFILL` for now; the dialog for them is still to come.
- A saved Layer Manager column layout from before this branch shows the new
  columns at the end, after Description.
- A shape and its marked fill select as one (`with_layer_fills`, called where
  the selection expands leaders), so erase, move, copy and the Properties
  palette act on both. Clicking the fill selects the shape too; to remove only
  the fill, choose fill None in the Graphic Attributes palette.
- A shape moved to another layer takes its fill along (both are selected) but
  the fill keeps its style until the layer style is applied again (phase 5).
