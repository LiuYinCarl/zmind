use std::collections::HashMap;

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use super::{MindMap, NodeLayout};

impl MindMap {
    /// Rebuild display state: visible list, layout, and canvas.
    pub fn refresh_display(&mut self) {
        self.visible_nodes = self.collect_visible(self.root_id, &mut Vec::new());
        self.calculate_layout(self.max_node_width, self.line_spacing);
        self.build_canvas();
    }

    fn collect_visible(&self, id: usize, ancestors_collapsed: &mut Vec<bool>) -> Vec<usize> {
        let mut result = Vec::new();
        if let Some(node) = self.nodes.get(&id) {
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

    /// Calculate layout for all visible nodes in one post-order pass.
    pub fn calculate_layout(&mut self, max_width: usize, line_spacing: usize) {
        self.layouts.clear();
        if self.visible_nodes.is_empty() {
            self.visible_nodes = self.collect_visible(self.root_id, &mut Vec::new());
        }

        let root = self.root_id;
        let spacing = line_spacing.max(0);
        let connector_gap = 6;

        let map_h = self.layout_pass(root, 0, 0, 0, max_width, spacing, connector_gap);

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

        self.map_width = self
            .layouts
            .values()
            .map(|l| l.x + l.w + 4)
            .max()
            .unwrap_or(1);
        self.map_height = map_h.max(1);
    }

    /// Post-order layout: returns total height of subtree rooted at id.
    fn layout_pass(
        &mut self,
        id: usize,
        depth: usize,
        base_y: usize,
        parent_rx: usize,
        max_w: usize,
        spacing: usize,
        gap: usize,
    ) -> usize {
        let node = match self.nodes.get(&id) {
            Some(n) => n.clone(),
            None => return 0,
        };

        let is_at_end = node.is_leaf()
            || node.collapsed
            || node.children.iter().all(|c| {
                self.nodes
                    .get(c)
                    .map(|n| n.hidden && !self.show_hidden)
                    .unwrap_or(true)
            });

        let width_limit = if is_at_end {
            (max_w as f64 * 1.3) as usize
        } else {
            max_w
        };
        let display_lines = Self::lines_for_display(&node.title, width_limit);
        let num_lines = display_lines.len().max(1);
        let display_w = display_lines
            .iter()
            .map(|l| UnicodeWidthStr::width(l.as_str()))
            .max()
            .unwrap_or(2);
        let w = display_lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(1)
            .max(1);

        let x = if depth == 0 { 0 } else { parent_rx + gap };

        let own_h = num_lines + spacing;

        let my_rx = x + display_w;
        let children_base = base_y + own_h;
        let mut children_heights = 0usize;
        if !node.collapsed {
            let show = self.show_hidden;
            for &child_id in &node.children {
                if self
                    .nodes
                    .get(&child_id)
                    .map(|n| !n.hidden || show)
                    .unwrap_or(false)
                {
                    let ch = self.layout_pass(
                        child_id,
                        depth + 1,
                        children_base + children_heights,
                        my_rx,
                        max_w,
                        spacing,
                        gap,
                    );
                    children_heights += ch;
                }
            }
        }

        let total_h = own_h + children_heights;

        self.layouts.insert(
            id,
            NodeLayout {
                id,
                x,
                y: base_y,
                w,
                h: total_h,
                depth,
                lines: num_lines,
                width_limit,
            },
        );

        total_h
    }

    /// Build the full canvas text buffer for the mind map.
    pub fn build_canvas(&mut self) {
        let h = self.map_height.max(1);
        let w = self.map_width.max(1);

        self.canvas = vec![vec![' '; w]; h];

        self.draw_connections(self.root_id);

        let node_data: Vec<(usize, String, NodeLayout)> = self
            .layouts
            .iter()
            .filter_map(|(&id, layout)| {
                self.nodes
                    .get(&id)
                    .map(|node| (id, node.title.clone(), layout.clone()))
            })
            .collect();

        for (_id, title, layout) in node_data {
            let display_lines = Self::lines_for_display(&title, self.max_node_width);
            for (li, line) in display_lines.iter().enumerate() {
                let cy = layout.y + li;
                if cy < self.canvas.len() {
                    self.draw_text_at(layout.x, cy, line);
                }
            }
        }

        let cols = self.map_width.max(1);
        let mut col_w = vec![1; cols];
        for row in &self.canvas {
            for (j, &ch) in row.iter().enumerate() {
                let w = ch.width().unwrap_or(1);
                if w > col_w[j] {
                    col_w[j] = w;
                }
            }
        }
        self.canvas_col_widths = col_w;
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

        let visible_children: Vec<usize> = node
            .children
            .iter()
            .filter(|c| self.layouts.contains_key(c))
            .copied()
            .collect();

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

        let conn_right_len: usize = 3;

        let first_child = visible_children[0];
        let child_x = self
            .layouts
            .get(&first_child)
            .map(|l| l.x)
            .unwrap_or(my.x + my.w + 10);

        let bar_x = child_x.saturating_sub(conn_right_len);

        let parent_right = my.x + my.w;
        let parent_mid_y = my.y + my.lines / 2;

        if visible_children.len() == 1 {
            let child = self.layouts.get(&first_child).cloned();
            if let Some(cl) = child {
                let child_mid_y = cl.y + cl.lines / 2;

                if parent_mid_y == child_mid_y {
                    for x in parent_right..child_x.min(self.map_width) {
                        self.set_cell(x, parent_mid_y, '\u{2500}');
                    }
                } else {
                    for x in parent_right..bar_x.min(self.map_width) {
                        self.set_cell(x, parent_mid_y, '\u{2500}');
                    }
                    let (top, bottom) = if parent_mid_y < child_mid_y {
                        (parent_mid_y, child_mid_y)
                    } else {
                        (child_mid_y, parent_mid_y)
                    };
                    for y in top..=bottom {
                        self.set_cell(bar_x, y, '\u{2502}');
                    }
                    for x in (bar_x + 1)..child_x.min(self.map_width) {
                        self.set_cell(x, child_mid_y, '\u{2500}');
                    }
                    if child_mid_y < parent_mid_y {
                        self.set_cell(bar_x, child_mid_y, '\u{256d}');
                        self.set_cell(bar_x, parent_mid_y, '\u{256f}');
                    } else {
                        self.set_cell(bar_x, child_mid_y, '\u{2570}');
                        self.set_cell(bar_x, parent_mid_y, '\u{256e}');
                    }
                }
            }
            self.draw_connections(first_child);
            return;
        }

        for x in parent_right..bar_x.min(self.map_width) {
            self.set_cell(x, parent_mid_y, '\u{2500}');
        }

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
                    (true, true, true) => '\u{253c}',
                    (true, true, false) => '\u{252c}',
                    (true, false, true) => '\u{2534}',
                    (false, true, false) => '\u{251c}',
                    (false, false, true) => '\u{2570}',
                    (true, false, false) => '\u{2524}',
                    _ => '\u{2502}',
                };
                self.set_cell(bar_x, y, ch);
            }

            // Collect child mid positions before mutable drawing.
            let child_positions: Vec<(usize, usize, usize)> = visible_children
                .iter()
                .filter_map(|&cid| {
                    self.layouts.get(&cid).map(|cl| (cid, cl.y + cl.lines / 2, cl.x))
                })
                .collect();

            for (i, (child_id, child_mid_y, child_x)) in child_positions.iter().enumerate() {
                let child_id = *child_id;
                let child_mid_y = *child_mid_y;
                let child_x = *child_x;
                // Overwrite bar_x at child's mid-y with branch connector.
                // First child already handled by the vertical-bar loop above
                // (├ or ┬); last child also handled (╰ or ┴). Intermediate children
                // get │ from the loop — fix them to ├.
                let is_first = i == 0;
                let is_last = i == visible_children.len() - 1;
                let is_parent = child_mid_y == parent_mid_y;
                if !is_first && !is_last {
                    let ch = if is_parent { '\u{253c}' } else { '\u{251c}' };
                    self.set_cell(bar_x, child_mid_y, ch);
                }
                for x in (bar_x + 1)..child_x.min(self.map_width) {
                    self.set_cell(x, child_mid_y, '\u{2500}');
                }
                self.draw_connections(child_id);
            }
        } else {
            for &child_id in &visible_children {
                self.draw_connections(child_id);
            }
        }
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

    /// Split text into display lines: first by \n, then word-wrap each.
    pub fn lines_for_display(text: &str, max_w: usize) -> Vec<String> {
        let mut result = Vec::new();
        for raw_line in text.split('\n') {
            if raw_line.is_empty() {
                result.push(String::new());
                continue;
            }
            let rl_w = UnicodeWidthStr::width(raw_line);
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
            let test = if current.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current, word)
            };
            if UnicodeWidthStr::width(test.as_str()) > max_w && !current.is_empty() {
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

    /// Export canvas as ASCII with per-column visual alignment.
    pub fn export_ascii(&self) -> String {
        if self.canvas.is_empty() {
            return String::new();
        }
        let cols = self.canvas.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut col_w: Vec<usize> = vec![1; cols];
        for row in &self.canvas {
            for (j, &ch) in row.iter().enumerate() {
                let w = ch.width().unwrap_or(1);
                if w > col_w[j] {
                    col_w[j] = w;
                }
            }
        }
        let mut out = String::new();
        for row in &self.canvas {
            let mut line = String::new();
            let mut vis = 0usize;
            for (j, &ch) in row.iter().enumerate() {
                let w = ch.width().unwrap_or(1);
                let expect: usize = col_w[..j].iter().sum();
                // Skip alignment padding between consecutive narrow chars
                // that sit under a wide column (e.g. "NEW" below CJK text).
                let prev_narrow = j > 0 && row[j - 1].width().unwrap_or(1) == 1;
                if !(w == 1 && prev_narrow && col_w[j - 1] > 1 && ch.is_alphabetic()) {
                    while vis < expect {
                        line.push(' ');
                        vis += 1;
                    }
                }
                line.push(ch);
                vis += w;
            }
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }

    /// Export the mind map as an HTML file.
    pub fn export_html(&self) -> String {
        let title = self
            .nodes
            .get(&self.root_id)
            .map(|n| n.title.as_str())
            .unwrap_or("Mind Map");

        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"UTF-8\">\n");
        html.push_str(&format!("<title>{}</title>\n", title));
        html.push_str("<style>\n");
        html.push_str("body { font-family: system-ui, sans-serif; background: #1a1a2e; color: #e0e0e0; padding: 2em; }\n");
        html.push_str(
            "ul { list-style: none; padding-left: 1.5em; border-left: 2px solid #444; }\n",
        );
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
                        html.push_str(&format!(
                            "<li><details{}><summary>{}</summary>\n",
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
}

/// Simple HTML entity escaping
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
pub fn find_node_by_title(mm: &MindMap, title: &str) -> usize {
    mm.nodes
        .iter()
        .find(|(_, n)| n.title == title)
        .map(|(id, _)| *id)
        .expect(&format!("Node '{}' not found", title))
}
