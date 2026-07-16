# AGENTS.md — zmind

A terminal mind mapping TUI built in Rust with [ratatui](https://github.com/ratatui-org/ratatui).

## Essential Commands

| Command | Purpose |
|---------|---------|
| `cargo build` | Compile |
| `cargo test` | Run all tests |
| `cargo run` | Launch the TUI |
| `cargo fmt` | Format code (rustfmt) |
| `cargo fmt -- --check` | Check formatting (CI-style) |

There is no Makefile, CI config, or linting setup beyond what Cargo provides by default (`cargo check`, `cargo clippy` if installed).

## Architecture & Data Flow

```
cli.rs           → parses --version / --help, exits immediately

main.rs::App     → TUI event loop + view-mode state machine
  ├─ mind_map/
  │   ├─ mod.rs    → data types (Node, NodeLayout, MindMap) + constructor
  │   ├─ edit.rs   → CRUD, clipboard, paste, text import/export
  │   ├─ nav.rs    → navigation, collapse/expand, move, marks, symbols
  │   ├─ display.rs → layout engine, canvas drawing, export (ASCII/HTML)
  │   ├─ undo.rs   → undo/redo stack management
  │   └─ tests.rs  → all unit tests (104 tests)
  ├─ store.rs    → JSON load/save via `dirs` crate to platform config dir
  └─ view.rs     → all ratatui rendering (no logic, pure presentation)
```

### Key Structure: `MindMap`

The entire mind map is a flat `HashMap<usize, Node>`. Node IDs are monotonically increasing integers (`next_id()` = max key + 1).

- **Node 0**: hidden anchor (parent=MAX, never rendered)
- **Node 1**: visible root (parent=0)

Each `Node` has: `title`, `parent: usize`, `children: Vec<usize>`, `collapsed`, `hidden`, `note`.

Every mutation that changes structure or text must call `push_undo()` first (saves full snapshot), then `refresh_display()` (rebuilds visible_nodes, layout, and canvas). Methods that modify only node text (toggle_symbol, toggle_numbers, add_star, remove_star, modify_positive_rank, modify_negative_rank, append_clipboard_to_title) also call `refresh_display()` to keep the canvas in sync.

### Display Pipeline

`refresh_display()` runs three phases in order:

1. **`collect_visible()`** — recursive walk from root; respects `collapsed` and `hidden` flags (unless `show_hidden` is set). Populates `visible_nodes: Vec<usize>` in depth-first order.

2. **`calculate_layout()`** — single post-order traversal (`layout_pass`) that computes x/y/w/h/depth for every visible node. Leaf nodes get 1.3× max width. If `align_levels` is set, all nodes at the same depth get the same x. Result stored in `layouts: HashMap<usize, NodeLayout>`.

3. **`build_canvas()`** — allocates `canvas: Vec<Vec<char>>` at `map_width × map_height`, draws connections first (box-drawing chars `─│╭╮╰╯├┤┬┴┼`), then draws multi-line wrapped node text on top.

### Viewport Rendering

`view.rs::show_mind_map()` slices the canvas by `app.viewport_x/y` and renders within the terminal area. Each canvas cell is a char. The active node's bounding rectangle is highlighted by wrapping those chars in a `Span` with `bg: Color::Rgb(215, 135, 0)`.

### App State Machine

`ViewMode` enum controls what the event loop handles and what gets rendered:

```
MapList → ViewMindMap → (EditNode | Search | ViewNodeDetails → EditNodeNote | ViewHelp)
            ↑                    |
            └── (Esc/q) ─────────┘
```

All mutations go through `App` methods that call `MindMap` methods, then `sync_maps()` (copy back to maps vec) and `auto_save()` (persist to disk). Non-mutating display changes (viewport, locks) do not trigger save.

## Gotchas & Non-Obvious Details

### `#[serde(skip)]` Fields

These MindMap fields are runtime-only and never serialized: `undo_stack`, `redo_stack`, `clipboard`, `visible_nodes`, `layouts`, `map_width`, `map_height`, `canvas`, `max_node_width`, `line_spacing`, `show_hidden`, `align_levels`.

### Width vs Display Width

`NodeLayout.w` stores **char count** (used for canvas allocation), while `display_w` (computed via `unicode-width`) is used for child X-position offset. They differ for CJK characters and other wide glyphs. The canvas uses `char` columns; wide chars occupy one cell visually but have `width() == 2`. The `export_ascii()` function handles this by inserting padding spaces per-column to align wide chars.

### Undo Stack

Full snapshots of `(nodes, root_id, active_node)` — not incremental diffs. Max 50 entries. Pushing a new undo snapshot clears the redo stack.

### Clipboard

Text-based (tab-indented format). Stored as `Option<String>`. Shared across all maps. `from_text()` parses tab-indented text back into a `MindMap`. The parser uses `min_indent` normalization and handles multi-root inputs by creating a virtual root (node 1).

### Collapse Behavior on Leaf Nodes

`toggle_node()` (Space key) on a leaf node toggles the parent's collapse instead. This matches h-m-m behavior.

### Navigation Wrapping

`go_up()`/`go_down()` wrap around within siblings (last → first, first → last). `go_right()` goes to the middle child.

### App ↔ MindMap Sync

`App` holds display config (`show_hidden`, `align_levels`, `focus_lock`, `center_lock`, `viewport_x/y`) that the `MindMap` model does not know about. `sync_config()` pushes these into MindMap and calls `refresh_display()`. `sync_maps()` clones MindMap back into `maps[active_index]`.

### Viewport Bounds

Ctrl+arrows scroll with `saturating_sub/add` and `map_width/map_height` as max bounds. No wrapping. `c` centers with a fixed offset of (-20, -10) from the active node's layout position.

### Test Location

Tests live in `src/mind_map/tests.rs` under `#[cfg(test)] mod tests`. `find_node_by_title()` helper is in `src/mind_map/display.rs` (behind `#[cfg(test)]`).

### Dependencies

- `ratatui 0.27` + `crossterm` (TUI framework)
- `tui-input 0.9` (text input widget with cursor support)
- `unicode-width 0.1` (display width for CJK/wide chars)
- `serde` + `serde_json` (persistence)
- `dirs 5` (platform config directory)

## Conventions

- Rust edition 2021
- No `pub` on fields without explicit decision — MindMap fields are `pub` for direct access from `App` and `View`
- `View` is a stateless struct with only static methods (no fields, no `self` besides `&self`)
- `Store` is similarly a stateless struct with only static methods
- Error handling: loosely uses `Box<dyn Error>` in `main()`, `io::Result` in `run()`, `.ok()` for recoverable failures in I/O
- No logging framework; messages shown via `show_message()` modal
