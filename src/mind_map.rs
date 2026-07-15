use std::{collections::HashMap, fs, io, path::PathBuf};

/// A single node in the mind map tree.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub title: String,
    pub parent: usize,
    pub children: Vec<usize>,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub note: String,
}

impl Node {
    fn new(title: String, parent: usize) -> Self {
        Node {
            title,
            parent,
            children: Vec::new(),
            collapsed: false,
            hidden: false,
            note: String::new(),
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// Computed layout for rendering a node.
#[derive(Debug, Clone)]
pub struct NodeLayout {
    pub id: usize,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub depth: usize,
    /// Y offset of text within the node's bounding box (centering)
    pub yo: usize,
    /// Number of lines for multi-line wrapping
    pub lines: usize,
}

/// The entire mind map state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MindMap {
    pub nodes: HashMap<usize, Node>,
    pub root_id: usize,
    pub active_node: usize,
    pub filename: Option<PathBuf>,
    pub modified: bool,

    // Undo stack
    #[serde(skip)]
    undo_stack: Vec<UndoSnapshot>,
    #[serde(skip)]
    redo_stack: Vec<UndoSnapshot>,

    // Clipboard
    #[serde(skip)]
    pub clipboard: Option<String>,

    // Visible node list (for tree display)
    #[serde(skip)]
    pub visible_nodes: Vec<usize>,

    // Layout cache
    #[serde(skip)]
    pub layouts: HashMap<usize, NodeLayout>,
    #[serde(skip)]
    pub map_width: usize,
    #[serde(skip)]
    pub map_height: usize,

    // Canvas
    #[serde(skip)]
    pub canvas: Vec<Vec<char>>,
    #[serde(skip)]
    pub max_node_width: usize,
    #[serde(skip)]
    pub line_spacing: usize,
    #[serde(skip)]
    pub show_hidden: bool,
    #[serde(skip)]
    pub align_levels: bool,
}

#[derive(Debug, Clone)]
struct UndoSnapshot {
    pub nodes: HashMap<usize, Node>,
    pub root_id: usize,
    pub active_node: usize,
}

impl MindMap {
    const MAX_UNDO: usize = 50;

    /// Create an empty mind map with a single root node.
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        // Node 0 is a hidden anchor
        nodes.insert(
            0,
            Node {
                title: String::new(),
                parent: usize::MAX,
                children: vec![1],
                collapsed: false,
                hidden: true,
                note: String::new(),
            },
        );
        // Node 1 is the visible root
        nodes.insert(
            1,
            Node {
                title: "root".to_string(),
                parent: 0,
                children: Vec::new(),
                collapsed: false,
                hidden: false,
                note: String::new(),
            },
        );

        let mut mm = MindMap {
            nodes,
            root_id: 1,
            active_node: 1,
            filename: None,
            modified: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: None,
            visible_nodes: Vec::new(),
            layouts: HashMap::new(),
            map_width: 0,
            map_height: 0,
            canvas: Vec::new(),
            max_node_width: 40,
            line_spacing: 1,
            show_hidden: false,
            align_levels: false,
        };
        mm.refresh_display();
        mm
    }

    /// Load from a JSON file.
    pub fn from_file(path: &PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let mut mm: Self = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        mm.filename = Some(path.clone());
        mm.modified = false;
        mm.undo_stack.clear();
        mm.redo_stack.clear();
        mm.clipboard = None;
        mm.refresh_display();
        Ok(mm)
    }

    /// Save to JSON file.
    pub fn save(&mut self) -> io::Result<()> {
        if let Some(ref path) = self.filename {
            let json = serde_json::to_string_pretty(self)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            fs::write(path, &json)?;
            self.modified = false;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "No filename set"))
        }
    }

    pub fn save_as(&mut self, path: PathBuf) -> io::Result<()> {
        self.filename = Some(path);
        self.save()
    }

    // ─── Navigation ────────────────────────────────────────────────    // ─── Navigation ────────────────────────────────────────────────

    pub fn go_left(&mut self) {
        let current = self.active_node;
        if current <= self.root_id {
            return;
        }
        if let Some(node) = self.nodes.get(&current) {
            let parent = node.parent;
            if self.nodes.contains_key(&parent) && parent != 0 {
                self.active_node = parent;
            }
        }
    }

    pub fn go_right(&mut self) {
        let current = self.active_node;
        if let Some(node) = self.nodes.get(&current) {
            if !node.children.is_empty() && !node.collapsed {
                let mid = node.children.len() / 2;
                self.active_node = node.children[mid];
            }
        }
    }

    pub fn go_up(&mut self) {
        let current = self.active_node;
        if let Some(parent_id) = self.get_parent_id(current) {
            if let Some(parent) = self.nodes.get(&parent_id) {
                if let Some(pos) = parent.children.iter().position(|&c| c == current) {
                    if pos > 0 {
                        self.active_node = parent.children[pos - 1];
                    } else if parent.children.len() > 1 {
                        // Wrap around
                        self.active_node = *parent.children.last().unwrap();
                    }
                }
            }
        }
    }

    pub fn go_down(&mut self) {
        let current = self.active_node;
        if let Some(parent_id) = self.get_parent_id(current) {
            if let Some(parent) = self.nodes.get(&parent_id) {
                if let Some(pos) = parent.children.iter().position(|&c| c == current) {
                    if pos + 1 < parent.children.len() {
                        self.active_node = parent.children[pos + 1];
                    } else if parent.children.len() > 1 {
                        // Wrap around
                        self.active_node = parent.children[0];
                    }
                }
            }
        }
    }

    pub fn go_to_root(&mut self) {
        self.active_node = self.root_id;
    }

    pub fn go_to_top(&mut self) {
        self.active_node = self.visible_nodes.first().copied().unwrap_or(self.root_id);
    }

    pub fn go_to_bottom(&mut self) {
        self.active_node = self.visible_nodes.last().copied().unwrap_or(self.root_id);
    }

    fn get_parent_id(&self, id: usize) -> Option<usize> {
        self.nodes.get(&id).map(|n| n.parent)
    }

    // ─── Editing ───────────────────────────────────────────────────

    fn next_id(&self) -> usize {
        self.nodes.keys().max().map(|m| m + 1).unwrap_or(2)
    }

    pub fn edit_node(&mut self, new_title: String) {
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&self.active_node) {
            node.title = new_title;
        }
        self.modified = true;
        self.refresh_display();
    }

    pub fn update_note(&mut self, note: String) {
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&self.active_node) {
            node.note = note;
        }
        self.modified = true;
    }

    pub fn insert_sibling(&mut self) {
        self.push_undo();
        let current = self.active_node;
        let parent_id = self.get_parent_id(current).unwrap_or(0);

        if parent_id == 0 && current == self.root_id {
            // Root has no siblings, make a child instead
            self.insert_child();
            return;
        }

        let new_id = self.next_id();
        let new_node = Node::new("NEW".to_string(), parent_id);

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            // Insert after current node
            if let Some(pos) = parent.children.iter().position(|&c| c == current) {
                parent.children.insert(pos + 1, new_id);
            } else {
                parent.children.push(new_id);
            }
        }

        self.nodes.insert(new_id, new_node);
        self.active_node = new_id;
        self.modified = true;
        self.refresh_display();
    }

    pub fn insert_child(&mut self) {
        self.push_undo();
        let current = self.active_node;

        // Uncollapse if collapsed
        if let Some(node) = self.nodes.get_mut(&current) {
            node.collapsed = false;
        }

        let new_id = self.next_id();
        let new_node = Node::new("NEW".to_string(), current);

        if let Some(parent) = self.nodes.get_mut(&current) {
            parent.children.insert(0, new_id);
        }

        self.nodes.insert(new_id, new_node);
        self.active_node = new_id;
        self.modified = true;
        self.refresh_display();
    }

    pub fn delete_node(&mut self, use_clipboard: bool) {
        let current = self.active_node;
        if current == self.root_id || current == 0 {
            return;
        }

        self.push_undo();

        if use_clipboard {
            self.clipboard = Some(self.node_to_text(current));
        }

        // Remove from parent's children list
        let parent_id = self.get_parent_id(current).unwrap_or(0);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.retain(|&c| c != current);
        }

        // Remove all descendants
        let to_remove = self.collect_all_descendants(current);
        for id in &to_remove {
            self.nodes.remove(id);
        }
        self.nodes.remove(&current);

        // Move active node to parent or next sibling
        if let Some(parent) = self.nodes.get(&parent_id) {
            if parent.children.is_empty() {
                self.active_node = parent_id;
            } else {
                self.active_node = parent.children[0];
            }
        } else {
            self.active_node = self.root_id;
        }

        self.modified = true;
        self.refresh_display();
    }

    pub fn delete_children(&mut self, use_clipboard: bool) {
        let current = self.active_node;

        self.push_undo();

        if use_clipboard {
            let mut text = String::new();
            let children: Vec<usize> = self
                .nodes
                .get(&current)
                .map(|n| n.children.clone())
                .unwrap_or_default();
            for &child_id in &children {
                text.push_str(&self.node_to_text(child_id));
            }
            self.clipboard = Some(text);
        }

        // Remove children
        let children: Vec<usize> = self
            .nodes
            .get(&current)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        for child_id in &children {
            let descendants = self.collect_all_descendants(*child_id);
            for id in &descendants {
                self.nodes.remove(id);
            }
            self.nodes.remove(child_id);
        }

        if let Some(node) = self.nodes.get_mut(&current) {
            node.children.clear();
        }

        self.modified = true;
        self.refresh_display();
    }

    fn collect_all_descendants(&self, id: usize) -> Vec<usize> {
        let mut result = Vec::new();
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            if let Some(node) = self.nodes.get(&current) {
                for &child in &node.children {
                    result.push(child);
                    stack.push(child);
                }
            }
        }
        result
    }

    // ─── Clipboard / Paste ─────────────────────────────────────────

    pub fn cut_node(&mut self) {
        self.yank_node();
        self.delete_node(false);
    }

    pub fn cut_children(&mut self) {
        self.yank_children();
        self.delete_children(false);
    }

    pub fn yank_node(&mut self) {
        let current = self.active_node;
        if current == 0 || current == self.root_id {
            return;
        }
        self.clipboard = Some(self.node_to_text(current));
    }

    pub fn yank_children(&mut self) {
        let current = self.active_node;
        let children: Vec<usize> = self
            .nodes
            .get(&current)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        let mut text = String::new();
        for &child_id in &children {
            text.push_str(&self.node_to_text(child_id));
        }
        self.clipboard = Some(text);
    }

    /// Parse tab-indented text (for clipboard only).
    pub(crate) fn from_text(text: &str) -> Self {
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.is_empty() { return Self::new(); }
        let mut nodes: HashMap<usize, Node> = HashMap::new();
        nodes.insert(0, Node { title: String::new(), parent: usize::MAX, children: Vec::new(), collapsed: false, hidden: true, note: String::new() });
        let mut min_indent = usize::MAX;
        for line in &lines { let indent = line.len() - line.trim_start().len(); if indent < min_indent { min_indent = indent; } }
        let mut id_counter: usize = 2;
        let mut level_parent: HashMap<usize, usize> = HashMap::new();
        let mut level_indent: HashMap<usize, usize> = HashMap::new();
        level_parent.insert(1, 0); level_indent.insert(1, 0);
        let mut prev_level = 1; let mut prev_indent = 0;
        for line in &lines {
            let indent = line.len() - line.trim_start().len();
            let adjusted = indent.saturating_sub(min_indent);
            let title = line.trim().to_string().replace("\\n", "\n");
            let level = if adjusted > prev_indent { let l = prev_level + 1; level_indent.insert(l, adjusted); l }
            else if adjusted < prev_indent { let mut found = 1; for (&l, &i) in &level_indent { if i == adjusted && l > found { found = l; } } found }
            else { prev_level };
            if level > prev_level { level_parent.insert(level, id_counter - 1); }
            let parent = *level_parent.get(&level).unwrap_or(&0);
            let node = Node::new(title, parent);
            nodes.insert(id_counter, node);
            if let Some(p) = nodes.get_mut(&parent) { p.children.push(id_counter); }
            prev_indent = adjusted; prev_level = level; id_counter += 1;
        }
        let mut first_level: Vec<usize> = Vec::new();
        for (&id, node) in &nodes { if id >= 2 && node.parent == 0 { first_level.push(id); } }
        let root_id = if first_level.is_empty() { if let Some(n) = nodes.get_mut(&2) { n.parent = 0; } 2 }
        else if first_level.len() == 1 { let rid = first_level[0]; if let Some(n) = nodes.get_mut(&rid) { n.parent = 0; } rid }
        else { let rid = 1; nodes.insert(1, Node { title: "root".to_string(), parent: 0, children: first_level.clone(), collapsed: false, hidden: false, note: String::new() }); for &id in &first_level { if let Some(n) = nodes.get_mut(&id) { n.parent = 1; } } rid };
        if let Some(n) = nodes.get_mut(&0) { n.children = vec![root_id]; }
        let active_node = root_id;
        let mut mm = MindMap { nodes, root_id, active_node, filename: None, modified: false, undo_stack: Vec::new(), redo_stack: Vec::new(), clipboard: None, visible_nodes: Vec::new(), layouts: HashMap::new(), map_width: 0, map_height: 0, canvas: Vec::new(), max_node_width: 40, line_spacing: 1, show_hidden: false, align_levels: false };
        mm.refresh_display();
        mm
    }

    /// Generate tab-indented text representation (for clipboard export).
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        if let Some(root) = self.nodes.get(&self.root_id) {
            if !root.hidden {
                output.push_str(&root.title.replace('\n', "\\n"));
                output.push('\n');
            }
            self.write_text_subtree(self.root_id, 1, &mut output);
        }
        output
    }

    fn write_text_subtree(&self, parent_id: usize, depth: usize, output: &mut String) {
        if let Some(parent) = self.nodes.get(&parent_id) {
            for &child_id in &parent.children {
                if let Some(child) = self.nodes.get(&child_id) {
                    let indent = "\t".repeat(depth);
                    output.push_str(&format!("{}{}\n", indent, child.title.replace('\n', "\\n")));
                    self.write_text_subtree(child_id, depth + 1, output);
                }
            }
        }
    }

    /// Convert a node and its subtree to tab-indented text.
    fn node_to_text(&self, id: usize) -> String {
        let mut output = String::new();
        if let Some(node) = self.nodes.get(&id) {
            output.push_str(&node.title);
            output.push('\n');
            for &child_id in &node.children {
                output.push_str(&self.subtree_to_text(child_id, 1));
            }
        }
        output
    }

    fn subtree_to_text(&self, id: usize, depth: usize) -> String {
        let mut output = String::new();
        if let Some(node) = self.nodes.get(&id) {
            let indent = "\t".repeat(depth);
            output.push_str(&format!("{}{}\n", indent, node.title));
            for &child_id in &node.children {
                output.push_str(&self.subtree_to_text(child_id, depth + 1));
            }
        }
        output
    }

    pub fn paste_as_children(&mut self) {
        let text = match self.clipboard.clone() {
            Some(t) => t,
            None => return,
        };
        if text.trim().is_empty() {
            return;
        }
        self.push_undo();
        let current = self.active_node;
        if let Some(node) = self.nodes.get_mut(&current) {
            node.collapsed = false;
        }
        let ids = self.paste_text(&text, current);
        if !ids.is_empty() {
            self.active_node = ids[0];
        }
        self.modified = true;
        self.refresh_display();
    }

    pub fn paste_as_siblings(&mut self) {
        let current = self.active_node;
        if current == self.root_id {
            self.paste_as_children();
            return;
        }
        let text = match self.clipboard.clone() {
            Some(t) => t,
            None => return,
        };
        if text.trim().is_empty() {
            return;
        }
        self.push_undo();
        let parent_id = self.get_parent_id(current).unwrap_or(0);
        let all_ids = self.paste_text(&text, parent_id);
        if !all_ids.is_empty() {
            // Move all pasted nodes to right after current
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                // Remove all pasted IDs from wherever they were appended
                parent.children.retain(|c| !all_ids.contains(c));
                if let Some(pos) = parent.children.iter().position(|&c| c == current) {
                    for (i, &id) in all_ids.iter().enumerate() {
                        parent.children.insert(pos + 1 + i, id);
                    }
                } else {
                    parent.children.extend(&all_ids);
                }
            }
            self.active_node = all_ids[0];
        }
        self.modified = true;
        self.refresh_display();
    }

    /// Parse clipboard text and transplant all top-level nodes. Returns new IDs.
    fn paste_text(&mut self, text: &str, target_parent: usize) -> Vec<usize> {
        let temp = MindMap::from_text(text);
        let temp_root = temp.root_id;
        let temp_root_children: Vec<usize> = temp.nodes.get(&temp_root)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        let is_virtual = temp_root == 1
            && temp.nodes.get(&1).map(|n| n.title.as_str() == "root" && n.parent == 0).unwrap_or(false);

        let mut result = Vec::new();
        if is_virtual {
            for &old_id in &temp_root_children {
                let new_id = self.transplant_subtree(&temp, old_id, target_parent);
                result.push(new_id);
            }
        } else if temp_root != 0 && temp_root != 1 {
            let new_id = self.transplant_subtree(&temp, temp_root, target_parent);
            result.push(new_id);
        }
        result
    }

    /// Deep-copy a subtree from src MindMap into self.
    fn transplant_subtree(&mut self, src: &MindMap, src_id: usize, new_parent: usize) -> usize {
        let new_id = self.next_id();
        if let Some(src_node) = src.nodes.get(&src_id) {
            let mut new_node = src_node.clone();
            new_node.parent = new_parent;
            new_node.collapsed = false;
            new_node.children.clear();
            self.nodes.insert(new_id, new_node);
            if let Some(parent) = self.nodes.get_mut(&new_parent) {
                parent.children.push(new_id);
            }
            let src_children: Vec<usize> = src_node.children.clone();
            for &child_id in &src_children {
                let child_new = self.transplant_subtree(src, child_id, new_id);
                if let Some(n) = self.nodes.get_mut(&new_id) {
                    n.children.push(child_new);
                }
            }
        }
        new_id
    }

    pub fn append_clipboard_to_title(&mut self) {
        let text = match self.clipboard.clone() {
            Some(t) => t,
            None => return,
        };
        let first_line = text.lines().next().unwrap_or("").trim().to_string();
        if first_line.is_empty() {
            return;
        }
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&self.active_node) {
            node.title.push_str(" ");
            node.title.push_str(&first_line);
        }
        self.modified = true;
    }

    pub fn delete_node_no_clipboard(&mut self) {
        self.delete_node(false);
    }

    // ─── Collapse / Expand ─────────────────────────────────────────

    pub fn toggle_node(&mut self) {
        let current = self.active_node;
        if let Some(node) = self.nodes.get_mut(&current) {
            if node.is_leaf() {
                // Go to parent and toggle
                let parent = node.parent;
                if let Some(p) = self.nodes.get_mut(&parent) {
                    p.collapsed = !p.collapsed;
                }
            } else {
                node.collapsed = !node.collapsed;
            }
        }
        self.refresh_display();
    }

    pub fn collapse_all(&mut self) {
        Self::collapse_level(self, 1);
    }

    pub fn collapse_children(&mut self) {
        let current = self.active_node;
        let children: Vec<usize> = self
            .nodes
            .get(&current)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        for child_id in children {
            if let Some(child) = self.nodes.get_mut(&child_id) {
                child.collapsed = true;
            }
        }
        self.refresh_display();
    }

    pub fn collapse_inner(&mut self) {
        let current = self.active_node;
        let children: Vec<usize> = self
            .nodes
            .get(&current)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        for child_id in children {
            if let Some(child) = self.nodes.get_mut(&child_id) {
                child.collapsed = true;
                self.collapse_subtree(child_id);
            }
        }
        self.refresh_display();
    }

    fn collapse_subtree(&mut self, id: usize) {
        if let Some(node) = self.nodes.get_mut(&id) {
            let children: Vec<usize> = node.children.clone();
            for child_id in children {
                if let Some(child) = self.nodes.get_mut(&child_id) {
                    child.collapsed = true;
                    self.collapse_subtree(child_id);
                }
            }
        }
    }

    pub fn expand_all(&mut self) {
        for node in self.nodes.values_mut() {
            node.collapsed = false;
        }
        self.refresh_display();
    }

    pub fn collapse_level(&mut self, depth: usize) {
        let root = self.root_id;
        self.expand_to_depth(root, 0, depth);
        self.refresh_display();
    }

    fn expand_to_depth(&mut self, id: usize, current_depth: usize, max_depth: usize) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.collapsed = current_depth >= max_depth;
            let children: Vec<usize> = node.children.clone();
            for child_id in children {
                self.expand_to_depth(child_id, current_depth + 1, max_depth);
            }
        }
    }

    pub fn focus(&mut self) {
        let active = self.active_node;

        // Collapse everything
        for node in self.nodes.values_mut() {
            node.collapsed = true;
        }

        // Expand ancestors of active node
        let mut ancestors = Vec::new();
        let mut current = active;
        while let Some(node) = self.nodes.get(&current) {
            ancestors.push(current);
            current = node.parent;
        }

        for &ancestor in &ancestors {
            if let Some(node) = self.nodes.get_mut(&ancestor) {
                node.collapsed = false;
            }
        }

        // Expand descendants of active
        self.expand_descendants(active);

        self.refresh_display();
    }

    fn expand_descendants(&mut self, id: usize) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.collapsed = false;
            let children: Vec<usize> = node.children.clone();
            for child in children {
                self.expand_descendants(child);
            }
        }
    }

    pub fn collapse_other_branches(&mut self) {
        let active = self.active_node;

        // Find the first-level ancestor
        let mut first_level_ancestor = active;
        loop {
            let parent = self.nodes.get(&first_level_ancestor).map(|n| n.parent).unwrap_or(0);
            if parent == self.root_id || parent == 0 {
                break;
            }
            first_level_ancestor = parent;
        }

        // Collapse all first-level nodes except the branch containing active
        let root = self.root_id;
        if let Some(root_node) = self.nodes.get(&root) {
            let children: Vec<usize> = root_node.children.clone();
            for child_id in children {
                if child_id != first_level_ancestor {
                    if let Some(child) = self.nodes.get_mut(&child_id) {
                        child.collapsed = true;
                    }
                } else if let Some(child) = self.nodes.get_mut(&child_id) {
                    child.collapsed = false;
                }
            }
        }

        self.refresh_display();
    }

    pub fn toggle_hide(&mut self) {
        let current = self.active_node;
        if current == self.root_id {
            return;
        }
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&current) {
            node.hidden = !node.hidden;
            if node.hidden {
                // Move to parent
                self.active_node = node.parent;
            }
        }
        self.modified = true;
        self.refresh_display();
    }

    // ─── Move Nodes ────────────────────────────────────────────────

    pub fn move_node_up(&mut self) {
        let current = self.active_node;
        if current == self.root_id {
            return;
        }
        self.push_undo();
        let parent_id = self.get_parent_id(current).unwrap_or(0);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if let Some(pos) = parent.children.iter().position(|&c| c == current) {
                if pos > 0 {
                    parent.children.swap(pos, pos - 1);
                }
            }
        }
        self.modified = true;
        self.refresh_display();
    }

    pub fn move_node_down(&mut self) {
        let current = self.active_node;
        if current == self.root_id {
            return;
        }
        self.push_undo();
        let parent_id = self.get_parent_id(current).unwrap_or(0);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if let Some(pos) = parent.children.iter().position(|&c| c == current) {
                if pos + 1 < parent.children.len() {
                    parent.children.swap(pos, pos + 1);
                }
            }
        }
        self.modified = true;
        self.refresh_display();
    }

    pub fn sort_siblings(&mut self) {
        let current = self.active_node;
        if current == self.root_id {
            return;
        }
        self.push_undo();
        let parent_id = self.get_parent_id(current).unwrap_or(0);
        // Collect titles first to avoid borrow conflict
        let mut child_titles: Vec<(usize, String)> = Vec::new();
        if let Some(parent) = self.nodes.get(&parent_id) {
            for &child_id in &parent.children {
                if let Some(child) = self.nodes.get(&child_id) {
                    child_titles.push((child_id, child.title.clone()));
                }
            }
        }
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.sort_by(|a, b| {
                let title_a = child_titles.iter().find(|(id, _)| id == a).map(|(_, t)| t.as_str()).unwrap_or("");
                let title_b = child_titles.iter().find(|(id, _)| id == b).map(|(_, t)| t.as_str()).unwrap_or("");
                title_a.cmp(title_b)
            });
        }
        self.modified = true;
        self.refresh_display();
    }

    // ─── Symbols / Marks ───────────────────────────────────────────

    pub fn toggle_symbol(&mut self, symbol1: &str, symbol2: &str) {
        let current = self.active_node;
        self.push_undo();

        if let Some(node) = self.nodes.get_mut(&current) {
            let len1 = symbol1.len() + 1;
            let len2 = symbol2.len() + 1;

            if node.title.starts_with(&format!("{} ", symbol1)) {
                node.title = format!(
                    "{} {}",
                    symbol2,
                    &node.title[len1..]
                );
            } else if node.title.starts_with(&format!("{} ", symbol2)) {
                node.title = node.title[len2..].to_string();
            } else {
                node.title = format!("{} {}", symbol1, node.title);
            }
        }

        self.modified = true;
    }

    pub fn toggle_numbers(&mut self) {
        let current = self.active_node;
        if current == self.root_id {
            return;
        }
        self.push_undo();

        let parent_id = self.get_parent_id(current).unwrap_or(0);
        let siblings: Vec<usize> = self
            .nodes
            .get(&parent_id)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        // Check if already numbered
        let has_numbers = siblings.iter().any(|&sid| {
            self.nodes
                .get(&sid)
                .map(|n| n.title.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
                .unwrap_or(false)
        });

        for (i, &sid) in siblings.iter().enumerate() {
            if let Some(node) = self.nodes.get_mut(&sid) {
                if has_numbers {
                    // Remove numbers
                    if let Some(pos) = node.title.find(". ") {
                        if node.title[..pos].chars().all(|c| c.is_ascii_digit()) {
                            node.title = node.title[pos + 2..].to_string();
                        }
                    }
                } else {
                    // Add numbers
                    let pad = if siblings.len() > 9 { 2 } else { 1 };
                    node.title = format!("{:0width$}. {}", i + 1, node.title, width = pad);
                }
            }
        }

        self.modified = true;
    }

    // ─── Ranking ───────────────────────────────────────────────────

    pub fn add_rank(&mut self, pos: i32, neg: i32) {
        let current = self.active_node;
        self.push_undo();

        if let Some(node) = self.nodes.get_mut(&current) {
            let mut new_title = String::new();

            // Remove existing rank markers
            let cleaned = Self::strip_ranks(&node.title);

            for _ in 0..pos.max(0) {
                new_title.push('+');
            }
            for _ in 0..neg.max(0) {
                new_title.push('-');
            }
            if !new_title.is_empty() {
                new_title.push(' ');
            }
            new_title.push_str(&cleaned);

            node.title = new_title;
        }

        self.modified = true;
    }

    fn strip_ranks(title: &str) -> String {
        let trimmed = title.trim_start_matches(|c: char| c == '+' || c == '-' || c == ' ');
        trimmed.to_string()
    }

    /// Calculate current rank from title and modify it
    pub fn modify_positive_rank(&mut self, delta: i32) {
        let current = self.active_node;
        self.push_undo();

        if let Some(node) = self.nodes.get_mut(&current) {
            let (pos, neg) = Self::parse_ranks(&node.title);
            let cleaned = Self::strip_ranks(&node.title);
            let new_pos = (pos + delta).max(0);

            let mut new_title = String::new();
            for _ in 0..new_pos {
                new_title.push('+');
            }
            for _ in 0..neg {
                new_title.push('-');
            }
            if !new_title.is_empty() {
                new_title.push(' ');
            }
            new_title.push_str(&cleaned);
            node.title = new_title;
        }

        self.modified = true;
    }

    pub fn modify_negative_rank(&mut self, delta: i32) {
        let current = self.active_node;
        self.push_undo();

        if let Some(node) = self.nodes.get_mut(&current) {
            let (pos, neg) = Self::parse_ranks(&node.title);
            let cleaned = Self::strip_ranks(&node.title);
            let new_neg = (neg + delta).max(0);

            let mut new_title = String::new();
            for _ in 0..pos {
                new_title.push('+');
            }
            for _ in 0..new_neg {
                new_title.push('-');
            }
            if !new_title.is_empty() {
                new_title.push(' ');
            }
            new_title.push_str(&cleaned);
            node.title = new_title;
        }

        self.modified = true;
    }

    fn parse_ranks(title: &str) -> (i32, i32) {
        let mut pos = 0i32;
        let mut neg = 0i32;
        let mut seen_neg = false;
        for c in title.chars() {
            match c {
                '+' if !seen_neg => pos += 1,
                '-' => {
                    seen_neg = true;
                    neg += 1;
                }
                ' ' => continue,
                _ => break,
            }
        }
        (pos, neg)
    }

    // ─── Stars ─────────────────────────────────────────────────────

    pub fn add_star(&mut self) {
        let current = self.active_node;
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&current) {
            if !node.title.starts_with('★') && !node.title.starts_with('☆') {
                node.title = format!("★ {}", node.title);
            } else {
                let star_count = node.title.chars().take_while(|&c| c == '★' || c == '☆').count();
                let rest: String = node.title.chars().skip(star_count).collect();
                let rest = rest.trim_start();
                node.title = format!("{} {}", "★".repeat(star_count + 1), rest);
            }
        }
        self.modified = true;
    }

    pub fn remove_star(&mut self) {
        let current = self.active_node;
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&current) {
            let star_count = node.title.chars().take_while(|&c| c == '★' || c == '☆').count();
            if star_count > 0 {
                let rest: String = node.title.chars().skip(star_count).collect();
                let rest = rest.trim_start();
                if star_count > 1 {
                    node.title = format!("{} {}", "★".repeat(star_count - 1), rest);
                } else {
                    node.title = rest.to_string();
                }
            }
        }
        self.modified = true;
    }

    // ─── Undo / Redo ───────────────────────────────────────────────

    fn push_undo(&mut self) {
        let snapshot = UndoSnapshot {
            nodes: self.nodes.clone(),
            root_id: self.root_id,
            active_node: self.active_node,
        };
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();

        if self.undo_stack.len() > Self::MAX_UNDO {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            // Save current state to redo
            let redo_snapshot = UndoSnapshot {
                nodes: self.nodes.clone(),
                root_id: self.root_id,
                active_node: self.active_node,
            };
            self.redo_stack.push(redo_snapshot);

            // Restore
            self.nodes = snapshot.nodes;
            self.root_id = snapshot.root_id;
            self.active_node = snapshot.active_node;
            self.modified = true;
            self.refresh_display();
        }
    }

    pub fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop() {
            let undo_snapshot = UndoSnapshot {
                nodes: self.nodes.clone(),
                root_id: self.root_id,
                active_node: self.active_node,
            };
            self.undo_stack.push(undo_snapshot);

            self.nodes = snapshot.nodes;
            self.root_id = snapshot.root_id;
            self.active_node = snapshot.active_node;
            self.modified = true;
            self.refresh_display();
        }
    }

    // ─── Export ────────────────────────────────────────────────────

    /// Export the mind map as an HTML file.
    pub fn export_html(&self) -> String {
        let title = self.nodes.get(&self.root_id)
            .map(|n| n.title.as_str())
            .unwrap_or("Mind Map");

        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"UTF-8\">\n");
        html.push_str(&format!("<title>{}</title>\n", title));
        html.push_str("<style>\n");
        html.push_str("body { font-family: system-ui, sans-serif; background: #1a1a2e; color: #e0e0e0; padding: 2em; }\n");
        html.push_str("ul { list-style: none; padding-left: 1.5em; border-left: 2px solid #444; }\n");
        html.push_str("li { position: relative; padding: 0.2em 0; }\n");
        html.push_str("li::before { content: ''; position: absolute; left: -1.5em; top: 0.8em; width: 1.2em; border-top: 2px solid #444; }\n");
        html.push_str("details > summary { cursor: pointer; color: #f0a040; }\n");
        html.push_str(".collapsed-marker { color: #888; }\n");
        html.push_str("</style></head><body>\n");
        html.push_str(&format!("<h1>{}</h1>\n", title));
        html.push_str("<ul>\n");
        self.write_html_subtree(self.root_id, &mut html);
        html.push_str("</ul>\n");
        html.push_str("</body></html>\n");
        html
    }

    fn write_html_subtree(&self, parent_id: usize, html: &mut String) {
        if let Some(parent) = self.nodes.get(&parent_id) {
            for &child_id in &parent.children {
                if let Some(child) = self.nodes.get(&child_id) {
                    if child.hidden {
                        continue;
                    }
                    let title = html_escape(&child.title);
                    if !child.is_leaf() {
                        html.push_str(&format!("<li><details{}><summary>{}</summary>\n",
                            if child.collapsed { "" } else { " open" },
                            title
                        ));
                        html.push_str("<ul>\n");
                        self.write_html_subtree(child_id, html);
                        html.push_str("</ul></details></li>\n");
                    } else {
                        html.push_str(&format!("<li>{}</li>\n", title));
                    }
                }
            }
        }
    }

    /// Rebuild display state: visible list, layout, and canvas.
    pub fn refresh_display(&mut self) {
        self.visible_nodes = self.collect_visible(self.root_id, &mut Vec::new());
        self.calculate_layout(self.max_node_width, self.line_spacing);
        self.build_canvas();
    }

    fn collect_visible(&self, id: usize, ancestors_collapsed: &mut Vec<bool>) -> Vec<usize> {
        let mut result = Vec::new();
        if let Some(node) = self.nodes.get(&id) {
            // Include node if it's not hidden OR we're showing hidden
            let is_visible = !node.hidden || self.show_hidden;
            if is_visible {
                result.push(id);
            }
            let is_hidden_by_ancestor = ancestors_collapsed.iter().any(|&c| c);
            if !is_hidden_by_ancestor && !node.collapsed && is_visible {
                ancestors_collapsed.push(node.collapsed);
                for &child_id in &node.children {
                    result.extend(self.collect_visible(child_id, ancestors_collapsed));
                }
                ancestors_collapsed.pop();
            }
        }
        result
    }

    // ─── Layout Calculation ───────────────────────────────────────

    /// Calculate layout for all visible nodes in one post-order pass.
    pub fn calculate_layout(&mut self, max_width: usize, line_spacing: usize) {
        self.layouts.clear();
        if self.visible_nodes.is_empty() {
            self.visible_nodes = self.collect_visible(self.root_id, &mut Vec::new());
        }

        let root = self.root_id;
        let spacing = line_spacing.max(0);
        let connector_gap = 6; // space between parent right edge and child

        // Single post-order traversal
        let map_h = self.layout_pass(root, 0, 0, 0, max_width, spacing, connector_gap);

        // Align levels if enabled
        if self.align_levels {
            let mut max_x_per_depth: HashMap<usize, usize> = HashMap::new();
            for layout in self.layouts.values() {
                let cur = max_x_per_depth.get(&layout.depth).copied().unwrap_or(0);
                max_x_per_depth.insert(layout.depth, cur.max(layout.x));
            }
            for layout in self.layouts.values_mut() {
                if let Some(&aligned_x) = max_x_per_depth.get(&layout.depth) {
                    layout.x = aligned_x;
                }
            }
        }

        self.map_width = self.layouts.values()
            .map(|l| l.x + l.w + 4)
            .max()
            .unwrap_or(1);
        self.map_height = map_h.max(1);
    }

    /// Post-order layout: returns total height of subtree rooted at id.
    fn layout_pass(&mut self, id: usize, depth: usize, base_y: usize, parent_rx: usize, max_w: usize, spacing: usize, gap: usize) -> usize {
        let node = match self.nodes.get(&id) {
            Some(n) => n.clone(),
            None => return 0,
        };

        // ── Compute text dimensions ──
        let is_at_end = node.is_leaf() || node.collapsed || node.children.iter()
            .all(|c| self.nodes.get(c).map(|n| n.hidden && !self.show_hidden).unwrap_or(true));

        let width_limit = if is_at_end { (max_w as f64 * 1.3) as usize } else { max_w };
        let display_lines = Self::lines_for_display(&node.title, width_limit);
        let num_lines = display_lines.len().max(1);
        let _display_w = display_lines.iter()
            .map(|l| unicode_width::UnicodeWidthStr::width(l.as_str()))
            .max()
            .unwrap_or(2);
        // w = number of chars in the longest line (canvas positions), not display width
        let w = display_lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(1);

        // ── Compute X: root at 0, children at parent's right edge + gap ──
        let x = if depth == 0 { 0 } else { parent_rx + gap };

        // ── Own height ──
        let own_h = num_lines + spacing;

        // ── Recurse to children (start after parent's own lines) ──
        let my_rx = x + w;
        let children_base = base_y + own_h;
        let mut children_heights = 0usize;
        if !node.collapsed {
            let show = self.show_hidden;
            for &child_id in &node.children {
                if self.nodes.get(&child_id).map(|n| !n.hidden || show).unwrap_or(false) {
                    let ch = self.layout_pass(child_id, depth + 1, children_base + children_heights, my_rx, max_w, spacing, gap);
                    children_heights += ch;
                }
            }
        }

        // ── Post-order: total height = own + children ──
        let total_h = own_h + children_heights;
        let yo = 0;

        self.layouts.insert(id, NodeLayout {
            id, x, y: base_y + yo, w, h: total_h, depth, yo, lines: num_lines,
        });

        total_h
    }

    /// Export current canvas as ASCII art text.
    pub fn export_ascii(&self) -> String {
        let mut out = String::new();
        for row in &self.canvas {
            let line: String = row.iter().collect();
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }

    /// Split text into display lines: first by \n, then word-wrap each.
    fn lines_for_display(text: &str, max_w: usize) -> Vec<String> {
        let mut result = Vec::new();
        for raw_line in text.split('\n') {
            if raw_line.is_empty() {
                result.push(String::new());
                continue;
            }
            let rl_w = unicode_width::UnicodeWidthStr::width(raw_line);
            if rl_w <= max_w {
                result.push(raw_line.to_string());
            } else {
                result.extend(Self::wrap_text(raw_line, max_w));
            }
        }
        if result.is_empty() {
            result.push(String::new());
        }
        result
    }

    fn wrap_text(text: &str, max_w: usize) -> Vec<String> {
        let mut result = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return vec![String::new()];
        }
        let mut current = String::new();
        for word in words {
            let test = if current.is_empty() { word.to_string() } else { format!("{} {}", current, word) };
            if unicode_width::UnicodeWidthStr::width(test.as_str()) > max_w && !current.is_empty() {
                result.push(current);
                current = word.to_string();
            } else {
                current = test;
            }
        }
        if !current.is_empty() {
            result.push(current);
        }
        if result.is_empty() {
            result.push(String::new());
        }
        result
    }

    fn draw_text_at(&mut self, x: usize, y: usize, text: &str) {
        if y >= self.canvas.len() {
            return;
        }
        let chars: Vec<char> = text.chars().collect();
        let max_w = self.canvas[y].len().saturating_sub(x);
        for (i, &ch) in chars.iter().enumerate().take(max_w) {
            self.canvas[y][x + i] = ch;
        }
    }

    // ─── Canvas Rendering ──────────────────────────────────────────

    /// Build the full canvas text buffer for the mind map.
    pub fn build_canvas(&mut self) {
        let h = self.map_height.max(1);
        let w = self.map_width.max(1);

        self.canvas = vec![vec![' '; w]; h];

        // Draw connections first (behind text)
        self.draw_connections(self.root_id);

        // Draw node text (multi-line wrapped)
        let node_data: Vec<(usize, String, NodeLayout)> = self
            .layouts
            .iter()
            .filter_map(|(&id, layout)| {
                self.nodes.get(&id).map(|node| (id, node.title.clone(), layout.clone()))
            })
            .collect();

        for (_id, title, layout) in node_data {
            // Split by newlines, then word-wrap each
            let display_lines = Self::lines_for_display(&title, layout.w.saturating_sub(1));
            for (li, line) in display_lines.iter().enumerate() {
                let cy = layout.y + li;
                if cy < self.canvas.len() {
                    self.draw_text_at(layout.x, cy, line);
                }
            }
        }
    }

    fn set_cell(&mut self, x: usize, y: usize, ch: char) {
        if y < self.canvas.len() && x < self.canvas[y].len() {
            self.canvas[y][x] = ch;
        }
    }

    fn draw_connections(&mut self, id: usize) {
        let node = match self.nodes.get(&id) {
            Some(n) => n.clone(),
            None => return,
        };

        let visible_children: Vec<usize> = node.children.iter()
            .filter(|c| self.layouts.contains_key(c))
            .copied()
            .collect();

        // Draw collapse marker if collapsed with hidden children
        if node.collapsed && !node.children.is_empty() {
            if let Some(layout) = self.layouts.get(&id) {
                let mx = layout.x + layout.w;
                let my = layout.y;
                for (i, ch) in "[+]".chars().enumerate() {
                    self.set_cell(mx + i, my, ch);
                }
            }
            return;
        }

        if visible_children.is_empty() {
            return;
        }

        let my = self.layouts.get(&id).cloned();
        if my.is_none() {
            return;
        }
        let my = my.unwrap();

        // Connector constants (like h-m-m)
        let conn_right_len: usize = 3;

        // All children at same level share the same x position (from layout)
        let first_child = visible_children[0];
        let child_x = self.layouts.get(&first_child).map(|l| l.x).unwrap_or(my.x + my.w + 10);

        // Vertical bar x position (just before the children)
        let bar_x = child_x.saturating_sub(conn_right_len);

        // Parent's right edge to bar
        let parent_right = my.x + my.w;
        let parent_mid_y = my.y + my.lines / 2;

        if visible_children.len() == 1 {
            let child = self.layouts.get(&first_child).cloned();
            if let Some(cl) = child {
                let child_mid_y = cl.y + cl.lines / 2;

                if parent_mid_y == child_mid_y {
                    // Same row: straight horizontal line, no bar needed
                    for x in parent_right..child_x.min(self.map_width) {
                        self.set_cell(x, parent_mid_y, '─');
                    }
                } else {
                    // Different rows: parent horizontal → bar → child horizontal
                    for x in parent_right..bar_x.min(self.map_width) {
                        self.set_cell(x, parent_mid_y, '─');
                    }
                    let (top, bottom) = if parent_mid_y < child_mid_y {
                        (parent_mid_y, child_mid_y)
                    } else {
                        (child_mid_y, parent_mid_y)
                    };
                    for y in top..=bottom {
                        self.set_cell(bar_x, y, '│');
                    }
                    for x in (bar_x + 1)..child_x.min(self.map_width) {
                        self.set_cell(x, child_mid_y, '─');
                    }
                    if child_mid_y < parent_mid_y {
                        self.set_cell(bar_x, child_mid_y, '╭');
                        self.set_cell(bar_x, parent_mid_y, '╯');
                    } else {
                        self.set_cell(bar_x, child_mid_y, '╰');
                        self.set_cell(bar_x, parent_mid_y, '╮');
                    }
                }
            }
            self.draw_connections(first_child);
            return;
        }

        // Multiple children: parent horizontal + vertical bar + child horizontals
        for x in parent_right..bar_x.min(self.map_width) {
            self.set_cell(x, parent_mid_y, '─');
        }

        // Multiple children
        let last_child = visible_children[visible_children.len() - 1];
        let first_cl = self.layouts.get(&first_child).cloned();
        let last_cl = self.layouts.get(&last_child).cloned();

        if let (Some(fc), Some(lc)) = (first_cl, last_cl) {
            let first_y = fc.y + fc.lines / 2;
            let last_y = lc.y + lc.lines / 2;
            let bar_top = parent_mid_y.min(first_y);
            let bar_bottom = parent_mid_y.max(last_y);

            for y in bar_top..=bar_bottom {
                let is_first = y == first_y;
                let is_last = y == last_y;
                let is_parent = y == parent_mid_y;
                let ch = match (is_parent, is_first, is_last) {
                    (true, true, true) => '┼',   // parent=first=last (one child at same y)
                    (true, true, false) => '┬',  // parent at first child
                    (true, false, true) => '┴',  // parent at last child
                    (false, true, false) => '├', // first child (below parent)
                    (false, false, true) => '╰', // last child
                    (true, false, false) => '┤', // parent between children
                    _ => '│',
                };
                self.set_cell(bar_x, y, ch);
            }

            // Draw individual connectors to each child
            for &child_id in &visible_children {
                if let Some(cl) = self.layouts.get(&child_id) {
                    let child_mid_y = cl.y + cl.lines / 2;
                    for x in (bar_x + 1)..cl.x.min(self.map_width) {
                        self.set_cell(x, child_mid_y, '─');
                    }
                    // Recurse
                    self.draw_connections(child_id);
                }
            }
        } else {
            for &child_id in &visible_children {
                self.draw_connections(child_id);
            }
        }
    }
}

/// Simple HTML entity escaping
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══ Parsing ══════════════════════════════════════════════

    #[test]
    fn test_parse_flat() {
        let input = "root\n\tA\n\tB\n\tC";
        let mm = MindMap::from_text(input);
        let output = mm.to_text();
        let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 4, "Should have 4 lines (root + 3 children)");
    }

    #[test]
    fn test_parse_nested() {
        let input = "root\n\tA\n\t\tA1\n\t\tA2\n\tB";
        let mm = MindMap::from_text(input);
        let s = mm.to_text();
        assert!(s.contains("A"), "Should contain A");
        assert!(s.contains("A1"), "Should contain A1");
        assert!(s.contains("A2"), "Should contain A2");
        assert!(s.contains("B"), "Should contain B");
    }

    #[test]
    fn test_parse_multi_root_creates_virtual_root() {
        // Two top-level items → virtual root created
        let input = "First\nSecond";
        let mm = MindMap::from_text(input);
        let root = &mm.nodes[&mm.root_id];
        assert_eq!(root.title, "root", "Should create virtual root");
        assert_eq!(root.children.len(), 2, "Should have 2 children under root");
    }

    #[test]
    fn test_parse_single_root_no_virtual() {
        let input = "OnlyRoot\n\tChild1\n\tChild2";
        let mm = MindMap::from_text(input);
        let root = &mm.nodes[&mm.root_id];
        assert_eq!(root.title, "OnlyRoot", "Single root should keep its title");
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn test_parse_deep_nesting() {
        let input = "L0\n\tL1\n\t\tL2\n\t\t\tL3\n\t\t\t\tL4";
        let mm = MindMap::from_text(input);
        let s = mm.to_text();
        assert!(s.contains("L0"));
        assert!(s.contains("L4"));
    }

    #[test]
    fn test_parse_empty() {
        let mm = MindMap::from_text("");
        assert_eq!(mm.root_id, 1);
        assert!(mm.nodes.contains_key(&1));
    }

    #[test]
    fn test_roundtrip_save_load() {
        let input = "root\n\tA\n\tB\n\t\tB1\n\tC";
        let mm = MindMap::from_text(input);
        let saved = mm.to_text();
        let mm2 = MindMap::from_text(&saved);
        let saved2 = mm2.to_text();
        assert_eq!(saved, saved2, "Roundtrip should be stable");
    }

    #[test]
    fn test_parse_spaces_as_indent() {
        // Spaces instead of tabs
        let input = "root\n  Child A\n    Grandchild\n  Child B";
        let mm = MindMap::from_text(input);
        let s = mm.to_text();
        assert!(s.contains("Child A"));
        assert!(s.contains("Grandchild"));
        assert!(s.contains("Child B"));
    }

    // ═══ CRUD ══════════════════════════════════════════════════

    #[test]
    fn test_new_map() {
        let mm = MindMap::new();
        assert_eq!(mm.root_id, 1);
        assert!(mm.nodes.contains_key(&mm.root_id));
        assert_eq!(mm.nodes[&mm.root_id].title, "root");
    }

    #[test]
    fn test_insert_sibling() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        // Activate A
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.insert_sibling();
        // Should have 3 children now
        let root = &mm.nodes[&mm.root_id];
        assert_eq!(root.children.len(), 3);
    }

    #[test]
    fn test_insert_child() {
        let mut mm = MindMap::from_text("root\n\tParent");
        let pid = find_node_by_title(&mm, "Parent");
        mm.active_node = pid;
        mm.insert_child();
        let parent = &mm.nodes[&pid];
        assert_eq!(parent.children.len(), 1);
        assert!(!parent.collapsed, "Parent should be uncollapsed after insert");
    }

    #[test]
    fn test_edit_node() {
        let mut mm = MindMap::new();
        mm.insert_child();
        mm.edit_node("Test Child".to_string());
        let child_id = mm.active_node;
        assert_eq!(mm.nodes[&child_id].title, "Test Child");
    }

    #[test]
    fn test_delete_node() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.delete_node(false);
        let root = &mm.nodes[&mm.root_id];
        assert_eq!(root.children.len(), 1, "Should have 1 child left");
        let remaining = &mm.nodes[&root.children[0]];
        assert_eq!(remaining.title, "B");
    }

    #[test]
    fn test_delete_children() {
        let mut mm = MindMap::from_text("root\n\tA\n\t\tA1\n\t\tA2");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.delete_children(false);
        let a = &mm.nodes[&a_id];
        assert!(a.children.is_empty(), "All children should be deleted");
    }

    #[test]
    fn test_delete_node_with_clipboard() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.cut_node(); // = delete_node(true)
        assert!(mm.clipboard.is_some(), "Clipboard should have content");
        let root = &mm.nodes[&mm.root_id];
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn test_cannot_delete_root() {
        let mut mm = MindMap::new();
        let count_before = mm.nodes.len();
        mm.active_node = mm.root_id;
        mm.delete_node(false);
        assert_eq!(mm.nodes.len(), count_before, "Root should not be deleted");
    }

    // ═══ Clipboard ═════════════════════════════════════════════

    #[test]
    fn test_yank_node() {
        let mut mm = MindMap::from_text("root\n\tA\n\t\tA1");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.yank_node();
        assert!(mm.clipboard.is_some());
        let clip = mm.clipboard.as_ref().unwrap();
        assert!(!clip.is_empty());
    }

    #[test]
    fn test_yank_children() {
        let mut mm = MindMap::from_text("root\n\tA\n\t\tA1\n\t\tA2");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.yank_children();
        assert!(mm.clipboard.is_some());
    }

    #[test]
    fn test_paste_as_children() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        // Yank "A" including its subtree
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.yank_node();
        // Paste under "B"
        let b_id = find_node_by_title(&mm, "B");
        mm.active_node = b_id;
        mm.paste_as_children();
        let b = &mm.nodes[&b_id];
        assert!(!b.children.is_empty(), "B should have pasted children");
        assert!(!b.collapsed, "B should be uncollapsed");
    }

    #[test]
    fn test_paste_as_siblings() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.yank_node();
        let b_id = find_node_by_title(&mm, "B");
        mm.active_node = b_id;
        mm.paste_as_siblings();
        let root = &mm.nodes[&mm.root_id];
        assert!(root.children.len() >= 2, "Should have at least 2 children");
    }

    #[test]
    fn test_append_clipboard_to_title() {
        let mut mm = MindMap::from_text("root\n\tHello\n\tWorld");
        let world_id = find_node_by_title(&mm, "World");
        mm.active_node = world_id;
        mm.yank_node();
        let hello_id = find_node_by_title(&mm, "Hello");
        mm.active_node = hello_id;
        mm.append_clipboard_to_title();
        assert!(mm.nodes[&hello_id].title.contains("World"));
    }

    #[test]
    fn test_delete_node_no_clipboard() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.delete_node_no_clipboard();
        assert!(mm.clipboard.is_none(), "Clipboard should be empty");
        let root = &mm.nodes[&mm.root_id];
        assert_eq!(root.children.len(), 1);
    }

    // ═══ Collapse / Expand ════════════════════════════════════

    #[test]
    fn test_toggle_collapse() {
        let input = "root\n\tA\n\t\tA1\n\t\tA2";
        let mut mm = MindMap::from_text(input);
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.toggle_node();
        assert!(mm.nodes[&a_id].collapsed, "A should be collapsed");
        mm.toggle_node();
        assert!(!mm.nodes[&a_id].collapsed, "A should be expanded");
    }

    #[test]
    fn test_toggle_collapse_leaf_goes_to_parent() {
        let input = "root\n\tA\n\t\tA1";
        let mut mm = MindMap::from_text(input);
        let a1_id = find_node_by_title(&mm, "A1");
        mm.active_node = a1_id;
        mm.toggle_node(); // Leaf → should toggle parent
        let a_id = find_node_by_title(&mm, "A");
        assert!(mm.nodes[&a_id].collapsed, "Parent should be collapsed");
    }

    #[test]
    fn test_expand_all() {
        let input = "root\n\tA\n\t\tA1\n\tB";
        let mut mm = MindMap::from_text(input);
        // Collapse everything first
        mm.collapse_all();
        mm.expand_all();
        for node in mm.nodes.values() {
            assert!(!node.collapsed, "All nodes should be expanded");
        }
    }

    #[test]
    fn test_collapse_level() {
        let input = "L0\n\tL1\n\t\tL2\n\t\t\tL3";
        let mut mm = MindMap::from_text(input);
        mm.collapse_level(2); // Only show L0 and L1
        // L2 should have collapsed = true
        let l2_id = find_node_by_title(&mm, "L2");
        assert!(mm.nodes[&l2_id].collapsed, "L2 should be collapsed at level 2");
    }

    #[test]
    fn test_focus() {
        let input = "root\n\tBranch1\n\t\tB1a\n\tBranch2\n\t\tB2a\n\t\tB2b";
        let mut mm = MindMap::from_text(input);
        mm.collapse_all();
        let b2a_id = find_node_by_title(&mm, "B2a");
        mm.active_node = b2a_id;
        mm.focus();
        // Branch2 should be expanded (ancestor of active)
        let b2_id = find_node_by_title(&mm, "Branch2");
        assert!(!mm.nodes[&b2_id].collapsed, "Branch2 should be expanded by focus");
    }

    #[test]
    fn test_collapse_other_branches() {
        let input = "root\n\tA\n\tB\n\tC";
        let mut mm = MindMap::from_text(input);
        let b_id = find_node_by_title(&mm, "B");
        mm.active_node = b_id;
        mm.collapse_other_branches();
        let a_id = find_node_by_title(&mm, "A");
        let c_id = find_node_by_title(&mm, "C");
        assert!(mm.nodes[&a_id].collapsed, "A should be collapsed");
        assert!(mm.nodes[&c_id].collapsed, "C should be collapsed");
        assert!(!mm.nodes[&b_id].collapsed, "B (active branch) should not be collapsed");
    }

    #[test]
    fn test_collapse_children() {
        let input = "root\n\tA\n\t\tA1\n\t\tA2";
        let mut mm = MindMap::from_text(input);
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.collapse_children();
        let a1_id = find_node_by_title(&mm, "A1");
        let a2_id = find_node_by_title(&mm, "A2");
        assert!(mm.nodes[&a1_id].collapsed, "A1 should be collapsed");
        assert!(mm.nodes[&a2_id].collapsed, "A2 should be collapsed");
    }

    #[test]
    fn test_collapse_inner() {
        let input = "root\n\tA\n\t\tA1\n\t\t\tA1a\n\t\tA2";
        let mut mm = MindMap::from_text(input);
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.collapse_inner();
        let a1_id = find_node_by_title(&mm, "A1");
        assert!(mm.nodes[&a1_id].collapsed, "A1 should be collapsed");
        let a1a_id = find_node_by_title(&mm, "A1a");
        assert!(mm.nodes[&a1a_id].collapsed, "A1a should also be collapsed (deep collapse)");
    }

    // ═══ Navigation ════════════════════════════════════════════

    #[test]
    fn test_nav_up_down() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
        let b_id = find_node_by_title(&mm, "B");
        mm.active_node = b_id;
        mm.go_up();
        assert_eq!(mm.nodes[&mm.active_node].title, "A");
        mm.go_down();
        assert_eq!(mm.nodes[&mm.active_node].title, "B");
        mm.go_down();
        assert_eq!(mm.nodes[&mm.active_node].title, "C");
    }

    #[test]
    fn test_nav_left_right() {
        let mut mm = MindMap::from_text("root\n\tA\n\t\tA1");
        let a1_id = find_node_by_title(&mm, "A1");
        mm.active_node = a1_id;
        mm.go_left(); // Go to parent
        assert_eq!(mm.nodes[&mm.active_node].title, "A");
        mm.go_right(); // Go to first child
        assert_eq!(mm.nodes[&mm.active_node].title, "A1");
    }

    #[test]
    fn test_nav_top_bottom() {
        let mut mm = MindMap::from_text("root\n\tFirst\n\tMiddle\n\tLast");
        mm.go_to_bottom();
        assert_eq!(mm.nodes[&mm.active_node].title, "Last");
        mm.go_to_top();
        assert_eq!(mm.nodes[&mm.active_node].title, "root");
    }

    #[test]
    fn test_nav_root() {
        let mut mm = MindMap::from_text("root\n\tDeep\n\t\tDeeper");
        let deeper_id = find_node_by_title(&mm, "Deeper");
        mm.active_node = deeper_id;
        mm.go_to_root();
        assert_eq!(mm.active_node, mm.root_id);
    }

    #[test]
    fn test_nav_up_wraps() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.go_up(); // Should wrap to last sibling
        assert_eq!(mm.nodes[&mm.active_node].title, "C");
    }

    #[test]
    fn test_nav_down_wraps() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
        let c_id = find_node_by_title(&mm, "C");
        mm.active_node = c_id;
        mm.go_down(); // Should wrap to first sibling
        assert_eq!(mm.nodes[&mm.active_node].title, "A");
    }

    // ═══ Move / Sort ═══════════════════════════════════════════

    #[test]
    fn test_move_node_up() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
        let b_id = find_node_by_title(&mm, "B");
        mm.active_node = b_id;
        mm.move_node_up();
        let root = &mm.nodes[&mm.root_id];
        assert_eq!(mm.nodes[&root.children[0]].title, "B", "B should now be first");
        assert_eq!(mm.nodes[&root.children[1]].title, "A", "A should now be second");
    }

    #[test]
    fn test_move_node_down() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
        let b_id = find_node_by_title(&mm, "B");
        mm.active_node = b_id;
        mm.move_node_down();
        let root = &mm.nodes[&mm.root_id];
        assert_eq!(mm.nodes[&root.children[2]].title, "B", "B should now be last");
    }

    #[test]
    fn test_sort_siblings() {
        let mut mm = MindMap::from_text("root\n\tC\n\tA\n\tB");
        let c_id = find_node_by_title(&mm, "C");
        mm.active_node = c_id;
        mm.sort_siblings();
        let root = &mm.nodes[&mm.root_id];
        let titles: Vec<&str> = root.children.iter()
            .map(|id| mm.nodes[id].title.as_str()).collect();
        assert_eq!(titles, vec!["A", "B", "C"], "Should be sorted alphabetically");
    }

    #[test]
    fn test_cannot_move_root() {
        let mut mm = MindMap::from_text("root\n\tA");
        mm.active_node = mm.root_id;
        let children_before = mm.nodes[&mm.root_id].children.clone();
        mm.move_node_up();
        mm.move_node_down();
        assert_eq!(mm.nodes[&mm.root_id].children, children_before, "Root should not move");
    }

    // ═══ Marks ═════════════════════════════════════════════════

    #[test]
    fn test_toggle_symbol() {
        let mut mm = MindMap::from_text("root\n\tItem");
        let item_id = find_node_by_title(&mm, "Item");
        mm.active_node = item_id;
        mm.toggle_symbol("✓", "✗");
        assert!(mm.nodes[&item_id].title.starts_with("✓ "), "Should add checkmark");
        mm.toggle_symbol("✓", "✗");
        assert!(mm.nodes[&item_id].title.starts_with("✗ "), "Should change to cross");
        mm.toggle_symbol("✓", "✗");
        assert!(!mm.nodes[&item_id].title.starts_with("✓") && !mm.nodes[&item_id].title.starts_with("✗"),
            "Should remove symbol");
    }

    #[test]
    fn test_toggle_numbers() {
        let mut mm = MindMap::from_text("root\n\tApple\n\tBanana\n\tCherry");
        let apple_id = find_node_by_title(&mm, "Apple");
        mm.active_node = apple_id;
        mm.toggle_numbers();
        // All siblings should be numbered
        let children = mm.nodes[&mm.root_id].children.clone();
        for (i, &cid) in children.iter().enumerate() {
            let title = &mm.nodes[&cid].title;
            assert!(title.starts_with(&format!("{}.", i + 1)),
                "Should be numbered: {}", title);
        }
        // Toggle again to remove
        mm.toggle_numbers();
        for &cid in &children {
            let title = &mm.nodes[&cid].title;
            assert!(!title.chars().next().unwrap().is_ascii_digit(),
                "Numbers should be removed: {}", title);
        }
    }

    #[test]
    fn test_positive_rank() {
        let mut mm = MindMap::from_text("root\n\tTask");
        let task_id = find_node_by_title(&mm, "Task");
        mm.active_node = task_id;
        mm.modify_positive_rank(1);
        assert!(mm.nodes[&task_id].title.starts_with('+'));
        mm.modify_positive_rank(2);
        assert!(mm.nodes[&task_id].title.starts_with("+++"));
        mm.modify_positive_rank(-1);
        assert!(mm.nodes[&task_id].title.starts_with("++"));
    }

    #[test]
    fn test_negative_rank() {
        let mut mm = MindMap::from_text("root\n\tTask");
        let task_id = find_node_by_title(&mm, "Task");
        mm.active_node = task_id;
        mm.modify_negative_rank(1);
        assert!(mm.nodes[&task_id].title.starts_with('-'));
        mm.modify_negative_rank(2);
        assert!(mm.nodes[&task_id].title.starts_with("---"));
        mm.modify_negative_rank(-1);
        assert!(mm.nodes[&task_id].title.starts_with("--"));
    }

    #[test]
    fn test_stars() {
        let mut mm = MindMap::from_text("root\n\tItem");
        let item_id = find_node_by_title(&mm, "Item");
        mm.active_node = item_id;
        mm.add_star();
        assert!(mm.nodes[&item_id].title.starts_with('★'));
        mm.add_star();
        assert!(mm.nodes[&item_id].title.starts_with("★★"));
        mm.remove_star();
        assert!(mm.nodes[&item_id].title.starts_with('★'));
        mm.remove_star();
        assert!(!mm.nodes[&item_id].title.starts_with('★'));
    }

    #[test]
    fn test_toggle_hide() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.toggle_hide();
        assert!(mm.nodes[&a_id].hidden, "A should be hidden");
        assert!(mm.active_node != a_id, "Active node should move away from hidden");
        // Show hidden should reveal it
        mm.show_hidden = true;
        mm.refresh_display();
        assert!(mm.visible_nodes.contains(&a_id), "A should be visible when show_hidden=true");
    }

    // ═══ Undo / Redo ══════════════════════════════════════════

    #[test]
    fn test_undo_edit() {
        let mut mm = MindMap::from_text("root\n\tOriginal");
        let orig_id = find_node_by_title(&mm, "Original");
        mm.active_node = orig_id;
        let original_title = mm.nodes[&orig_id].title.clone();
        mm.edit_node("Changed".to_string());
        assert_eq!(mm.nodes[&orig_id].title, "Changed");
        mm.undo();
        assert_eq!(mm.nodes[&orig_id].title, original_title, "Should restore original");
    }

    #[test]
    fn test_undo_insert() {
        let mut mm = MindMap::from_text("root\n\tA");
        let root = &mm.nodes[&mm.root_id];
        let child_count = root.children.len();
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.insert_sibling();
        assert!(mm.nodes[&mm.root_id].children.len() > child_count);
        mm.undo();
        assert_eq!(mm.nodes[&mm.root_id].children.len(), child_count, "Should revert insert");
    }

    #[test]
    fn test_undo_delete() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.delete_node(false);
        assert!(!mm.nodes.contains_key(&a_id));
        mm.undo();
        assert!(mm.nodes.contains_key(&a_id), "A should be restored");
        assert_eq!(mm.nodes[&a_id].title, "A");
    }

    #[test]
    fn test_redo() {
        let mut mm = MindMap::from_text("root\n\tX");
        let x_id = find_node_by_title(&mm, "X");
        mm.active_node = x_id;
        mm.edit_node("Y".to_string());
        assert_eq!(mm.nodes[&x_id].title, "Y");
        mm.undo();
        assert_eq!(mm.nodes[&x_id].title, "X");
        mm.redo();
        assert_eq!(mm.nodes[&x_id].title, "Y", "Redo should reapply");
    }

    #[test]
    fn test_max_undo_steps() {
        let mut mm = MindMap::new();
        // Push 60 undos (max is 50)
        for i in 0..60 {
            mm.insert_child();
            mm.edit_node(format!("Node {}", i));
        }
        assert!(mm.undo_stack.len() <= MindMap::MAX_UNDO, "Should not exceed MAX_UNDO");
    }

    // ═══ HTML Export ═══════════════════════════════════════════

    #[test]
    fn test_html_export() {
        let input = "root\n\tItem A\n\tItem B\n\t\tSub B1";
        let mm = MindMap::from_text(input);
        let html = mm.export_html();
        assert!(html.contains("<html>"), "Should be valid HTML");
        assert!(html.contains("Item A"), "Should contain Item A");
        assert!(html.contains("Sub B1"), "Should contain Sub B1");
        assert!(html.contains("<details"), "Should use details for parent nodes");
    }

    #[test]
    fn test_html_export_escapes() {
        let mm = MindMap::from_text("root\n\t<evil> & \"bad\"");
        let html = mm.export_html();
        assert!(!html.contains("<evil>"), "Should escape HTML tags");
        assert!(html.contains("&lt;evil&gt;"), "Should contain escaped tag");
    }

    // ═══ Layout ════════════════════════════════════════════════

    #[test]
    fn test_layout_produces_positions() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
        mm.calculate_layout(40, 0);
        assert!(!mm.layouts.is_empty(), "Should produce layouts");
        for (_id, layout) in &mm.layouts {
            assert!(layout.w > 0, "Node width should be positive");
            assert!(layout.h > 0, "Node height should be positive");
        }
    }

    #[test]
    fn test_canvas_builds() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        mm.calculate_layout(40, 0);
        mm.build_canvas();
        assert!(!mm.canvas.is_empty(), "Canvas should not be empty");
        assert!(mm.canvas.len() > 0, "Canvas should have rows");
        assert!(mm.canvas[0].len() > 0, "Canvas should have columns");
    }

    #[test]
    fn test_visible_nodes_tracks_collapse() {
        let mut mm = MindMap::from_text("root\n\tA\n\t\tA1\n\tB");
        mm.calculate_layout(40, 0);
        let count_before = mm.visible_nodes.len();
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.toggle_node();
        mm.refresh_display();
        let count_after = mm.visible_nodes.len();
        assert!(count_after < count_before, "Collapsing should reduce visible nodes");
    }

    // ═══ Canvas Rendering Tests ════════════════════════════════

    /// Build a simple mind map and verify canvas content.
    fn render_map(input: &str) -> MindMap {
        let mut mm = MindMap::from_text(input);
        mm.calculate_layout(40, 0);
        mm.build_canvas();
        mm
    }

    /// Get a row from the canvas as a String, trimmed of trailing spaces.
    fn canvas_row(mm: &MindMap, row: usize) -> String {
        if row >= mm.canvas.len() {
            return String::new();
        }
        mm.canvas[row].iter().collect::<String>().trim_end().to_string()
    }

    #[test]
    fn test_canvas_root_visible() {
        let mm = render_map("root\n\tA\n\tB");
        let row0 = canvas_row(&mm, 0);
        assert!(row0.contains("root"), "Root should be visible at row 0: {}", row0);
    }

    #[test]
    fn test_canvas_children_visible() {
        let mm = render_map("root\n\tAlpha\n\tBeta");
        let full: String = mm.canvas.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
        assert!(full.contains("Alpha"), "Child Alpha should be visible");
        assert!(full.contains("Beta"), "Child Beta should be visible");
    }

    #[test]
    fn test_canvas_has_connectors() {
        let mm = render_map("root\n\tChild");
        let full: String = mm.canvas.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
        assert!(full.contains('─'), "Should have horizontal connector '─'");
        // Single child at same Y may not have vertical bar — that's fine
    }

    #[test]
    fn test_connector_integrity_single_child_same_row() {
        // spacing=0: parent at y=0, child at y=1 (different rows!)
        // Parent horizontal + corner at row 0, child horizontal at row 1
        let mut mm = MindMap::from_text("root\n\tA");
        mm.line_spacing = 0;
        mm.refresh_display();
        // Parent row: root text + horizontal + corner
        let row0 = &mm.canvas[0];
        assert_eq!(row0[0], 'r');
        assert!(row0.iter().any(|&c| c == '╮' || c == '╯' || c == '─'),
            "Parent row should have connector");
        // Child row: corner + horizontal + text
        let child_y = mm.layouts.get(&3).unwrap().y;
        assert!(child_y > 0, "Child should be below parent");
    }

    #[test]
    fn test_connector_integrity_single_child_diff_row() {
        // spacing=1 → parent at y=0, child at y=2, connector bends
        let mut mm = MindMap::from_text("root\n\tA");
        mm.line_spacing = 1;
        mm.refresh_display();
        // Parent row should have horizontal line + corner
        let row0 = &mm.canvas[0];
        assert_eq!(row0[0], 'r');
        // Find where corner char is
        let has_corner = row0.iter().any(|&c| c == '╮' || c == '╯');
        assert!(has_corner, "Parent row should have corner char");
        // Child row should have corner + horizontal + text
        let child_y = mm.layouts.get(&3).unwrap().y;
        let child_row = &mm.canvas[child_y];
        let has_child_corner = child_row.iter().any(|&c| c == '╭' || c == '╰');
        assert!(has_child_corner, "Child row should have corner char");
        // Between parent and child: bar should exist
        let bar_x = mm.layouts.get(&3).unwrap().x - 3;
        for y in 1..child_y {
            assert!(mm.canvas[y][bar_x] == '│' || mm.canvas[y][bar_x] == '╭' || mm.canvas[y][bar_x] == '╰',
                "Row {} bar_x: expected bar char, got '{}'", y, mm.canvas[y][bar_x]);
        }
    }

    #[test]
    fn test_connector_integrity_multi_child() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
        mm.line_spacing = 0;
        mm.refresh_display();
        let bar_x = mm.layouts.get(&3).unwrap().x - 3;
        let parent_row = &mm.canvas[0];
        let parent_mid_y = 0usize;
        let first_id = 3usize;
        let first_y = mm.layouts.get(&first_id).unwrap().y;
        let last_id = mm.nodes.iter().find(|(_, n)| n.title == "C").map(|(id,_)|*id).unwrap();
        let last_y = mm.layouts.get(&last_id).unwrap().y;
        eprintln!("parent_mid={} first_y={} last_y={} bar_x={}", parent_mid_y, first_y, last_y, bar_x);
        eprintln!("Row 0 at bar_x: '{}' ({})", parent_row[bar_x], parent_row[bar_x] as u32);
        for y in 0..=3 {
            eprintln!("Row {}: '{}'", y, mm.canvas[y].iter().collect::<String>().trim_end());
        }
        assert_eq!(parent_row[bar_x], '┤', "Parent row bar should be '┤'");
        assert_eq!(mm.canvas[first_y][bar_x], '├', "First child bar should be '├'");
        assert_eq!(mm.canvas[last_y][bar_x], '╰', "Last child bar should be '╰'");
        for &cid in &[3, 4, 5] {
            let cl = &mm.layouts[&cid];
            let child_row = &mm.canvas[cl.y];
            for x in (bar_x + 1)..cl.x {
                assert_eq!(child_row[x], '─', "Child {} row {} col {}: expected '─' got '{}'", cid, cl.y, x, child_row[x]);
            }
        }
    }

    #[test]
    fn test_connector_no_space_at_bar() {
        // All cases: the bar_x column should never be space at any row that has a connector
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        mm.line_spacing = 0;
        mm.refresh_display();
        let bar_x = mm.layouts.get(&3).unwrap().x - 3;
        let bar_top = 0usize;
        let bar_bottom = mm.layouts.get(&4).unwrap().y;
        for y in bar_top..=bar_bottom {
            assert_ne!(mm.canvas[y][bar_x], ' ',
                "Bar at row {} col {} should not be space", y, bar_x);
        }
    }

    #[test]
    fn test_canvas_connector_no_gaps() {
        let mut mm = MindMap::from_text("root\n\tA\n\tB");
        mm.calculate_layout(40, 0);
        mm.build_canvas();
        let row0: String = mm.canvas[0].iter().collect();
        let start = row0.find("root").unwrap() + 4;
        let segment: String = mm.canvas[0][start..].iter().take(6).collect();
        // Bar should extend from parent row — no gaps
        assert!(!segment.starts_with(' '),
            "Connector should start right after text, got: '{}'", segment);
    }

    #[test]
    fn test_canvas_multi_branch_connectors() {
        // Use spacing=1 so children are clearly separated
        let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
        mm.line_spacing = 1;
        mm.refresh_display();
        let full: String = mm.canvas.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
        // Multi-branch should have corner chars
        assert!(full.contains('╭') || full.contains('╰'), "Should have corner chars for multi-branch");
    }

    #[test]
    fn test_canvas_collapsed_shows_plus() {
        let mut mm = MindMap::from_text("root\n\tParent\n\t\tHidden");
        let pid = find_node_by_title(&mm, "Parent");
        mm.active_node = pid;
        mm.toggle_node();
        mm.refresh_display();
        let full: String = mm.canvas.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
        assert!(full.contains("[+]"), "Collapsed parent should show [+]: {}", full);
    }

    #[test]
    fn test_canvas_multiline_node() {
        let mm = render_map("root\n\tLine1\\nLine2");
        let full: String = mm.canvas.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
        assert!(full.contains("Line1"), "First line should be visible");
        assert!(full.contains("Line2"), "Second line should be visible");
    }

    #[test]
    fn test_canvas_narrow_width_wraps() {
        // Use spaces so word-wrapping can split
        let mut mm = MindMap::from_text("root\n\tA Very Long Title That Must Wrap");
        mm.calculate_layout(10, 0);
        mm.build_canvas();
        let cid = find_node_by_title(&mm, "A Very Long Title That Must Wrap");
        let layout = mm.layouts.get(&cid).unwrap();
        assert!(layout.lines > 1, "Long spaced text should wrap with width=10, got {} lines", layout.lines);
    }

    // ═══ Helpers ═══════════════════════════════════════════════

    fn find_node_by_title(mm: &MindMap, title: &str) -> usize {
        mm.nodes.iter()
            .find(|(_, n)| n.title == title)
            .map(|(id, _)| *id)
            .expect(&format!("Node '{}' not found", title))
    }

    #[test]
    fn test_multiline_text() {
        // \\n in the file becomes \n in the title after unescaping
        let mut mm = MindMap::from_text("root\n\tLine1\\nLine2");
        mm.calculate_layout(40, 0);
        let child_id = find_node_by_title(&mm, "Line1\nLine2");
        let layout = mm.layouts.get(&child_id).unwrap();
        assert!(layout.lines >= 2, "Multi-line node should have >= 2 lines, got {}", layout.lines);
    }

    #[test]
    fn test_wrap_text_newlines() {
        let lines = MindMap::lines_for_display("Hello\nWorld", 40);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Hello");
        assert_eq!(lines[1], "World");
    }

    #[test]
    fn test_chinese_newline_edit() {
        let mut mm = MindMap::from_text("root\n\t测试\\n换行");
        mm.refresh_display();
        let cid = find_node_by_title(&mm, "测试\n换行");
        assert!(mm.layouts[&cid].lines >= 2);
    }

    #[test]
    fn test_edit_multiline_no_panic() {
        let mut mm = MindMap::new();
        mm.insert_child();
        mm.edit_node("这是测试\n第二行".to_string());
        mm.refresh_display();
        assert!(mm.canvas.len() > 0);
    }

    #[test]
    fn test_note_persistence() {
        use std::fs;
        let tmp = "/tmp/zmind_test_note.hmm";
        let tmp_json = "/tmp/zmind_test_note.hmm.json";
        let _ = fs::remove_file(tmp);
        let _ = fs::remove_file(tmp_json);

        // Create and save with note
        let mut mm = MindMap::from_text("root\n\tA");
        mm.filename = Some(std::path::PathBuf::from(tmp));
        let a_id = find_node_by_title(&mm, "A");
        mm.active_node = a_id;
        mm.update_note("This is a note".to_string());
        mm.save().unwrap();

        // Reload and verify
        let mm2 = MindMap::from_file(&std::path::PathBuf::from(tmp)).unwrap();
        let a2_id = find_node_by_title(&mm2, "A");
        assert_eq!(mm2.nodes[&a2_id].note, "This is a note");

        // Cleanup
        let _ = fs::remove_file(tmp);
        let _ = fs::remove_file(tmp_json);
    }

    #[test]
    fn test_save_load_newline_roundtrip() {
        let mut mm = MindMap::from_text("root\n\tA\\nB");
        let cid = find_node_by_title(&mm, "A\nB");
        mm.active_node = cid;
        mm.edit_node("中文\n测试".to_string());
        let saved = mm.to_text();
        let mut mm2 = MindMap::from_text(&saved);
        mm2.refresh_display();
        let _ = find_node_by_title(&mm2, "中文\n测试");
    }
}
