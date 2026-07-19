use super::{MindMap, Node};

impl MindMap {
    /// Get the next available ID.
    pub fn next_id(&self) -> usize {
        self.nodes.keys().max().map(|m| m + 1).unwrap_or(2)
    }

    /// Get the parent ID of a node.
    pub fn get_parent_id(&self, id: usize) -> Option<usize> {
        self.nodes.get(&id).map(|n| n.parent)
    }

    /// Collect all descendants of a node (BFS).
    pub fn collect_all_descendants(&self, id: usize) -> Vec<usize> {
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

    // ─── Editing ───────────────────────────────────────────────────

    pub fn edit_node(&mut self, new_title: String) {
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&self.active_node) {
            node.title = new_title;
        }
        self.refresh_display();
    }

    pub fn update_note(&mut self, note: String) {
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&self.active_node) {
            node.note = note;
        }
    }

    pub fn insert_sibling(&mut self) {
        self.push_undo();
        let current = self.active_node;
        let parent_id = self.get_parent_id(current).unwrap_or(0);

        if parent_id == 0 && current == self.root_id {
            self.insert_child();
            return;
        }

        let new_id = self.next_id();
        let new_node = Node::new("NEW".to_string(), parent_id);

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if let Some(pos) = parent.children.iter().position(|&c| c == current) {
                parent.children.insert(pos + 1, new_id);
            } else {
                parent.children.push(new_id);
            }
        }

        self.nodes.insert(new_id, new_node);
        self.active_node = new_id;
        self.refresh_display();
    }

    pub fn insert_child(&mut self) {
        self.push_undo();
        let current = self.active_node;

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

        let parent_id = self.get_parent_id(current).unwrap_or(0);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.retain(|&c| c != current);
        }

        let to_remove = self.collect_all_descendants(current);
        for id in &to_remove {
            self.nodes.remove(id);
        }
        self.nodes.remove(&current);

        if let Some(parent) = self.nodes.get(&parent_id) {
            if parent.children.is_empty() {
                self.active_node = parent_id;
            } else {
                self.active_node = parent.children[0];
            }
        } else {
            self.active_node = self.root_id;
        }

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

        self.refresh_display();
    }

    pub fn delete_node_no_clipboard(&mut self) {
        self.delete_node(false);
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
    pub fn from_text(text: &str) -> Self {
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.is_empty() {
            return Self::new();
        }
        let mut nodes = std::collections::HashMap::new();
        nodes.insert(
            0,
            Node {
                title: String::new(),
                parent: usize::MAX,
                children: Vec::new(),
                collapsed: false,
                hidden: true,
                note: String::new(),
            },
        );
        let mut min_indent = usize::MAX;
        for line in &lines {
            let indent = line.len() - line.trim_start().len();
            if indent < min_indent {
                min_indent = indent;
            }
        }
        let mut id_counter: usize = 2;
        let mut level_parent = std::collections::HashMap::new();
        let mut level_indent = std::collections::HashMap::new();
        level_parent.insert(1, 0);
        level_indent.insert(1, 0);
        let mut prev_level = 1;
        let mut prev_indent = 0;
        for line in &lines {
            let indent = line.len() - line.trim_start().len();
            let adjusted = indent.saturating_sub(min_indent);
            let title = line.trim().to_string().replace("\\n", "\n");
            let level = if adjusted > prev_indent {
                let l = prev_level + 1;
                level_indent.insert(l, adjusted);
                l
            } else if adjusted < prev_indent {
                let mut found = 1;
                for (&l, &i) in &level_indent {
                    if i == adjusted && l > found {
                        found = l;
                    }
                }
                found
            } else {
                prev_level
            };
            if level > prev_level {
                level_parent.insert(level, id_counter - 1);
            }
            let parent = *level_parent.get(&level).unwrap_or(&0);
            let node = Node::new(title, parent);
            nodes.insert(id_counter, node);
            if let Some(p) = nodes.get_mut(&parent) {
                p.children.push(id_counter);
            }
            prev_indent = adjusted;
            prev_level = level;
            id_counter += 1;
        }
        let mut first_level: Vec<usize> = Vec::new();
        for (&id, node) in &nodes {
            if id >= 2 && node.parent == 0 {
                first_level.push(id);
            }
        }
        let root_id = if first_level.is_empty() {
            if let Some(n) = nodes.get_mut(&2) {
                n.parent = 0;
            }
            2
        } else if first_level.len() == 1 {
            let rid = first_level[0];
            if let Some(n) = nodes.get_mut(&rid) {
                n.parent = 0;
            }
            rid
        } else {
            let rid = 1;
            nodes.insert(
                1,
                Node {
                    title: "root".to_string(),
                    parent: 0,
                    children: first_level.clone(),
                    collapsed: false,
                    hidden: false,
                    note: String::new(),
                },
            );
            for &id in &first_level {
                if let Some(n) = nodes.get_mut(&id) {
                    n.parent = 1;
                }
            }
            rid
        };
        if let Some(n) = nodes.get_mut(&0) {
            n.children = vec![root_id];
        }
        let active_node = root_id;
        let mut mm = MindMap {
            name: String::new(),
            nodes,
            root_id,
            active_node,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: None,
            visible_nodes: Vec::new(),
            layouts: std::collections::HashMap::new(),
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
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
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
        self.refresh_display();
    }

    /// Parse clipboard text and transplant all top-level nodes. Returns new IDs.
    fn paste_text(&mut self, text: &str, target_parent: usize) -> Vec<usize> {
        let temp = MindMap::from_text(text);
        let temp_root = temp.root_id;
        let temp_root_children: Vec<usize> = temp
            .nodes
            .get(&temp_root)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        let is_virtual = temp_root == 1
            && temp
                .nodes
                .get(&1)
                .map(|n| n.title.as_str() == "root" && n.parent == 0)
                .unwrap_or(false);

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
        self.refresh_display();
    }
}
