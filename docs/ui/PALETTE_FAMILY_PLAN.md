# OCS — every palette in the Properties family

Goal: one default palette style. Every docked or floating palette looks and behaves like the Properties palette of the redesign: same title bar, same insets, same rows, colours, text and scrollbar. Palettes are built from one shared widget kit; a new palette is assembled from it, never styled by hand.

Checked against `ui/text-pallets-documenttabs-redesign` @ `af3e9cb`. Replaces the "Later" section of `INTEGRATION_PLAN.md`; `WIDGET_AUDIT.md` is no longer a separate step (its survey is section 2 below; re-run its counts script as the CI check of prompt 9).

## 1. The palette anatomy (the default)

Every palette is the same stack, top to bottom; each band is optional except the title bar and the content:

| Band | Look (Properties today) | Shared widget |
|---|---|---|
| Title bar | 23 px, `palette_body`, 13 px capitals, ☰ + close always, detach + pin on hover | `dock::title_bar_with` ✅ |
| Toolbar | a row of 22 px square icon buttons, 4 px gaps | `kit::toolbar` 🆕 (from `dock::tool_button`) |
| Selector / search | 22 px `palette_bar` box, 8 px left / 10 px right inset (the object list box) | `kit::select_box`, `kit::search_field` 🆕 |
| Tabs | segmented row (Block Palette, Tool / Authoring Palettes, Sheet Set) | `kit::tabs` 🆕 |
| Content | one of: property grid · list with columns · tree · cards · custom canvas | grid ✅ `prop_grid`; list, tree 🆕 |
| Category bars | 18 px `palette_bar`, 11 px semibold, caret right, 3 px gap above | `prop_grid::category_bar` ✅ |
| Scroll area | 8 / 10 px insets, floating 10 px bar with 6 px thumb in a reserved gutter | `prop_grid::scroll_area` ✅ |
| Empty state | one muted line, 10 px, inset like the grid | `kit::empty_state` 🆕 |
| Footer | one 18 px status line (Layer Manager "Current layer: …") | `kit::footer` 🆕 |
| Frame | `palette_body`, splitter between stacked palettes | `dock::frame`, `SplitterGrip` ✅ |
| Menus / tooltips | `palette_menu`, one tooltip style | `palette_menu` ✅, `kit::tooltip` 🆕 |

Rules for every palette: colours only from `chrome::`, text only from `text::`, sizes only from `palette_metrics`; rows 18 px (`palette_metrics::ROW_H`); no `theme.palette()` pairs, no literal sizes. `ROW_H = 26` (`src/ui/mod.rs`) is retired at the end.

## 2. Where each palette stands

Counts per file on `af3e9cb` (`theme.palette()` = generic iced colours still used; "literals" = `.size(n)`, `.padding(n)`, fixed widths/heights with numbers).

| Palette | File | Lines | Content today | `palette()` | literals | Kit pieces it needs | Size |
|---|---|---|---|---|---|---|---|
| Properties | `ui/properties.rs` | 3150 | property grid | done | done | — (reference) | — |
| Graphic Attributes | `window/graphic_attributes.rs` | 1460 | name/value rows, combos, own `ROW_H*0.42` text | 14 | 32 | grid, category bar | S |
| Browser | `window/browser.rs` | 150 | own `section_header` + tree | 1 | 6 | category bar, tree row | S |
| Point Cloud Manager | `window/pc_manager.rs` | 420 | search + list | 4 | 13 | search, list row, empty state | S |
| Command History | `ui/command_line.rs` (`history_panel`) | — | text list | 42 (whole file) | 48 | list row, scroll area | S |
| Node Graph | `ui/node_graph.rs` | 2180 | canvas + toolbar | 13 | 32 | toolbar, frame only | S |
| Authoring Palettes | `window/authoring_palettes.rs` | 510 | tabs + tool buttons | 6 | 11 | tabs, list row (icon + name) | M |
| Tool Palettes | `window/tool_palettes.rs` | 510 | tabs + tools, column sort | 6 | 31 | tabs, list row | M |
| Parameters Manager | `window/parameters_manager.rs` | 1500 | editable table, own `header_row` | 3 | 24 | list with columns, header, field styles | M |
| Count | `window/count_palette.rs` | 860 | toolbar (`tool_button`) + table | 8 | 44 | toolbar, list with columns, empty state | M |
| Layer Walk | `window/layer_walk.rs` | 1810 | combos + list, `ROW_H`, own `FONT_SZ` | 27 | 39 | select box, list row, footer | M |
| QuickCalc | `window/quick_calc/view.rs` | 960 | toolbar, history, keypad, own `section_header` | 1 | 41 | toolbar, category bar, field styles; keypad stays custom | M |
| Block Palette | `window/block_palette.rs` | 1710 | tabs, filter + view combo, cards, "OPTIONS" section with checkboxes and X/Y/Z fields | 17 | 46 | tabs, search, select box, category bar, grid (options), card grid | L |
| External References | `window/xref_manager.rs` | 2800 | toolbar, table with column drag, tree, details pane split | 48 | 35 | toolbar, list with columns + resizable header, tree, grid (details), splitter | L |
| Layers | `window/layers.rs` | 2940 | toolbar, filter tree, table with toggle columns, inline rename, footer | 52 | 57 | toolbar, search, tree, list with columns + toggle cells, footer | L |
| Sheet Set Manager | `window/sheet_set.rs` | 2320 | tabs, trees, own `category_view`, 10 scroll areas | 15 | 134 | tabs, tree, category bar, grid, scroll area | L |
| Plugin panels | `window/plugin_panel.rs` | 1700 | the plugin's `DialogControl` list (fields, buttons, lists) | — | — | map each control onto the kit | M |

Already shared and correct: title bar, frame, splitter, strip, palette menu, property grid (row, category bar, scroll area, field / combo / read-only styles, cell buttons, selected-row controls).

Hand-built copies to replace (one each, found in the survey): `section_header` in QuickCalc and Browser, `category_view` in Sheet Set, `header_row` in Parameters Manager, the search inputs of Layers / Block Palette / Point Cloud Manager, the own list rows and column headers of Layers, Xref, Layer Walk, Count, Tool Palettes, Parameters Manager.

## 3. The kit

Move the shared pieces into one module so palettes import one thing:

```
src/ui/kit/
  mod.rs          re-exports; the palette anatomy in its doc comment
  grid.rs         ← prop_grid.rs (row_view, category_bar, scroll_area, field styles, …)
  toolbar.rs      toolbar(buttons), tool_button (from dock::tool_button), toggle state
  field.rs        search_field, select_box (combo head), number_field
  list.rs         list_row(cells, selected, on_select), column_header(cols, sort, on_resize),
                  toggle_cell(icon on/off), text_cell, icon_cell
  tree.rs         tree_row(depth, expanded, icon, label, selected)
  tabs.rs         tabs(items, active) — segmented, palette_bar track, palette_body active
  cards.rs        card_grid(cards, card_w) — Block Palette's large icons
  status.rs       empty_state(text), footer(text)
  tooltip.rs      one tooltip style (chrome::panel, 1 px border, UI_SMALL text)
```

`dock::title_bar_with`, `dock::frame`, `SplitterGrip` and `palette_menu` stay where they are (dock chrome); `kit::mod` re-exports them.

Shared behaviour the list widgets must support (from the survey), so no palette needs its own copy:
- selection: single / multi (Ctrl, Shift), `row_selected` colour, keyboard up/down;
- right-click → `palette_menu` at the pointer;
- double-click and inline rename (Layers, Xref, Sheet Set);
- toggle cells with their own click (Layers on/freeze/lock/plot);
- column header: sort arrow, drag to resize (Xref, Layers already persist widths), min width;
- tree: indent 14 px per level (= `INDENT`), caret, lazy children.

## 4. Prompts

One PR each, in this order. Every prompt ends with screenshots at 100 / 125 / 150 %, Studio Dark and Light (`UI_SHOTS_ONLY=<scene> cargo test --lib ui_shots -- --ignored`; add a scene per palette), checked against Properties.

| # | Prompt | Files | Size |
|---|---|---|---|
| 1 | **Kit module.** Move `prop_grid.rs` to `ui/kit/grid.rs` (keep a `prop_grid` re-export for one release), add `kit/mod.rs`, `status.rs` (empty state, footer), `tooltip.rs`; switch Properties to the tooltip style. No look change. | `ui/kit/*`, `ui/prop_grid.rs`, `ui/properties.rs` | S |
| 2 | **Toolbar, search, select box, tabs.** Build them from Properties' object list box and `dock::tool_button`; tests for sizes. Use them first in Point Cloud Manager and Browser (with `category_bar`). | `kit/toolbar.rs`, `kit/field.rs`, `kit/tabs.rs`, `pc_manager.rs`, `browser.rs` | M |
| 3 | **List row, column header, tree row.** Selection, right-click, double-click, toggle cells, sort and resize. Use in Command History and Count. | `kit/list.rs`, `kit/tree.rs`, `command_line.rs`, `count_palette.rs` | M |
| 4 | **Graphic Attributes and QuickCalc** onto grid + category bar + toolbar; QuickCalc keypad keeps its own buttons but takes `chrome::` colours and `text::` sizes. | `graphic_attributes.rs`, `quick_calc/*` | M |
| 5 | **Tool Palettes, Authoring Palettes, Parameters Manager** onto tabs + list rows + column header. | 3 files | M |
| 6 | **Layer Walk and Node Graph** (list + select box + footer; Node Graph frame and toolbar only). | 2 files | M |
| 7 | **Block Palette**: tabs, search, select box, `kit::cards` for the icon views, the OPTIONS section as a `category_bar` + grid rows (checkbox rows become boolean value cells). | `block_palette.rs`, `kit/cards.rs` | L |
| 8a | **Layer Manager**: toolbar, search, filter tree, list with toggle columns, inline rename, footer. Keep its persisted column widths. | `layers.rs` | L |
| 8b | **External References**: same list and tree, details pane as a grid, splitter = `SplitterGrip`. | `xref_manager.rs` | L |
| 8c | **Sheet Set Manager**: tabs, trees, `category_view` → `category_bar`, one scroll area per pane. | `sheet_set.rs` | L |
| 9 | **Retire and guard.** Remove `ROW_H = 26`, the `prop_grid` re-export and the now unused local `FONT_SZ` / `COMBO_PAD_V`. Add `docs/ui/audit_counts.sh` and a CI step (in the informational lint job, like rustfmt and clippy) that reports when a palette file (the list in section 2) uses `theme.palette()` / `background.*` / `primary.*`, `Color::from_rgb*` or a literal in `.size/.padding/.spacing/.width/.height`. | `ui/mod.rs`, CI, script | S |
| 10 | **Plugin panels.** Map each `DialogControl` (fields, buttons, text inputs, lists) onto the kit, so plugin palettes join the family without plugin changes. | `window/plugin_panel.rs` | M |

## 5. Rules for every prompt

- Behaviour unchanged: same messages, commands, keyboard, drag, persisted state (column widths, collapsed sections, tab choice).
- Tokens only (as in `INTEGRATION_PLAN.md`): colours from `chrome::`, text from `text::`, sizes from `palette_metrics` (add a named constant there if one is missing).
- Format only touched files (`rustfmt --edition 2021 <files>`), never `cargo fmt` on the tree; `cargo clippy --workspace --all-targets` adds no warnings in touched code; `cargo test --workspace` (run with `-j 1` here: the lib and lib-test builds don't fit in memory together).
- Tests that assert sizes or colours (`theme_accessibility_tests.rs`, `app/dock/tests.rs`, per-palette tests that find text in the view) move to the tokens and keep their contrast thresholds.

## 6. Risks

- **Layer Manager and Xref tables**: column resizing, toggle columns, inline rename and their tests (`app/dock/tests.rs` drives the Layer Manager's real view, e.g. the group-node test) — the list widget must support all of it before 8a/8b, or those prompts grow into L+.
- **Rows go from 26 to 18 px**: inputs, checkboxes and icon toggles must fit 18 px (16 px icons, 1 px field padding); some palettes may need a larger row for icon cells — add `ROW_H_ICON` rather than a literal.
- **Block Palette cards and QuickCalc keypad** are not grids; they join the family through colours, text, frame, toolbar and tabs, not through rows.
- **Plugin panels** depend on the plugin API's widget mapping; keep the API unchanged.
- **Wide palettes** (Xref, Layers default 460 px): the fixed 118 px name column is for property grids only; tables keep their own column widths.

## 7. Decided

- Properties is the reference; the kit's first widgets are the ones Properties already uses.
- Order: kit first, small palettes next to prove each widget, the three large tables last.
- One PR per prompt; screenshots against Properties in every PR.
