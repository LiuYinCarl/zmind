use std::{
    error::Error,
    io::{self, stdout},
};

use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    prelude::*,
    widgets::*,
};
use tui_input::{backend::crossterm::EventHandler as _, Input};

mod cli;
mod mind_map;
mod view;

use cli::Cli;
use mind_map::MindMap;
use view::View;

#[derive(Default, PartialEq, Debug, Clone)]
pub enum ViewMode {
    FilePicker,
    #[default]
    ViewMindMap,
    EditNode,
    Search,
    SaveAs,
    ConfirmQuit,
    Message,
    ViewHelp,
    ViewNodeDetails,
    EditNodeNote,
}

pub struct App {
    pub mind_map: MindMap,
    pub view_mode: ViewMode,
    pub previous_view_mode: ViewMode,
    pub message: String,
    pub search_term: String,
    pub search_results: Vec<usize>,
    pub search_index: usize,
    pub viewport_x: usize,
    pub viewport_y: usize,
    pub focus_lock: bool,
    pub center_lock: bool,
    pub show_hidden: bool,
    pub align_levels: bool,
    pub symbol1: String,
    pub symbol2: String,
    prev_active: usize,
    file_list: Vec<std::path::PathBuf>,
    file_list_state: ListState,
}

fn init_terminal() -> Result<Terminal<impl Backend>, Box<dyn Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::read();

    // Setup terminal
    let terminal = init_terminal()?;

    // Create app and run
    App::setup(cli).run(terminal)?;

    restore_terminal()?;

    Ok(())
}

impl App {
    fn setup(cli: Cli) -> Self {
        let (mind_map, start_view) = if let Some(ref path) = cli.filename {
            let mm = MindMap::from_file(path).unwrap_or_else(|_| {
                let mut mm = MindMap::new();
                mm.filename = Some(path.clone());
                mm
            });
            (mm, ViewMode::ViewMindMap)
        } else {
            (MindMap::new(), ViewMode::FilePicker)
        };

        let active = mind_map.active_node;
        let file_list = Self::scan_hmm_files();
        let mut file_list_state = ListState::default();
        if !file_list.is_empty() {
            file_list_state.select(Some(0));
        }

        App {
            mind_map,
            view_mode: start_view,
            previous_view_mode: ViewMode::FilePicker,
            message: String::new(),
            search_term: String::new(),
            search_results: Vec::new(),
            search_index: 0,
            viewport_x: 0,
            viewport_y: 0,
            focus_lock: false,
            center_lock: false,
            show_hidden: false,
            align_levels: false,
            symbol1: "✓".to_string(),
            symbol2: "✗".to_string(),
            prev_active: active,
            file_list,
            file_list_state,
        }
    }

    fn scan_hmm_files() -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
    }

    fn run(&mut self, mut terminal: Terminal<impl Backend>) -> io::Result<()> {
        let mut input = Input::default();

        loop {
            terminal.draw(|f| {
                self.render(f, f.size(), &input);
            })?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    use KeyCode::*;
                    match self.view_mode {
                        ViewMode::FilePicker => match key.code {
                            Char('q') => return Ok(()),
                            Char('n') => {
                                self.mind_map = MindMap::new();
                                self.view_mode = ViewMode::ViewMindMap;
                            }
                            Enter => {
                                if let Some(idx) = self.file_list_state.selected() {
                                    if idx == 0 {
                                        // "New mind map" option
                                        self.mind_map = MindMap::new();
                                        self.view_mode = ViewMode::ViewMindMap;
                                    } else if (idx - 1) < self.file_list.len() {
                                        let path = &self.file_list[idx - 1];
                                        match MindMap::from_file(path) {
                                            Ok(mm) => {
                                                self.mind_map = mm;
                                                self.view_mode = ViewMode::ViewMindMap;
                                            }
                                            Err(e) => {
                                                self.show_message(&format!("Error: {}", e));
                                            }
                                        }
                                    }
                                }
                            }
                            Down | Char('j') => {
                                let len = self.file_list.len() + 1; // +1 for "New"
                                if len > 0 {
                                    let i = self.file_list_state.selected().unwrap_or(0);
                                    self.file_list_state.select(Some((i + 1) % len));
                                }
                            }
                            Up | Char('k') => {
                                let len = self.file_list.len() + 1;
                                if len > 0 {
                                    let i = self.file_list_state.selected().unwrap_or(0);
                                    self.file_list_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
                                }
                            }
                            _ => {}
                        },

                        ViewMode::ViewMindMap => {
                            match key.code {
                            // ─── Navigation ──────────────────────
                            Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.viewport_x = self.viewport_x.saturating_sub(4);
                            }
                            Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.viewport_x = self.viewport_x.saturating_add(4);
                                // Clamp
                                let max_x = self.mind_map.map_width.saturating_sub(10);
                                if self.viewport_x > max_x {
                                    self.viewport_x = max_x;
                                }
                            }
                            Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.viewport_y = self.viewport_y.saturating_sub(1);
                            }
                            Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.viewport_y = self.viewport_y.saturating_add(1);
                                let max_y = self.mind_map.map_height.saturating_sub(5);
                                if self.viewport_y > max_y {
                                    self.viewport_y = max_y;
                                }
                            }
                            Char('h') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    self.show_hidden = !self.show_hidden;
                                    self.sync_config();
                                    self.show_message(&format!("Show hidden: {}", if self.show_hidden { "ON" } else { "OFF" }));
                                } else {
                                    self.previous_view_mode = ViewMode::ViewMindMap;
                                    self.view_mode = ViewMode::ViewHelp;
                                }
                            }
                            Left => {
                                self.mind_map.go_left();
                            }
                            Char('l') | Right => {
                                self.mind_map.go_right();
                            }
                            Char('j') => {
                                if key.modifiers.contains(KeyModifiers::ALT) {
                                    self.mind_map.remove_star();
                                } else {
                                    self.mind_map.go_down();
                                }
                            }
                            Down => {
                                self.mind_map.go_down();
                            }
                            Char('k') => {
                                if key.modifiers.contains(KeyModifiers::ALT) {
                                    self.mind_map.add_star();
                                } else {
                                    self.mind_map.go_up();
                                }
                            }
                            Up => {
                                self.mind_map.go_up();
                            }
                            Char('g') => {
                                self.mind_map.go_to_top();
                            }
                            Char('G') => {
                                self.mind_map.go_to_bottom();
                            }
                            Char('m') | Char('~') => {
                                self.mind_map.go_to_root();
                            }

                            // ─── Viewport ───────────────────────
                            Char('c') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    return Ok(());
                                } else {
                                    self.center_on_active();
                                }
                            }
                            Char('C') => {
                                self.center_lock = !self.center_lock;
                                self.show_message(&format!("Center lock: {}", if self.center_lock { "ON" } else { "OFF" }));
                                if self.center_lock {
                                    self.center_on_active();
                                }
                            }
                            Char('F') => {
                                self.focus_lock = !self.focus_lock;
                                self.show_message(&format!("Focus lock: {}", if self.focus_lock { "ON" } else { "OFF" }));
                                if self.focus_lock {
                                    self.mind_map.focus();
                                }
                            }
                            Char('|') => {
                                self.align_levels = !self.align_levels;
                                self.sync_config();
                                self.show_message(&format!("Aligned levels: {}", if self.align_levels { "ON" } else { "OFF" }));
                            }

                            // ─── Editing ──────────────────────────
                            Char('o') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    let title = self.get_active_title();
                                    let _ = std::process::Command::new("xdg-open")
                                        .arg(&title)
                                        .spawn();
                                } else {
                                    self.mind_map.insert_sibling();
                                }
                            }
                            Enter => {
                                self.mind_map.insert_sibling();
                            }
                            Char('O') | Tab => {
                                self.mind_map.insert_child();
                            }
                            Char('e') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    let ascii = self.mind_map.export_ascii();
                                    let path = std::path::PathBuf::from("zmind_export.txt");
                                    match std::fs::write(&path, &ascii) {
                                        Ok(()) => self.show_message(&format!("ASCII exported to {}", path.display())),
                                        Err(er) => self.show_message(&format!("Export error: {}", er)),
                                    }
                                } else {
                                    input = input.clone().with_value(self.get_active_title());
                                    self.previous_view_mode = ViewMode::ViewMindMap;
                                    self.view_mode = ViewMode::EditNode;
                                }
                            }
                            Char('a') => {
                                input = input.clone().with_value(self.get_active_title());
                                self.previous_view_mode = ViewMode::ViewMindMap;
                                self.view_mode = ViewMode::EditNode;
                            }
                            Char('E') | Char('A') => {
                                input.reset();
                                self.previous_view_mode = ViewMode::ViewMindMap;
                                self.view_mode = ViewMode::EditNode;
                            }
                            Char('i') | Char('I') => {
                                self.previous_view_mode = ViewMode::ViewMindMap;
                                self.view_mode = ViewMode::ViewNodeDetails;
                            }

                            // ─── Delete / Clipboard ──────────────
                            Char('d') => {
                                self.mind_map.cut_node();
                            }
                            Char('D') => {
                                self.mind_map.cut_children();
                            }
                            Delete => {
                                self.mind_map.delete_node_no_clipboard();
                            }
                            Char('y') => {
                                self.mind_map.yank_node();
                                self.show_message("Node yanked (copied)");
                            }
                            Char('Y') => {
                                self.mind_map.yank_children();
                                self.show_message("Children yanked (copied)");
                            }
                            Char('p') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    self.mind_map.append_clipboard_to_title();
                                } else {
                                    self.mind_map.paste_as_children();
                                }
                            }
                            Char('P') => {
                                self.mind_map.paste_as_siblings();
                            }

                            // ─── Collapse / Expand ───────────────
                            Char(' ') => {
                                self.mind_map.toggle_node();
                            }
                            Char('v') => {
                                self.mind_map.collapse_all();
                            }
                            Char('V') => {
                                self.mind_map.collapse_children();
                            }
                            Char('b') => {
                                self.mind_map.expand_all();
                            }
                            Char('f') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    input.reset();
                                    self.search_results.clear();
                                    self.search_index = 0;
                                    self.search_term.clear();
                                    self.previous_view_mode = ViewMode::ViewMindMap;
                                    self.view_mode = ViewMode::Search;
                                } else {
                                    self.mind_map.focus();
                                }
                            }
                            Char('r') => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    self.mind_map.redo();
                                } else {
                                    self.mind_map.collapse_other_branches();
                                }
                            }
                            Char('R') => {
                                self.mind_map.collapse_inner();
                            }
                            Char(c @ '1'..='9') => {
                                let depth = (c as u8 - b'0') as usize;
                                self.mind_map.collapse_level(depth);
                            }

                            // ─── Marks ───────────────────────────
                            Char('t') => {
                                self.mind_map.toggle_symbol(
                                    &self.symbol1,
                                    &self.symbol2,
                                );
                            }
                            Char('#') => {
                                self.mind_map.toggle_numbers();
                            }
                            Char('+') => {
                                self.mind_map.modify_positive_rank(-1);
                            }
                            Char('=') => {
                                self.mind_map.modify_positive_rank(1);
                            }
                            Char('-') => {
                                self.mind_map.modify_negative_rank(1);
                            }
                            Char('_') => {
                                self.mind_map.modify_negative_rank(-1);
                            }
                            Char('H') => {
                                self.mind_map.toggle_hide();
                            }

                            // ─── Move / Sort ─────────────────────
                            Char('J') => {
                                self.mind_map.move_node_down();
                            }
                            Char('K') => {
                                self.mind_map.move_node_up();
                            }
                            Char('T') => {
                                self.mind_map.sort_siblings();
                            }

                            // ─── File ─────────────────────────────
                            Char('s') => {
                                if self.mind_map.filename.is_some() {
                                    match self.mind_map.save() {
                                        Ok(()) => self.show_message("Saved"),
                                        Err(e) => self.show_message(&format!("Save error: {}", e)),
                                    }
                                } else {
                                    input =
                                        input.clone().with_value(String::new());
                                    self.previous_view_mode = ViewMode::ViewMindMap;
                                    self.view_mode = ViewMode::SaveAs;
                                }
                            }
                            Char('S') => {
                                input = input.clone().with_value(String::new());
                                self.previous_view_mode = ViewMode::ViewMindMap;
                                self.view_mode = ViewMode::SaveAs;
                            }
                            Char('x') => {
                                self.export_html();
                            }
                            Char('X') => {
                                let text = self.mind_map.to_text();
                                self.mind_map.clipboard = Some(text);
                                self.show_message("Text map copied to clipboard");
                            }

                            // ─── Undo ─────────────────────────────
                            Char('u') => {
                                self.mind_map.undo();
                            }

                            // ─── Search ───────────────────────────
                            Char('/') => {
                                input.reset();
                                self.search_results.clear();
                                self.search_index = 0;
                                self.search_term.clear();
                                self.previous_view_mode = ViewMode::ViewMindMap;
                                self.view_mode = ViewMode::Search;
                            }
                            Char('n') => {
                                self.next_search_result();
                            }
                            Char('N') => {
                                self.prev_search_result();
                            }

                            // ─── Width / Spacing ─────────────────
                            Char('w') => {
                                self.mind_map.max_node_width =
                                    (self.mind_map.max_node_width as f32 * 1.2) as usize;
                                self.sync_config();
                                self.show_message(&format!(
                                    "Width: {}",
                                    self.mind_map.max_node_width,
                                ));
                            }
                            Char('W') => {
                                self.mind_map.max_node_width =
                                    (self.mind_map.max_node_width as f32 / 1.2)
                                        .max(10.0) as usize;
                                self.sync_config();
                                self.show_message(&format!(
                                    "Width: {}",
                                    self.mind_map.max_node_width,
                                ));
                            }
                            Char('z') => {
                                self.mind_map.line_spacing =
                                    self.mind_map.line_spacing.saturating_sub(1);
                                self.sync_config();
                                self.show_message(&format!(
                                    "Spacing: {}",
                                    self.mind_map.line_spacing
                                ));
                            }
                            Char('Z') => {
                                self.mind_map.line_spacing += 1;
                                self.sync_config();
                                self.show_message(&format!(
                                    "Spacing: {}",
                                    self.mind_map.line_spacing
                                ));
                            }

                            // ─── Quit ─────────────────────────────
                            Char('q') => {
                                if self.mind_map.modified {
                                    self.previous_view_mode = ViewMode::ViewMindMap;
                                    self.view_mode = ViewMode::ConfirmQuit;
                                    self.message =
                                        "Unsaved changes! Press y to confirm quit.".to_string();
                                } else {
                                    return Ok(());
                                }
                            }
                            Char('Q') => {
                                return Ok(());
                            }

                            // ─── Help ─────────────────────────────
                            Char('?') => {
                                self.previous_view_mode = ViewMode::ViewMindMap;
                                self.view_mode = ViewMode::ViewHelp;
                            }
                            Esc => {
                                if self.mind_map.modified {
                                    self.previous_view_mode = ViewMode::ViewMindMap;
                                    self.view_mode = ViewMode::ConfirmQuit;
                                    self.message =
                                        "Unsaved changes! Press y to confirm, any other key to cancel.".to_string();
                                } else {
                                    self.file_list = Self::scan_hmm_files();
                                    self.view_mode = ViewMode::FilePicker;
                                }
                            }

                            _ => {}
                            }
                            self.apply_locks();
                        }

                        ViewMode::EditNode => match key.code {
                            Enter => {
                                self.mind_map.edit_node(input.value().to_string());
                                input.reset();
                                self.view_mode = ViewMode::ViewMindMap;
                            }
                            Esc => {
                                input.reset();
                                self.view_mode = ViewMode::ViewMindMap;
                            }
                            _ => {
                                input.handle_event(&Event::Key(key));
                            }
                        },

                        ViewMode::Search => match key.code {
                            Enter => {
                                self.search_term = input.value().to_string();
                                self.do_search();
                                input.reset();
                                if self.search_results.is_empty() {
                                    self.show_message(&format!(
                                        "No results for: {}",
                                        self.search_term
                                    ));
                                } else {
                                    self.search_index = 0;
                                    self.mind_map.active_node =
                                        self.search_results[0];
                                    self.view_mode = ViewMode::ViewMindMap;
                                }
                            }
                            Esc => {
                                input.reset();
                                self.view_mode = ViewMode::ViewMindMap;
                            }
                            _ => {
                                input.handle_event(&Event::Key(key));
                            }
                        },

                        ViewMode::SaveAs => match key.code {
                            Enter => {
                                let mut path = std::path::PathBuf::from(input.value());
                                if path.extension().is_none() {
                                    path.set_extension("json");
                                }
                                match self.mind_map.save_as(path) {
                                    Ok(()) => {
                                        self.file_list = Self::scan_hmm_files();
                                        self.view_mode = ViewMode::ViewMindMap;
                                        self.show_message("Saved");
                                    }
                                    Err(e) => {
                                        self.view_mode = ViewMode::ViewMindMap;
                                        self.show_message(&format!("Save error: {}", e));
                                    }
                                }
                                input.reset();
                            }
                            Esc => {
                                input.reset();
                                self.view_mode = ViewMode::ViewMindMap;
                            }
                            _ => {
                                input.handle_event(&Event::Key(key));
                            }
                        },

                        ViewMode::ConfirmQuit => match key.code {
                            Char('y') | Char('Y') => {
                                return Ok(());
                            }
                            _ => {
                                self.view_mode = ViewMode::ViewMindMap;
                            }
                        },

                        ViewMode::Message => {
                            self.view_mode = self.previous_view_mode.clone();
                        }

                        ViewMode::ViewHelp => {
                            self.view_mode = self.previous_view_mode.clone();
                        }

                        ViewMode::ViewNodeDetails => match key.code {
                            Char('e') => {
                                input = input.clone().with_value(
                                    self.mind_map.nodes.get(&self.mind_map.active_node)
                                        .map(|n| n.note.clone()).unwrap_or_default()
                                );
                                self.previous_view_mode = ViewMode::ViewNodeDetails;
                                self.view_mode = ViewMode::EditNodeNote;
                            }
                            _ => {
                                self.view_mode = ViewMode::ViewMindMap;
                            }
                        },

                        ViewMode::EditNodeNote => match key.code {
                            Enter => {
                                self.mind_map.update_note(input.value().to_string());
                                input.reset();
                                self.view_mode = ViewMode::ViewNodeDetails;
                            }
                            Esc => {
                                input.reset();
                                self.view_mode = ViewMode::ViewNodeDetails;
                            }
                            _ => {
                                input.handle_event(&Event::Key(key));
                            }
                        },
                    }
                }
            }
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, input: &Input) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ]);

        let [header_area, main_area, footer_area] = layout.areas(area);

        // Header
        if self.view_mode == ViewMode::FilePicker {
            View::show_file_picker_hint(f, header_area);
        } else {
            View::show_hint_bar(f, header_area);
        }

        // Main view
        if self.view_mode == ViewMode::FilePicker {
            View::show_file_picker(self, f, main_area);
        } else if self.view_mode == ViewMode::ViewHelp {
            View::show_help_modal(f, area);
        } else if self.view_mode == ViewMode::ViewNodeDetails {
            View::show_node_details_modal(self, f, area);
        } else {
            View::show_mind_map(self, f, main_area);
        }

        // Footer
        View::show_status_bar(self, f, footer_area);

        // Overlay modals
        match self.view_mode {
            ViewMode::EditNode => {
                let replace = input.value().is_empty();
                View::show_edit_modal(f, area, input, replace);
            }
            ViewMode::EditNodeNote => {
                View::show_edit_note_modal(f, area, input);
            }
            ViewMode::Search => {
                View::show_search_modal(f, area, input);
            }
            ViewMode::SaveAs => {
                View::show_save_as_modal(f, area, input);
            }
            ViewMode::ConfirmQuit => {
                View::show_confirm_modal(f, area, &self.message);
            }
            ViewMode::Message => {
                View::show_message_modal(f, area, &self.message);
            }
            _ => {}
        }
    }

    // ─── Helpers ─────────────────────────────────────────────────

    fn get_active_title(&self) -> String {
        self.mind_map
            .nodes
            .get(&self.mind_map.active_node)
            .map(|n| n.title.clone())
            .unwrap_or_default()
    }

    fn show_message(&mut self, msg: &str) {
        self.message = msg.to_string();
        self.previous_view_mode = self.view_mode.clone();
        self.view_mode = ViewMode::Message;
    }

    fn do_search(&mut self) {
        self.search_results.clear();
        let term = self.search_term.to_lowercase();

        for (&id, node) in &self.mind_map.nodes {
            if node.title.to_lowercase().contains(&term) && !node.hidden {
                self.search_results.push(id);
            }
        }

        // Sort by node id (which roughly corresponds to creation order)
        self.search_results.sort();
    }

    fn center_on_active(&mut self) {
        if let Some(layout) = self.mind_map.layouts.get(&self.mind_map.active_node) {
            // Use approximate terminal dimensions
            // These will be refined when we have actual terminal dimensions from render
            self.viewport_x = layout.x.saturating_sub(20);
            self.viewport_y = layout.y.saturating_sub(10);
        }
    }

    fn sync_config(&mut self) {
        self.mind_map.show_hidden = self.show_hidden;
        self.mind_map.align_levels = self.align_levels;
        self.mind_map.refresh_display();
    }

    /// Call after any active node change to apply locks.
    fn apply_locks(&mut self) {
        if self.mind_map.active_node == self.prev_active {
            return;
        }
        self.prev_active = self.mind_map.active_node;
        if self.focus_lock {
            self.mind_map.focus();
        }
        if self.center_lock {
            self.center_on_active();
            self.mind_map.refresh_display();
        }
    }

    fn export_html(&mut self) {
        let html = self.mind_map.export_html();
        let path = match &self.mind_map.filename {
            Some(p) => {
                let mut new_path = p.clone();
                new_path.set_extension("html");
                new_path
            }
            None => std::path::PathBuf::from("zmind_export.html"),
        };
        match std::fs::write(&path, &html) {
            Ok(()) => self.show_message(&format!("Exported to {}", path.display())),
            Err(e) => self.show_message(&format!("Export error: {}", e)),
        }
    }

    fn next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_index = (self.search_index + 1) % self.search_results.len();
        self.mind_map.active_node = self.search_results[self.search_index];
    }

    fn prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        if self.search_index == 0 {
            self.search_index = self.search_results.len() - 1;
        } else {
            self.search_index -= 1;
        }
        self.mind_map.active_node = self.search_results[self.search_index];
    }
}
