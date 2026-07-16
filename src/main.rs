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
mod store;
mod view;

use cli::Cli;
use mind_map::MindMap;
use store::Store;
use view::View;

#[derive(Default, PartialEq, Debug, Clone)]
pub enum ViewMode {
    #[default]
    MapList,
    ViewMindMap,
    EditNode,
    RenameMap,
    Search,
    ConfirmQuit,
    Message,
    ViewHelp,
    ViewNodeDetails,
    EditNodeNote,
}

pub struct App {
    pub mind_map: MindMap,
    pub maps: Vec<MindMap>,
    pub active_index: usize,
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
    map_list_state: ListState,
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
    let _cli = Cli::read();
    let terminal = init_terminal()?;
    App::setup().run(terminal)?;
    restore_terminal()?;
    Ok(())
}

impl App {
    fn auto_save(&self) {
        Store::save(&self.maps, self.active_index);
    }

    fn setup() -> Self {
        let (maps, active_index) = Store::load();
        let mind_map = maps[active_index].clone();
        let active = mind_map.active_node;
        let mut map_list_state = ListState::default();
        if !maps.is_empty() {
            map_list_state.select(Some(active_index));
        }

        App {
            mind_map,
            maps,
            active_index,
            view_mode: ViewMode::MapList,
            previous_view_mode: ViewMode::MapList,
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
            symbol1: "\u{2713}".to_string(),
            symbol2: "\u{2717}".to_string(),
            prev_active: active,
            map_list_state,
        }
    }

    fn sync_maps(&mut self) {
        self.maps[self.active_index] = self.mind_map.clone();
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
                        // ─── Map List ─────────────────────────
                        ViewMode::MapList => match key.code {
                            Char('q') => return Ok(()),
                            Char('n') => {
                                input = input.clone().with_value(String::new());
                                self.previous_view_mode = ViewMode::MapList;
                                self.view_mode = ViewMode::RenameMap;
                            }
                            Enter => {
                                if let Some(idx) = self.map_list_state.selected() {
                                    if idx < self.maps.len() {
                                        self.mind_map = self.maps[idx].clone();
                                        self.active_index = idx;
                                        self.prev_active = self.mind_map.active_node;
                                        self.sync_config();
                                        self.view_mode = ViewMode::ViewMindMap;
                                    }
                                }
                            }
                            Char('r') => {
                                if let Some(idx) = self.map_list_state.selected() {
                                    if idx < self.maps.len() {
                                        input =
                                            input.clone().with_value(self.maps[idx].name.clone());
                                        self.previous_view_mode = ViewMode::MapList;
                                        self.view_mode = ViewMode::RenameMap;
                                    }
                                }
                            }
                            Char('d') => {
                                if self.maps.len() > 1 {
                                    if let Some(idx) = self.map_list_state.selected() {
                                        if idx < self.maps.len() {
                                            self.maps.remove(idx);
                                            if self.active_index >= self.maps.len() {
                                                self.active_index = self.maps.len() - 1;
                                            }
                                            self.map_list_state.select(Some(self.active_index));
                                            self.auto_save();
                                        }
                                    }
                                }
                            }
                            Down | Char('j') => {
                                let len = self.maps.len();
                                if len > 0 {
                                    let i = self.map_list_state.selected().unwrap_or(0);
                                    self.map_list_state.select(Some((i + 1) % len));
                                }
                            }
                            Up | Char('k') => {
                                let len = self.maps.len();
                                if len > 0 {
                                    let i = self.map_list_state.selected().unwrap_or(0);
                                    self.map_list_state.select(Some(if i == 0 {
                                        len - 1
                                    } else {
                                        i - 1
                                    }));
                                }
                            }
                            _ => {}
                        },

                        // ─── Rename Map ───────────────────────
                        ViewMode::RenameMap => match key.code {
                            Enter => {
                                let name = input.value().to_string();
                                input.reset();
                                if !name.is_empty() {
                                    if self.previous_view_mode == ViewMode::MapList {
                                        let mut mm = MindMap::new_named(name.clone());
                                        mm.refresh_display();
                                        self.maps.push(mm);
                                        self.active_index = self.maps.len() - 1;
                                        self.mind_map = self.maps[self.active_index].clone();
                                        self.map_list_state.select(Some(self.active_index));
                                        self.auto_save();
                                        self.view_mode = ViewMode::ViewMindMap;
                                    } else {
                                        self.mind_map.name = name;
                                        self.sync_maps();
                                        self.auto_save();
                                        self.view_mode = ViewMode::MapList;
                                    }
                                }
                            }
                            Esc => {
                                input.reset();
                                self.view_mode = ViewMode::MapList;
                            }
                            _ => {
                                input.handle_event(&Event::Key(key));
                            }
                        },

                        // ─── Mind Map ─────────────────────────
                        ViewMode::ViewMindMap => {
                            match key.code {
                                Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    self.viewport_x = self.viewport_x.saturating_sub(4);
                                }
                                Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    self.viewport_x = self.viewport_x.saturating_add(4);
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
                                        self.sync_maps();
                                        self.auto_save();
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
                                        self.sync_maps();
                                        self.auto_save();
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

                                Char('c') => {
                                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                                        return Ok(());
                                    } else {
                                        self.center_on_active();
                                    }
                                }
                                Char('C') => {
                                    self.center_lock = !self.center_lock;
                                    if self.center_lock {
                                        self.center_on_active();
                                    }
                                }
                                Char('F') => {
                                    self.focus_lock = !self.focus_lock;
                                    if self.focus_lock {
                                        self.mind_map.focus();
                                    }
                                }
                                Char('|') => {
                                    self.align_levels = !self.align_levels;
                                    self.sync_config();
                                }

                                Char('e') => {
                                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                                        let ascii = self.mind_map.export_ascii();
                                        match std::fs::write("zmind_export.txt", &ascii) {
                                            Ok(()) => {
                                                self.show_message("Exported to zmind_export.txt")
                                            }
                                            Err(e) => {
                                                self.show_message(&format!("Export error: {}", e))
                                            }
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
                                Char('o') => {
                                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                                        let _ = std::process::Command::new("xdg-open")
                                            .arg(&self.get_active_title())
                                            .spawn();
                                    } else {
                                        self.mind_map.insert_sibling();
                                        self.sync_maps();
                                        self.auto_save();
                                    }
                                }
                                Enter => {
                                    self.mind_map.insert_sibling();
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('O') | Tab => {
                                    self.mind_map.insert_child();
                                    self.sync_maps();
                                    self.auto_save();
                                }

                                Char('d') => {
                                    self.mind_map.cut_node();
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('D') => {
                                    self.mind_map.cut_children();
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Delete => {
                                    self.mind_map.delete_node_no_clipboard();
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('y') => {
                                    self.mind_map.yank_node();
                                }
                                Char('Y') => {
                                    self.mind_map.yank_children();
                                }
                                Char('p') => {
                                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                                        self.mind_map.append_clipboard_to_title();
                                    } else {
                                        self.mind_map.paste_as_children();
                                    }
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('P') => {
                                    self.mind_map.paste_as_siblings();
                                    self.sync_maps();
                                    self.auto_save();
                                }

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
                                    self.mind_map.collapse_level((c as u8 - b'0') as usize);
                                }

                                Char('t') => {
                                    let s1 = self.symbol1.clone();
                                    let s2 = self.symbol2.clone();
                                    self.mind_map.toggle_symbol(&s1, &s2);
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('#') => {
                                    self.mind_map.toggle_numbers();
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('+') => {
                                    self.mind_map.modify_positive_rank(-1);
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('=') => {
                                    self.mind_map.modify_positive_rank(1);
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('-') => {
                                    self.mind_map.modify_negative_rank(1);
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('_') => {
                                    self.mind_map.modify_negative_rank(-1);
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('H') => {
                                    self.mind_map.toggle_hide();
                                    self.sync_maps();
                                    self.auto_save();
                                }

                                Char('J') => {
                                    self.mind_map.move_node_down();
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('K') => {
                                    self.mind_map.move_node_up();
                                    self.sync_maps();
                                    self.auto_save();
                                }
                                Char('T') => {
                                    self.mind_map.sort_siblings();
                                    self.sync_maps();
                                    self.auto_save();
                                }

                                Char('x') => {
                                    self.export_html();
                                }
                                Char('X') => {
                                    self.mind_map.clipboard = Some(self.mind_map.to_text());
                                }

                                Char('u') => {
                                    self.mind_map.undo();
                                    self.sync_maps();
                                    self.auto_save();
                                }

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

                                Char('w') => {
                                    self.mind_map.max_node_width =
                                        (self.mind_map.max_node_width as f32 * 1.2) as usize;
                                    self.sync_config();
                                }
                                Char('W') => {
                                    self.mind_map.max_node_width =
                                        (self.mind_map.max_node_width as f32 / 1.2).max(10.0)
                                            as usize;
                                    self.sync_config();
                                }
                                Char('z') => {
                                    self.mind_map.line_spacing =
                                        self.mind_map.line_spacing.saturating_sub(1);
                                    self.sync_config();
                                }
                                Char('Z') => {
                                    self.mind_map.line_spacing += 1;
                                    self.sync_config();
                                }

                                Char('q') => {
                                    self.sync_maps();
                                    self.auto_save();
                                    self.view_mode = ViewMode::MapList;
                                }
                                Char('Q') => {
                                    return Ok(());
                                }
                                Esc => {
                                    self.sync_maps();
                                    self.auto_save();
                                    self.view_mode = ViewMode::MapList;
                                }

                                Char('?') => {
                                    self.previous_view_mode = ViewMode::ViewMindMap;
                                    self.view_mode = ViewMode::ViewHelp;
                                }

                                _ => {}
                            }
                            self.apply_locks();
                        }

                        ViewMode::EditNode => match key.code {
                            Enter => {
                                self.mind_map.edit_node(input.value().to_string());
                                self.sync_maps();
                                self.auto_save();
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
                                if !self.search_results.is_empty() {
                                    self.search_index = 0;
                                    self.mind_map.active_node = self.search_results[0];
                                }
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

                        ViewMode::ConfirmQuit => match key.code {
                            Char('y') | Char('Y') => return Ok(()),
                            _ => self.view_mode = ViewMode::ViewMindMap,
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
                                    self.mind_map
                                        .nodes
                                        .get(&self.mind_map.active_node)
                                        .map(|n| n.note.clone())
                                        .unwrap_or_default(),
                                );
                                self.previous_view_mode = ViewMode::ViewNodeDetails;
                                self.view_mode = ViewMode::EditNodeNote;
                            }
                            _ => self.view_mode = ViewMode::ViewMindMap,
                        },

                        ViewMode::EditNodeNote => match key.code {
                            Enter => {
                                self.mind_map.update_note(input.value().to_string());
                                self.sync_maps();
                                self.auto_save();
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

        match self.view_mode {
            ViewMode::MapList | ViewMode::RenameMap => {
                View::show_map_list_hint(f, header_area);
                View::show_map_list(self, f, main_area);
            }
            _ => {
                View::show_hint_bar(f, header_area);
                if self.view_mode == ViewMode::ViewHelp {
                    View::show_help_modal(f, area);
                } else if self.view_mode == ViewMode::ViewNodeDetails {
                    View::show_node_details_modal(self, f, area);
                } else {
                    View::show_mind_map(self, f, main_area);
                }
                View::show_status_bar(self, f, footer_area);
            }
        }

        match self.view_mode {
            ViewMode::EditNode => View::show_edit_modal(f, area, input, input.value().is_empty()),
            ViewMode::EditNodeNote => View::show_edit_note_modal(f, area, input),
            ViewMode::Search => View::show_search_modal(f, area, input),
            ViewMode::RenameMap => View::show_rename_map_modal(f, area, input),
            ViewMode::ConfirmQuit => View::show_confirm_modal(f, area, &self.message),
            ViewMode::Message => View::show_message_modal(f, area, &self.message),
            _ => {}
        }
    }

    fn show_message(&mut self, msg: &str) {
        self.message = msg.to_string();
        self.previous_view_mode = self.view_mode.clone();
        self.view_mode = ViewMode::Message;
    }

    fn get_active_title(&self) -> String {
        self.mind_map
            .nodes
            .get(&self.mind_map.active_node)
            .map(|n| n.title.clone())
            .unwrap_or_default()
    }

    fn do_search(&mut self) {
        self.search_results.clear();
        let term = self.search_term.to_lowercase();
        for (&id, node) in &self.mind_map.nodes {
            if node.title.to_lowercase().contains(&term) && !node.hidden {
                self.search_results.push(id);
            }
        }
        self.search_results.sort();
    }

    fn center_on_active(&mut self) {
        if let Some(layout) = self.mind_map.layouts.get(&self.mind_map.active_node) {
            self.viewport_x = layout.x.saturating_sub(20);
            self.viewport_y = layout.y.saturating_sub(10);
        }
    }

    fn sync_config(&mut self) {
        self.mind_map.show_hidden = self.show_hidden;
        self.mind_map.align_levels = self.align_levels;
        self.mind_map.refresh_display();
    }

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
        let _ = std::fs::write("zmind_export.html", &html);
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
        self.search_index = if self.search_index == 0 {
            self.search_results.len() - 1
        } else {
            self.search_index - 1
        };
        self.mind_map.active_node = self.search_results[self.search_index];
    }
}
