# zmind - Terminal Mind Mapping Tool

**zmind** is a keyboard-centric terminal-based mind mapping tool, inspired by [h-m-m](https://github.com/nadrad/h-m-m) and built in Rust.

## Features

- 🌳 Tree-based mind map visualization in the terminal
- ⌨️ Full keyboard navigation (hjkl, arrow keys)
- ✏️ Create, edit, delete nodes (siblings and children)
- 📋 Cut, copy, paste (clipboard operations)
- 🗂️ Collapse/expand nodes, focus mode
- ↩️ Undo/redo support
- 🔍 Search functionality
- 💾 Save/load mind map files (.hmm format)
- 🏷️ Node marking (symbols, numbering, ranking, stars)
- 🔤 Sort siblings, move nodes up/down
- 🙈 Hide/show nodes

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Open a new mind map
zmind

# Open an existing .hmm file
zmind my_map.hmm
```

### File Format

Mind maps are stored in plain text files using tab indentation to represent hierarchy:

```
root
    item A
    item B
        item Ba
        item Bb
            item BbX
    item C
```

## Key Bindings

### Navigation
- `h` / `←` - Go to parent
- `l` / `→` - Go to first child
- `j` / `↓` - Go down (next sibling)
- `k` / `↑` - Go up (previous sibling)
- `g` - Go to top
- `G` - Go to bottom
- `m` / `~` - Go to root

### Editing
- `o` / `Enter` - Insert sibling
- `O` / `Tab` - Insert child
- `e` / `i` / `a` - Edit node (append)
- `E` / `I` / `A` - Edit node (replace)
- `d` - Cut node (to clipboard)
- `D` - Cut children (to clipboard)
- `Delete` - Delete node (without clipboard)
- `y` - Copy node
- `Y` - Copy children
- `p` - Paste as children
- `P` - Paste as siblings

### Collapse/Expand
- `Space` - Toggle collapse
- `v` - Collapse all to level 1
- `V` - Collapse children
- `b` - Expand all
- `f` - Focus on active node
- `r` - Collapse other branches
- `R` - Collapse inner branches
- `1`-`9` - Collapse to level N

### Marks
- `t` - Toggle ✓ / ✗ / none
- `#` - Toggle numbering
- `+`/`=` - Modify positive rank
- `-`/`_` - Modify negative rank
- `Alt+j` - Add star
- `Alt+k` - Remove star

### Move/Sort
- `J` - Move node down
- `K` - Move node up
- `T` - Sort siblings
- `H` - Toggle hidden

### File
- `s` - Save
- `S` - Save as
- `u` - Undo
- `Ctrl+r` - Redo
- `/` - Search
- `n` / `N` - Next/previous search result
- `q` - Quit (prompts if unsaved)
- `Q` - Force quit
- `?` - Show keybindings

## License

MIT OR Apache-2.0
