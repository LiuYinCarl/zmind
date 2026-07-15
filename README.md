# zmind -- Terminal Mind Mapping Tool

**zmind** is a keyboard-centric, terminal-based mind mapping tool built in Rust using [ratatui](https://github.com/ratatui-org/ratatui). Inspired by [h-m-m](https://github.com/nadrad/h-m-m) and following the architecture of [basilk](https://github.com/GabAlpha/basilk).

## Features

- Visual mind map rendering with box-drawing connector lines
- Full keyboard navigation (vim-style + arrow keys)
- Create, edit, delete nodes (siblings and children)
- Cut, copy, paste (clipboard operations)
- Collapse/expand nodes, focus mode, level-based collapse
- Undo/redo support
- Search functionality
- JSON file format with note persistence per node
- Multi-line node text support
- Node detail view with editable notes
- File picker for managing multiple mind maps
- HTML and ASCII export
- Node marking (symbols, numbering, ranking, stars)
- Sort siblings, move nodes up/down, hide/show nodes
- Chinese/UTF-8 text support

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Open file picker (list .json files in current directory)
zmind

# Open a specific mind map
zmind my_map.json
```

### File Format

Mind maps are stored as JSON files. Example:

```json
{
  "nodes": {
    "0": { "title": "", "parent": 18446744073709551615, "children": [1], "collapsed": false, "hidden": true, "note": "" },
    "1": { "title": "root", "parent": 0, "children": [2, 3], "collapsed": false, "hidden": false, "note": "" },
    "2": { "title": "item A", "parent": 1, "children": [], "collapsed": false, "hidden": false, "note": "optional note" },
    "3": { "title": "item B", "parent": 1, "children": [4], "collapsed": false, "hidden": false, "note": "" },
    "4": { "title": "item Ba", "parent": 3, "children": [], "collapsed": false, "hidden": false, "note": "" }
  },
  "root_id": 1,
  "active_node": 1,
  "filename": null,
  "modified": true
}
```

## Key Bindings

### Navigation

| Key | Action |
|-----|--------|
| `Left` | Go to parent |
| `l` / `Right` | Go to first child |
| `j` / `Down` | Go down (next sibling) |
| `k` / `Up` | Go up (previous sibling) |
| `g` | Go to top |
| `G` | Go to bottom |
| `m` / `~` | Go to root |
| `Ctrl+arrows` | Scroll viewport |
| `c` | Center on active node |

### Editing

| Key | Action |
|-----|--------|
| `o` / `Enter` | Insert sibling |
| `O` / `Tab` | Insert child |
| `e` / `a` | Edit node (append) |
| `E` / `A` | Edit node (replace) |
| `d` | Cut node (to clipboard) |
| `D` | Cut children (to clipboard) |
| `Delete` | Delete node (without clipboard) |
| `y` | Copy node |
| `Y` | Copy children |
| `p` | Paste as children |
| `P` | Paste as siblings |
| `Ctrl+p` | Append clipboard to title |

### Collapse / Expand

| Key | Action |
|-----|--------|
| `Space` | Toggle collapse |
| `v` | Collapse all to level 1 |
| `V` | Collapse children |
| `b` | Expand all |
| `f` | Focus on active node |
| `r` | Collapse other branches |
| `R` | Collapse inner branches |
| `1` - `9` | Collapse to level N |

### View

| Key | Action |
|-----|--------|
| `i` / `I` | Node detail view (title + note) |
| `e` (in detail) | Edit note |
| `F` | Toggle focus lock |
| `C` | Toggle center lock |
| `Ctrl+h` | Toggle show hidden nodes |
| `\|` | Toggle aligned levels |
| `w` / `W` | Increase / decrease node width |
| `z` / `Z` | Decrease / increase line spacing |

### Marks

| Key | Action |
|-----|--------|
| `t` | Toggle checkmark / cross / none |
| `#` | Toggle numbering |
| `+` / `=` | Modify positive rank |
| `-` / `_` | Modify negative rank |
| `Alt+j` | Add star |
| `Alt+k` | Remove star |

### Move / Sort

| Key | Action |
|-----|--------|
| `J` | Move node down |
| `K` | Move node up |
| `T` | Sort siblings |
| `H` | Toggle hidden |

### File / Export

| Key | Action |
|-----|--------|
| `s` | Save |
| `S` | Save as |
| `x` | Export HTML |
| `X` | Export text map to clipboard |
| `Ctrl+e` | Export ASCII art to file |
| `Ctrl+o` | Open link (xdg-open) |
| `u` | Undo |
| `Ctrl+r` | Redo |
| `/` | Search |
| `n` / `N` | Next / previous search result |

### General

| Key | Action |
|-----|--------|
| `h` / `?` | Show keybindings |
| `q` | Quit (prompts if unsaved) |
| `Q` | Force quit |
| `Esc` | Back to file picker |

### File Picker

| Key | Action |
|-----|--------|
| `Up` / `Down` / `j` / `k` | Move selection |
| `Enter` | Open selected file |
| `n` | New mind map |
| `q` | Quit |

## License

MIT OR Apache-2.0
