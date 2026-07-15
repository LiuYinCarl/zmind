use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use tui_input::Input;

use crate::{mind_map::MindMap, App};

pub struct View {}

impl View {
    /// File picker: show available .hmm files + new option.
    pub fn show_file_picker(app: &mut App, f: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = Vec::new();
        items.push(ListItem::from(Span::styled(
            "+ New mind map",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        for path in &app.file_list {
            let name = path.file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "?".to_string());
            items.push(ListItem::from(Span::raw(name)));
        }

        let list = List::new(items)
            .block(Block::bordered().title(" zmind - Open Mind Map "))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
            .highlight_symbol("> ");

        f.render_stateful_widget(list, area, &mut app.file_list_state);
    }

    /// Render the mind map as a proper visual map with connecting lines.
    pub fn show_mind_map(app: &App, f: &mut Frame, area: Rect) {
        let mm = &app.mind_map;
        let vx = app.viewport_x;
        let vy = app.viewport_y;

        let max_rows = area.height.saturating_sub(2) as usize;
        let max_cols = area.width.saturating_sub(2) as usize;

        let mut lines: Vec<Line> = Vec::new();

        for row_offset in 0..max_rows {
            let canvas_row = vy + row_offset;
            let mut row_chars: Vec<char> = Vec::new();

            if canvas_row < mm.canvas.len() {
                let canvas_cols = mm.canvas[canvas_row].len();
                for col_offset in 0..max_cols {
                    let canvas_col = vx + col_offset;
                    if canvas_col < canvas_cols {
                        row_chars.push(mm.canvas[canvas_row][canvas_col]);
                    } else {
                        row_chars.push(' ');
                    }
                }
            } else {
                row_chars = vec![' '; max_cols];
            }

            // Highlight active node (multi-line aware)
            if let Some(active_layout) = mm.layouts.get(&mm.active_node) {
                let ax = active_layout.x;
                let ay = active_layout.y;
                let aw = active_layout.w;
                let alines = active_layout.lines;

                if canvas_row >= ay && canvas_row < ay + alines {
                    let rel_start = ax.saturating_sub(vx);
                    let rel_end = (ax + aw).saturating_sub(vx).min(max_cols);

                    if rel_start < rel_end && rel_end <= row_chars.len() {
                        let mut styled_spans: Vec<Span> = Vec::new();
                        if rel_start > 0 {
                            styled_spans.push(Span::raw(
                                row_chars[..rel_start].iter().collect::<String>()
                            ));
                        }
                        styled_spans.push(Span::styled(
                            row_chars[rel_start..rel_end].iter().collect::<String>(),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Rgb(215, 135, 0))
                                .add_modifier(Modifier::BOLD),
                        ));
                        if rel_end < row_chars.len() {
                            styled_spans.push(Span::raw(
                                row_chars[rel_end..].iter().collect::<String>()
                            ));
                        }
                        lines.push(Line::from(styled_spans));
                    } else {
                        lines.push(Line::raw(row_chars.iter().collect::<String>()));
                    }
                } else {
                    lines.push(Line::raw(row_chars.iter().collect::<String>()));
                }
            } else {
                lines.push(Line::raw(row_chars.iter().collect::<String>()));
            }
        }

        let title = Self::get_title(app);
        let paragraph = Paragraph::new(Text::from(lines))
            .block(Block::bordered().title(title));

        f.render_widget(paragraph, area);
    }

    fn get_title(app: &App) -> String {
        let mm = &app.mind_map;
        let modified = if mm.modified { " [+]" } else { "" };
        let pos = format!(" @{},{}", app.viewport_x, app.viewport_y);
        match &mm.filename {
            Some(path) => format!(
                " zmind: {}{}{} ",
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "?".to_string()),
                modified,
                pos,
            ),
            None => format!(" zmind{}{} ", modified, pos),
        }
    }

    /// Render status bar at the bottom.
    pub fn show_status_bar(app: &App, f: &mut Frame, area: Rect) {
        let mm = &app.mind_map;
        let node = mm.nodes.get(&mm.active_node);
        let title = node.map(|n| n.title.as_str()).unwrap_or("");

        let depth = Self::get_depth(mm, mm.active_node);

        // Safe truncation: take at most 27 chars, not bytes
        let display_title: String = title.chars().take(27).collect();
        let display_title = if title.chars().count() > 27 {
            format!("{}...", display_title)
        } else {
            display_title
        };

        let status = format!(
            " Node: {} | Depth: {} | Nodes: {} | Canvas: {}x{} ",
            display_title,
            depth,
            mm.visible_nodes.len(),
            mm.map_width,
            mm.map_height,
        );

        f.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::Gray)),
            area,
        );
    }

    fn get_depth(mm: &MindMap, node_id: usize) -> usize {
        let mut depth = 0;
        let mut current = node_id;
        loop {
            if let Some(node) = mm.nodes.get(&current) {
                if node.parent == 0 || node.parent == usize::MAX {
                    break;
                }
                current = node.parent;
                if current == mm.root_id {
                    break;
                }
                depth += 1;
            } else {
                break;
            }
        }
        depth
    }

    /// Render hint bar at the top.
    pub fn show_hint_bar(f: &mut Frame, area: Rect) {
        let hint = Span::styled(
            " h/? help | q quit | ←↑↓→/jkl:move | Ctrl+arrows:scroll",
            Style::default().fg(Color::Green),
        );
        f.render_widget(Paragraph::new(Line::from(hint)), area);
    }

    pub fn show_file_picker_hint(f: &mut Frame, area: Rect) {
        let hint = Span::styled(
            " ↑↓/jk:move | Enter:open | n:new | q:quit",
            Style::default().fg(Color::Green),
        );
        f.render_widget(Paragraph::new(Line::from(hint)), area);
    }

    // ─── Modals ────────────────────────────────────────────────────

    pub fn show_edit_modal(f: &mut Frame, area: Rect, input: &Input, replace: bool) {
        let title = if replace { "Edit (replace)" } else { "Edit (append)" };
        let modal_area = Self::create_rect_area(60, 3, area);

        let width = modal_area.width.max(3) - 3;
        let scroll = input.visual_scroll(width as usize);

        let input_widget = Paragraph::new(input.value())
            .block(Block::default().borders(Borders::ALL).title(title))
            .scroll((0, scroll as u16));

        f.render_widget(Clear, modal_area);
        f.render_widget(input_widget, modal_area);

        f.set_cursor(
            modal_area.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
            modal_area.y + 1,
        );
    }

    pub fn show_search_modal(f: &mut Frame, area: Rect, input: &Input) {
        let modal_area = Self::create_rect_area(50, 3, area);

        let width = modal_area.width.max(3) - 3;
        let scroll = input.visual_scroll(width as usize);

        let input_widget = Paragraph::new(input.value())
            .block(Block::default().borders(Borders::ALL).title("Search"))
            .scroll((0, scroll as u16));

        f.render_widget(Clear, modal_area);
        f.render_widget(input_widget, modal_area);

        f.set_cursor(
            modal_area.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
            modal_area.y + 1,
        );
    }

    pub fn show_save_as_modal(f: &mut Frame, area: Rect, input: &Input) {
        let modal_area = Self::create_rect_area(50, 3, area);

        let width = modal_area.width.max(3) - 3;
        let scroll = input.visual_scroll(width as usize);

        let input_widget = Paragraph::new(input.value())
            .block(Block::default().borders(Borders::ALL).title("Save As"))
            .scroll((0, scroll as u16));

        f.render_widget(Clear, modal_area);
        f.render_widget(input_widget, modal_area);

        f.set_cursor(
            modal_area.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
            modal_area.y + 1,
        );
    }

    pub fn show_message_modal(f: &mut Frame, area: Rect, message: &str) {
        let widget = Paragraph::new(Text::from(message))
            .alignment(Alignment::Center)
            .block(Block::bordered());

        let modal_area = Self::create_rect_area(50, 3, area);
        f.render_widget(Clear, modal_area);
        f.render_widget(widget, modal_area);
    }

    pub fn show_node_details_modal(app: &App, f: &mut Frame, area: Rect) {
        let mm = &app.mind_map;
        let node = mm.nodes.get(&mm.active_node);
        let title = node.map(|n| n.title.as_str()).unwrap_or("");
        let note = node.map(|n| n.note.as_str()).unwrap_or("");

        let note_display = if note.is_empty() { "(empty)" } else { note };

        let lines = vec![
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(title),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Note: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(note_display),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                "e: edit note | any other key: close",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )),
        ];

        let widget = Paragraph::new(Text::from(lines))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .block(Block::bordered().title(" Node Details "));

        let modal_area = Self::create_rect_area(60, 10, area);
        f.render_widget(Clear, modal_area);
        f.render_widget(widget, modal_area);
    }

    pub fn show_edit_note_modal(f: &mut Frame, area: Rect, input: &Input) {
        let modal_area = Self::create_rect_area(60, 5, area);

        let width = modal_area.width.max(3) - 3;
        let scroll = input.visual_scroll(width as usize);

        let input_widget = Paragraph::new(input.value())
            .block(Block::default().borders(Borders::ALL).title("Note"))
            .scroll((0, scroll as u16));

        f.render_widget(Clear, modal_area);
        f.render_widget(input_widget, modal_area);

        f.set_cursor(
            modal_area.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
            modal_area.y + 1,
        );
    }

    pub fn show_confirm_modal(f: &mut Frame, area: Rect, message: &str) {
        let widget = Paragraph::new(Text::from(format!(
            "{}\n\nPress y to confirm, any other key to cancel",
            message
        )))
        .alignment(Alignment::Center)
        .block(Block::bordered().title("Confirm"));

        let modal_area = Self::create_rect_area(50, 5, area);
        f.render_widget(Clear, modal_area);
        f.render_widget(widget, modal_area);
    }

    pub fn show_help_modal(f: &mut Frame, area: Rect) {
        let bindings = [
            ("Navigation", ""),
            ("←", "Go to parent"),
            ("l / →", "Go to first child"),
            ("j / ↓", "Go down (next sibling)"),
            ("k / ↑", "Go up (previous sibling)"),
            ("g", "Go to top"),
            ("G", "Go to bottom"),
            ("m / ~", "Go to root"),
            ("Ctrl+arrows", "Scroll viewport"),
            ("c", "Center on active node"),
            ("", ""),
            ("Editing", ""),
            ("o / Enter", "Insert sibling"),
            ("O / Tab", "Insert child"),
            ("e / a", "Edit node (append)"),
            ("E / A", "Edit node (replace)"),
            ("i / I", "Node details / note"),
            ("d", "Cut node (to clipboard)"),
            ("D", "Cut children (to clipboard)"),
            ("Delete", "Delete node (no clipboard)"),
            ("y", "Yank node (copy)"),
            ("Y", "Yank children (copy)"),
            ("p", "Paste as children"),
            ("P", "Paste as siblings"),
            ("", ""),
            ("Collapse / Expand", ""),
            ("Space", "Toggle collapse"),
            ("v", "Collapse all (level 1)"),
            ("V", "Collapse children"),
            ("b", "Expand all"),
            ("f", "Focus on active node"),
            ("r", "Collapse other branches"),
            ("R", "Collapse inner"),
            ("1-9", "Collapse to level N"),
            ("", ""),
            ("Marks", ""),
            ("t", "Toggle ✓ / ✗ / none"),
            ("#", "Toggle numbering"),
            ("+/-", "Modify positive rank"),
            ("_/=", "Modify negative rank"),
            ("", ""),
            ("Move / Sort", ""),
            ("J / K", "Move node down/up"),
            ("T", "Sort siblings"),
            ("H", "Toggle hidden"),
            ("", ""),
            ("File / Other", ""),
            ("s", "Save"),
            ("S", "Save as"),
            ("x / X", "Export HTML / text to clipboard"),
            ("Ctrl+e", "Export ASCII to file"),
            ("u", "Undo"),
            ("Ctrl+r", "Redo"),
            ("Ctrl+o", "Open link (xdg-open)"),
            ("/", "Search"),
            ("n / N", "Next/prev search result"),
            ("w / W", "Increase/decrease node width"),
            ("z / Z", "Decrease/increase spacing"),
            ("q", "Quit (if saved)"),
            ("Q", "Force quit"),
            ("h / ?", "This help"),
        ];

        let mut lines: Vec<Line> = bindings
            .iter()
            .map(|(keys, desc)| {
                if desc.is_empty() {
                    if keys.is_empty() {
                        Line::raw("")
                    } else {
                        Line::from(Span::styled(
                            format!("── {} ──", keys),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ))
                    }
                } else {
                    Line::from(vec![
                        Span::styled(
                            format!("{:<18}", keys),
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(*desc),
                    ])
                }
            })
            .collect();

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Press any key to close",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));

        let widget = Paragraph::new(Text::from(lines))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .block(Block::bordered().title(" Keybindings (h-m-m compatible) "));

        let modal_area = Self::create_rect_area(72, 90, area);
        f.render_widget(Clear, modal_area);
        f.render_widget(widget, modal_area);
    }

    // ─── Utility ───────────────────────────────────────────────────

    fn create_rect_area(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let content_height = percent_y.min(r.height.saturating_sub(2));
        let vertical_margin = (r.height.saturating_sub(content_height)) / 2;

        let popup_layout = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Length(vertical_margin),
            ratatui::layout::Constraint::Length(content_height),
            ratatui::layout::Constraint::Min(vertical_margin),
        ])
        .split(r);

        ratatui::layout::Layout::horizontal([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Min(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
    }
}
