# zmind -- Terminal Mind Mapping Tool

Keyboard-centric terminal mind mapping tool built in Rust with [ratatui](https://github.com/ratatui-org/ratatui).
Inspired by [h-m-m](https://github.com/nadrad/h-m-m), architecture follows [basilk](https://github.com/GabAlpha/basilk).

## Features

- Visual mind map with box-drawing connector lines
- Vim-style + arrow key navigation
- Create, edit, delete nodes (siblings and children)
- Cut, copy, paste with internal clipboard
- Collapse/expand nodes (by level, focus, branches)
- Undo/redo
- Search
- Auto-save to `~/.config/zmind/data.json`
- Multiple named mind maps (no file management needed)
- Multi-line node text (`\n` for newlines)
- Node detail view with editable notes
- HTML and ASCII export
- Node marking (checkmarks, numbering, ranking, stars)
- Sort, move, hide/show nodes
- UTF-8 support (Chinese, etc.)

## Installation

```bash
cargo install --path .
```

## Usage

```bash
zmind
```

Opens the map list. From there:

- `Enter` opens the selected map
- `n` creates a new map (prompts for name)
- `r` renames the selected map
- `d` deletes the selected map
- `q` quits

All changes are saved automatically. Press `Esc` or `q` from a map to return to the list.

### Data

All mind maps are stored in a single file:

| OS | Path |
|----|------|
| Linux | `~/.config/zmind/data.json` |
| macOS | `~/Library/Application Support/zmind/data.json` |
| Windows | `%APPDATA%\zmind\data.json` |

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
| `d` | Cut node |
| `D` | Cut children |
| `Delete` | Delete node (no clipboard) |
| `y` | Copy node |
| `Y` | Copy children |
| `p` | Paste as children |
| `P` | Paste as siblings |
| `Ctrl+p` | Append clipboard text to title |

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
| `F` | Toggle focus lock |
| `C` | Toggle center lock |

### View

| Key | Action |
|-----|--------|
| `i` / `I` | Node detail (title + note) |
| `e` (in detail) | Edit note |
| `Ctrl+h` | Toggle show hidden |
| `\|` | Toggle aligned levels |
| `w` / `W` | Increase / decrease node width |
| `z` / `Z` | Decrease / increase spacing |

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

### Export / History

| Key | Action |
|-----|--------|
| `x` | Export HTML |
| `X` | Export text map to clipboard |
| `Ctrl+e` | Export ASCII art to `zmind_export.txt` |
| `Ctrl+o` | Open link (xdg-open) |
| `u` | Undo |
| `Ctrl+r` | Redo |
| `/` | Search |
| `n` / `N` | Next / previous search result |

### General

| Key | Action |
|-----|--------|
| `h` / `?` | Show keybindings |
| `q` | Back to map list |
| `Q` | Force quit |
| `Esc` | Back to map list |

### Map List

| Key | Action |
|-----|--------|
| `Up` / `Down` / `j` / `k` | Move selection |
| `Enter` | Open map |
| `n` | New map |
| `r` | Rename map |
| `d` | Delete map |
| `q` | Quit |

## License

MIT OR Apache-2.0
