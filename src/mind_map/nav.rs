use super::MindMap;

impl MindMap {
    // ─── Navigation ────────────────────────────────────────────────

    pub fn go_left(&mut self) {
        let current = self.active_node;
        if current == self.root_id || current == 0 {
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

    // ─── Collapse / Expand ─────────────────────────────────────────

    pub fn toggle_node(&mut self) {
        let current = self.active_node;
        if let Some(node) = self.nodes.get_mut(&current) {
            if node.is_leaf() {
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
        self.collapse_level(1);
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

        for node in self.nodes.values_mut() {
            node.collapsed = true;
        }

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

        let mut first_level_ancestor = active;
        loop {
            let parent = self
                .nodes
                .get(&first_level_ancestor)
                .map(|n| n.parent)
                .unwrap_or(0);
            if parent == self.root_id || parent == 0 {
                break;
            }
            first_level_ancestor = parent;
        }

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
                self.active_node = node.parent;
            }
        }
        self.refresh_display();
    }

    /// Expand the node's ancestors (and itself) so it becomes visible,
    /// e.g. before jumping to a search result inside a collapsed subtree.
    pub fn reveal_node(&mut self, id: usize) {
        if !self.nodes.contains_key(&id) {
            return;
        }
        let mut ancestors = Vec::new();
        let mut current = id;
        while let Some(node) = self.nodes.get(&current) {
            ancestors.push(current);
            current = node.parent;
        }
        for ancestor in ancestors {
            if let Some(node) = self.nodes.get_mut(&ancestor) {
                node.collapsed = false;
            }
        }
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
        self.refresh_display();
    }

    pub fn sort_siblings(&mut self) {
        let current = self.active_node;
        if current == self.root_id {
            return;
        }
        self.push_undo();
        let parent_id = self.get_parent_id(current).unwrap_or(0);
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
                let title_a = child_titles
                    .iter()
                    .find(|(id, _)| id == a)
                    .map(|(_, t)| t.as_str())
                    .unwrap_or("");
                let title_b = child_titles
                    .iter()
                    .find(|(id, _)| id == b)
                    .map(|(_, t)| t.as_str())
                    .unwrap_or("");
                title_a.cmp(title_b)
            });
        }
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
                node.title = format!("{} {}", symbol2, &node.title[len1..]);
            } else if node.title.starts_with(&format!("{} ", symbol2)) {
                node.title = node.title[len2..].to_string();
            } else {
                node.title = format!("{} {}", symbol1, node.title);
            }
        }

        self.refresh_display();
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

        let has_numbers = siblings.iter().any(|&sid| {
            self.nodes
                .get(&sid)
                .map(|n| {
                    n.title
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        });

        for (i, &sid) in siblings.iter().enumerate() {
            if let Some(node) = self.nodes.get_mut(&sid) {
                if has_numbers {
                    if let Some(pos) = node.title.find(". ") {
                        if node.title[..pos].chars().all(|c| c.is_ascii_digit()) {
                            node.title = node.title[pos + 2..].to_string();
                        }
                    }
                } else {
                    let pad = if siblings.len() > 9 { 2 } else { 1 };
                    node.title = format!("{:0width$}. {}", i + 1, node.title, width = pad);
                }
            }
        }

        self.refresh_display();
    }

    fn strip_ranks(title: &str) -> String {
        let trimmed = title.trim_start_matches(['+', '-', ' ']);
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

        self.refresh_display();
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

        self.refresh_display();
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
                let star_count = node
                    .title
                    .chars()
                    .take_while(|&c| c == '★' || c == '☆')
                    .count();
                let rest: String = node.title.chars().skip(star_count).collect();
                let rest = rest.trim_start();
                node.title = format!("{} {}", "★".repeat(star_count + 1), rest);
            }
        }
        self.refresh_display();
    }

    pub fn remove_star(&mut self) {
        let current = self.active_node;
        self.push_undo();
        if let Some(node) = self.nodes.get_mut(&current) {
            let star_count = node
                .title
                .chars()
                .take_while(|&c| c == '★' || c == '☆')
                .count();
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
        self.refresh_display();
    }
}
