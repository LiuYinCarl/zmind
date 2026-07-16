use std::collections::HashMap;

mod display;
mod edit;
mod nav;
mod undo;

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
    pub(crate) fn new(title: String, parent: usize) -> Self {
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
    pub lines: usize,
}

/// The entire mind map state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MindMap {
    pub name: String,
    pub nodes: HashMap<usize, Node>,
    pub root_id: usize,
    pub active_node: usize,

    #[serde(skip)]
    pub(crate) undo_stack: Vec<UndoSnapshot>,
    #[serde(skip)]
    pub(crate) redo_stack: Vec<UndoSnapshot>,

    #[serde(skip)]
    pub clipboard: Option<String>,

    #[serde(skip)]
    pub visible_nodes: Vec<usize>,

    #[serde(skip)]
    pub layouts: HashMap<usize, NodeLayout>,
    #[serde(skip)]
    pub map_width: usize,
    #[serde(skip)]
    pub map_height: usize,

    #[serde(skip)]
    pub canvas: Vec<Vec<char>>,
    #[serde(skip)]
    pub canvas_col_widths: Vec<usize>,
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
pub(crate) struct UndoSnapshot {
    pub nodes: HashMap<usize, Node>,
    pub root_id: usize,
    pub active_node: usize,
}

impl MindMap {
    pub(crate) const MAX_UNDO: usize = 50;

    /// Create an empty mind map with a single root node.
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
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
            name: String::new(),
            nodes,
            root_id: 1,
            active_node: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: None,
            visible_nodes: Vec::new(),
            layouts: HashMap::new(),
            map_width: 0,
            map_height: 0,
            canvas: Vec::new(),
            canvas_col_widths: Vec::new(),
            max_node_width: 40,
            line_spacing: 1,
            show_hidden: false,
            align_levels: false,
        };
        mm.refresh_display();
        mm
    }

    pub fn new_named(name: String) -> Self {
        let mut mm = Self::new();
        mm.name = name;
        mm
    }
}

#[cfg(test)]
mod tests;
