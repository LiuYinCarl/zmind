use super::display::find_node_by_title;
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
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.insert_sibling();
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
    assert!(
        !parent.collapsed,
        "Parent should be uncollapsed after insert"
    );
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
    mm.cut_node();
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
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.yank_node();
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
    mm.toggle_node();
    let a_id = find_node_by_title(&mm, "A");
    assert!(mm.nodes[&a_id].collapsed, "Parent should be collapsed");
}

#[test]
fn test_expand_all() {
    let input = "root\n\tA\n\t\tA1\n\tB";
    let mut mm = MindMap::from_text(input);
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
    mm.collapse_level(2);
    let l2_id = find_node_by_title(&mm, "L2");
    assert!(
        mm.nodes[&l2_id].collapsed,
        "L2 should be collapsed at level 2"
    );
}

#[test]
fn test_focus() {
    let input = "root\n\tBranch1\n\t\tB1a\n\tBranch2\n\t\tB2a\n\t\tB2b";
    let mut mm = MindMap::from_text(input);
    mm.collapse_all();
    let b2a_id = find_node_by_title(&mm, "B2a");
    mm.active_node = b2a_id;
    mm.focus();
    let b2_id = find_node_by_title(&mm, "Branch2");
    assert!(
        !mm.nodes[&b2_id].collapsed,
        "Branch2 should be expanded by focus"
    );
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
    assert!(
        !mm.nodes[&b_id].collapsed,
        "B (active branch) should not be collapsed"
    );
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
    assert!(
        mm.nodes[&a1a_id].collapsed,
        "A1a should also be collapsed (deep collapse)"
    );
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
    mm.go_left();
    assert_eq!(mm.nodes[&mm.active_node].title, "A");
    mm.go_right();
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
    mm.go_up();
    assert_eq!(mm.nodes[&mm.active_node].title, "C");
}

#[test]
fn test_nav_down_wraps() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
    let c_id = find_node_by_title(&mm, "C");
    mm.active_node = c_id;
    mm.go_down();
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
    assert_eq!(
        mm.nodes[&root.children[0]].title, "B",
        "B should now be first"
    );
    assert_eq!(
        mm.nodes[&root.children[1]].title, "A",
        "A should now be second"
    );
}

#[test]
fn test_move_node_down() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
    let b_id = find_node_by_title(&mm, "B");
    mm.active_node = b_id;
    mm.move_node_down();
    let root = &mm.nodes[&mm.root_id];
    assert_eq!(
        mm.nodes[&root.children[2]].title, "B",
        "B should now be last"
    );
}

#[test]
fn test_sort_siblings() {
    let mut mm = MindMap::from_text("root\n\tC\n\tA\n\tB");
    let c_id = find_node_by_title(&mm, "C");
    mm.active_node = c_id;
    mm.sort_siblings();
    let root = &mm.nodes[&mm.root_id];
    let titles: Vec<&str> = root
        .children
        .iter()
        .map(|id| mm.nodes[id].title.as_str())
        .collect();
    assert_eq!(
        titles,
        vec!["A", "B", "C"],
        "Should be sorted alphabetically"
    );
}

#[test]
fn test_cannot_move_root() {
    let mut mm = MindMap::from_text("root\n\tA");
    mm.active_node = mm.root_id;
    let children_before = mm.nodes[&mm.root_id].children.clone();
    mm.move_node_up();
    mm.move_node_down();
    assert_eq!(
        mm.nodes[&mm.root_id].children, children_before,
        "Root should not move"
    );
}

// ═══ Marks ═════════════════════════════════════════════════

#[test]
fn test_toggle_symbol() {
    let mut mm = MindMap::from_text("root\n\tItem");
    let item_id = find_node_by_title(&mm, "Item");
    mm.active_node = item_id;
    mm.toggle_symbol("✓", "✗");
    assert!(
        mm.nodes[&item_id].title.starts_with("✓ "),
        "Should add checkmark"
    );
    mm.toggle_symbol("✓", "✗");
    assert!(
        mm.nodes[&item_id].title.starts_with("✗ "),
        "Should change to cross"
    );
    mm.toggle_symbol("✓", "✗");
    assert!(
        !mm.nodes[&item_id].title.starts_with("✓") && !mm.nodes[&item_id].title.starts_with("✗"),
        "Should remove symbol"
    );
}

#[test]
fn test_toggle_numbers() {
    let mut mm = MindMap::from_text("root\n\tApple\n\tBanana\n\tCherry");
    let apple_id = find_node_by_title(&mm, "Apple");
    mm.active_node = apple_id;
    mm.toggle_numbers();
    let children = mm.nodes[&mm.root_id].children.clone();
    for (i, &cid) in children.iter().enumerate() {
        let title = &mm.nodes[&cid].title;
        assert!(
            title.starts_with(&format!("{}.", i + 1)),
            "Should be numbered: {}",
            title
        );
    }
    mm.toggle_numbers();
    for &cid in &children {
        let title = &mm.nodes[&cid].title;
        assert!(
            !title.chars().next().unwrap().is_ascii_digit(),
            "Numbers should be removed: {}",
            title
        );
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
    assert!(
        mm.active_node != a_id,
        "Active node should move away from hidden"
    );
    mm.show_hidden = true;
    mm.refresh_display();
    assert!(
        mm.visible_nodes.contains(&a_id),
        "A should be visible when show_hidden=true"
    );
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
    assert_eq!(
        mm.nodes[&orig_id].title, original_title,
        "Should restore original"
    );
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
    assert_eq!(
        mm.nodes[&mm.root_id].children.len(),
        child_count,
        "Should revert insert"
    );
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
    for i in 0..60 {
        mm.insert_child();
        mm.edit_node(format!("Node {}", i));
    }
    assert!(
        mm.undo_stack.len() <= MindMap::MAX_UNDO,
        "Should not exceed MAX_UNDO"
    );
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
    assert!(
        html.contains("<details"),
        "Should use details for parent nodes"
    );
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
    assert!(
        count_after < count_before,
        "Collapsing should reduce visible nodes"
    );
}

// ═══ Golden-File Export Tests ═══════════════════════════════

/// JSON roundtrip + export, compare against expected string.
fn assert_export(tree: &str, expected: &str) {
    let mm0 = MindMap::from_text(tree);
    let json = serde_json::to_string(&mm0).unwrap();
    let mut mm: MindMap = serde_json::from_str(&json).unwrap();
    mm.line_spacing = 0;
    mm.refresh_display();
    assert_eq!(mm.export_ascii(), expected);
}

#[test]
fn test_export_simple() {
    assert_export(
        "root\n\tA\n\tB",
        concat!("root───┤\n", "       ├──A\n", "       ╰──B\n"),
    );
}

#[test]
fn test_export_single_child() {
    assert_export("root\n\tChild", concat!("root───╮\n", "       ╰──Child\n"));
}

#[test]
fn test_export_multi_branch() {
    assert_export(
        "root\n\tA\n\tB\n\tC",
        concat!(
            "root───┤\n",
            "       ├──A\n",
            "       ├──B\n",
            "       ╰──C\n"
        ),
    );
}

#[test]
fn test_export_nested() {
    assert_export(
        "root\n\tA\n\t\tA1\n\tB",
        concat!(
            "root───┤\n",
            "       ├──A───╮\n",
            "       │      ╰──A1\n",
            "       ╰──B\n",
        ),
    );
}

#[test]
fn test_export_collapsed() {
    let mm0 = MindMap::from_text("root\n\tParent\n\t\tHidden");
    let json = serde_json::to_string(&mm0).unwrap();
    let mut mm: MindMap = serde_json::from_str(&json).unwrap();
    mm.line_spacing = 0;
    let pid = find_node_by_title(&mm, "Parent");
    mm.active_node = pid;
    mm.toggle_node();
    mm.refresh_display();
    assert_eq!(
        mm.export_ascii(),
        concat!("root───╮\n", "       ╰──Parent[+]\n",)
    );
}

#[test]
fn test_export_multiline() {
    assert_export(
        "root\n\tLine1\\nLine2",
        concat!("root───╮\n", "       │  Line1\n", "       ╰──Line2\n"),
    );
}

// ═══ Engine Tests ══════════════════════════════════════════

#[test]
fn test_canvas_narrow_width_wraps() {
    let mut mm = MindMap::from_text("root\n\tA Very Long Title That Must Wrap");
    mm.calculate_layout(10, 0);
    mm.build_canvas();
    let cid = find_node_by_title(&mm, "A Very Long Title That Must Wrap");
    let layout = mm.layouts.get(&cid).unwrap();
    assert!(
        layout.lines > 1,
        "Long spaced text should wrap with width=10, got {} lines",
        layout.lines
    );
}

// ═══ Helpers ═══════════════════════════════════════════════

#[test]
fn test_multiline_text() {
    let mut mm = MindMap::from_text("root\n\tLine1\\nLine2");
    mm.calculate_layout(40, 0);
    let child_id = find_node_by_title(&mm, "Line1\nLine2");
    let layout = mm.layouts.get(&child_id).unwrap();
    assert!(
        layout.lines >= 2,
        "Multi-line node should have >= 2 lines, got {}",
        layout.lines
    );
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
    let mut mm = MindMap::from_text("root\n\tA");
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.update_note("This is a note".to_string());

    let json = serde_json::to_string(&mm).unwrap();
    let mm2: MindMap = serde_json::from_str(&json).unwrap();
    let a2_id = find_node_by_title(&mm2, "A");
    assert_eq!(mm2.nodes[&a2_id].note, "This is a note");
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

#[test]
fn test_export_ascii_alignment() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    mm.line_spacing = 0;
    mm.refresh_display();
    let json = serde_json::to_string(&mm).unwrap();
    let mut mm2: MindMap = serde_json::from_str(&json).unwrap();
    mm2.line_spacing = 0;
    mm2.refresh_display();
    let ascii = mm2.export_ascii();
    let expected = concat!("root───┤\n", "       ├──A\n", "       ╰──B\n");
    assert_eq!(ascii, expected);
}

#[test]
fn test_export_cjk_visual_align() {
    let tree = "root\n\t测试下\n\t\tNew\n\t\t\tNEW\n\t\t\tNEW\n\t\t\tNEW\n\t\t\tNEW\n\t\t\tNEW";
    let mm0 = MindMap::from_text(tree);
    let json = serde_json::to_string(&mm0).unwrap();
    let mut mm: MindMap = serde_json::from_str(&json).unwrap();
    mm.line_spacing = 0;
    mm.refresh_display();
    let ascii = mm.export_ascii();

    let expected = concat!(
        "root───╮\n",
        "       ╰──测试下──────╮\n",
        "                      ╰──New───┤\n",
        "                               ├──NEW\n",
        "                               ├──NEW\n",
        "                               ├──NEW\n",
        "                               ├──NEW\n",
        "                               ╰──NEW\n",
    );
    assert_eq!(ascii, expected, "Export mismatch");
}

// ═══ Edge Cases: Navigation ════════════════════════════════

#[test]
fn test_nav_go_up_single_child_no_wrap() {
    let mut mm = MindMap::from_text("root\n\tOnly");
    let only_id = find_node_by_title(&mm, "Only");
    mm.active_node = only_id;
    mm.go_up();
    assert_eq!(mm.active_node, only_id, "Single child should not wrap");
}

#[test]
fn test_nav_go_down_single_child_no_wrap() {
    let mut mm = MindMap::from_text("root\n\tOnly");
    let only_id = find_node_by_title(&mm, "Only");
    mm.active_node = only_id;
    mm.go_down();
    assert_eq!(mm.active_node, only_id, "Single child should not wrap");
}

#[test]
fn test_nav_go_up_from_root() {
    let mut mm = MindMap::new();
    mm.go_up();
    assert_eq!(mm.active_node, mm.root_id, "Root go_up should stay on root");
}

#[test]
fn test_nav_go_down_from_root() {
    let mut mm = MindMap::new();
    mm.go_down();
    assert_eq!(
        mm.active_node, mm.root_id,
        "Root go_down should stay on root"
    );
}

#[test]
fn test_nav_go_left_from_root() {
    let mut mm = MindMap::new();
    mm.go_left();
    assert_eq!(
        mm.active_node, mm.root_id,
        "Root go_left should stay on root"
    );
}

#[test]
fn test_nav_go_right_to_middle_child() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB\n\tC");
    mm.go_right();
    assert_eq!(
        mm.nodes[&mm.active_node].title, "B",
        "go_right should go to middle child"
    );
}

#[test]
fn test_nav_go_to_top_empty() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    mm.go_to_top();
    assert_eq!(
        mm.nodes[&mm.active_node].title, "root",
        "go_to_top should go to root"
    );
}

#[test]
fn test_nav_go_right_on_collapsed() {
    let mut mm = MindMap::from_text("root\n\tA\n\t\tA1");
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.toggle_node();
    mm.go_right();
    assert_eq!(mm.active_node, a_id, "go_right on collapsed should stay");
}

// ═══ Edge Cases: Move / Sort ═══════════════════════════════

#[test]
fn test_move_node_up_first_noop() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.move_node_up();
    let root = &mm.nodes[&mm.root_id];
    assert_eq!(
        mm.nodes[&root.children[0]].title, "A",
        "First child should stay first"
    );
}

#[test]
fn test_move_node_down_last_noop() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    let b_id = find_node_by_title(&mm, "B");
    mm.active_node = b_id;
    mm.move_node_down();
    let root = &mm.nodes[&mm.root_id];
    let last = root.children.last().unwrap();
    assert_eq!(mm.nodes[last].title, "B", "Last child should stay last");
}

#[test]
fn test_toggle_hide_on_root_noop() {
    let mut mm = MindMap::new();
    mm.active_node = mm.root_id;
    mm.toggle_hide();
    assert!(!mm.nodes[&mm.root_id].hidden, "Root should not be hidden");
}

// ═══ Edge Cases: Undo / Redo ═══════════════════════════════

#[test]
fn test_undo_empty_stack() {
    let mut mm = MindMap::new();
    let nodes_before = mm.nodes.clone();
    mm.undo();
    assert_eq!(
        mm.nodes.len(),
        nodes_before.len(),
        "Undo with empty stack should do nothing"
    );
}

#[test]
fn test_redo_empty_stack() {
    let mut mm = MindMap::new();
    let nodes_before = mm.nodes.clone();
    mm.redo();
    assert_eq!(
        mm.nodes.len(),
        nodes_before.len(),
        "Redo with empty stack should do nothing"
    );
}

#[test]
fn test_redo_cleared_after_new_edit() {
    let mut mm = MindMap::from_text("root\n\tA");
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.edit_node("B".to_string());
    mm.undo();
    mm.edit_node("C".to_string());
    mm.redo();
    assert_eq!(
        mm.nodes[&a_id].title, "C",
        "Redo stack should be cleared after new edit"
    );
}

#[test]
fn test_update_note_undo() {
    let mut mm = MindMap::from_text("root\n\tA");
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.update_note("My note".to_string());
    assert_eq!(mm.nodes[&a_id].note, "My note");
    mm.undo();
    assert_eq!(mm.nodes[&a_id].note, "", "Undo should restore empty note");
}

// ═══ Edge Cases: Clipboard ═════════════════════════════════

#[test]
fn test_paste_as_children_empty_clipboard() {
    let mut mm = MindMap::from_text("root\n\tA");
    mm.clipboard = None;
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.paste_as_children();
    let a = &mm.nodes[&a_id];
    assert!(
        a.children.is_empty(),
        "Paste with empty clipboard should do nothing"
    );
}

#[test]
fn test_paste_as_siblings_empty_clipboard() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    mm.clipboard = None;
    let b_id = find_node_by_title(&mm, "B");
    mm.active_node = b_id;
    mm.paste_as_siblings();
    let root = &mm.nodes[&mm.root_id];
    assert_eq!(
        root.children.len(),
        2,
        "Paste with empty clipboard should do nothing"
    );
}

#[test]
fn test_cut_node_root_noop() {
    let mut mm = MindMap::new();
    mm.active_node = mm.root_id;
    let count_before = mm.nodes.len();
    mm.cut_node();
    assert_eq!(mm.nodes.len(), count_before, "Cut root should do nothing");
}

#[test]
fn test_append_clipboard_empty() {
    let mut mm = MindMap::from_text("root\n\tHello");
    mm.clipboard = None;
    let hello_id = find_node_by_title(&mm, "Hello");
    mm.active_node = hello_id;
    mm.append_clipboard_to_title();
    assert_eq!(
        mm.nodes[&hello_id].title, "Hello",
        "Title should not change"
    );
}

#[test]
fn test_paste_as_siblings_on_root() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    let a_id = find_node_by_title(&mm, "A");
    mm.active_node = a_id;
    mm.yank_node();
    mm.active_node = mm.root_id;
    mm.paste_as_siblings();
    let root = &mm.nodes[&mm.root_id];
    assert!(
        !root.children.is_empty(),
        "Paste as siblings on root should redirect to children"
    );
}

#[test]
fn test_insert_sibling_on_root_creates_child() {
    let mut mm = MindMap::new();
    mm.active_node = mm.root_id;
    mm.insert_sibling();
    let root = &mm.nodes[&mm.root_id];
    assert_eq!(
        root.children.len(),
        1,
        "Insert sibling on root should create child"
    );
}

// ═══ Edge Cases: Collapse ══════════════════════════════════

#[test]
fn test_collapse_level_zero() {
    let mut mm = MindMap::from_text("root\n\tL1\n\t\tL2");
    mm.collapse_level(0);
    assert!(
        mm.nodes[&mm.root_id].collapsed,
        "Level 0 should collapse root at depth 0"
    );
}

#[test]
fn test_collapse_all() {
    let mut mm = MindMap::from_text("root\n\tA\n\t\tA1\n\tB");
    mm.collapse_all();
    let a_id = find_node_by_title(&mm, "A");
    let b_id = find_node_by_title(&mm, "B");
    assert!(
        mm.nodes[&a_id].collapsed,
        "First-level children should be collapsed"
    );
    assert!(
        mm.nodes[&b_id].collapsed,
        "First-level children should be collapsed"
    );
}

// ═══ Edge Cases: Export ════════════════════════════════════

#[test]
fn test_export_ascii_empty_canvas() {
    let mut mm = MindMap::new();
    mm.canvas.clear();
    let ascii = mm.export_ascii();
    assert_eq!(ascii, "", "Empty canvas should produce empty string");
}

#[test]
fn test_html_export_hidden_nodes_skipped() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    let b_id = find_node_by_title(&mm, "B");
    mm.active_node = b_id;
    mm.toggle_hide();
    let html = mm.export_html();
    assert!(
        !html.contains(">B<"),
        "Hidden node 'B' should not appear in HTML"
    );
}

// ═══ Edge Cases: Display Engine ════════════════════════════

#[test]
fn test_lines_for_display_empty() {
    let lines = MindMap::lines_for_display("", 40);
    assert_eq!(lines.len(), 1, "Empty string should produce one empty line");
    assert_eq!(lines[0], "");
}

#[test]
fn test_lines_for_display_narrow_wrap() {
    let lines = MindMap::lines_for_display("hello world test", 10);
    assert!(
        lines.len() > 1,
        "Narrow width should cause word wrap, got {} lines",
        lines.len()
    );
}

#[test]
fn test_collect_all_descendants_leaf() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    let a_id = find_node_by_title(&mm, "A");
    let desc = mm.collect_all_descendants(a_id);
    assert!(desc.is_empty(), "Leaf node should have no descendants");
}

#[test]
fn test_collect_all_descendants_deep() {
    let mm = MindMap::from_text("root\n\tA\n\t\tA1\n\t\t\tA1a\n\t\tA2");
    let a_id = find_node_by_title(&mm, "A");
    let desc = mm.collect_all_descendants(a_id);
    assert_eq!(desc.len(), 3, "A should have 3 descendants");
}

#[test]
fn test_visible_nodes_with_show_hidden() {
    let mut mm = MindMap::from_text("root\n\tA\n\tB");
    let b_id = find_node_by_title(&mm, "B");
    mm.active_node = b_id;
    mm.toggle_hide();
    assert!(
        !mm.visible_nodes.contains(&b_id),
        "Hidden node should not be visible"
    );
    mm.show_hidden = true;
    mm.refresh_display();
    assert!(
        mm.visible_nodes.contains(&b_id),
        "Hidden node should be visible with show_hidden=true"
    );
}

#[test]
fn test_canvas_with_cjk_chars() {
    let mut mm = MindMap::from_text("root\n\t中文测试");
    mm.calculate_layout(40, 0);
    mm.build_canvas();
    assert!(mm.canvas.len() > 0, "Canvas should handle CJK chars");
    let cid = find_node_by_title(&mm, "中文测试");
    let layout = mm.layouts.get(&cid).unwrap();
    assert!(
        layout.w >= 4,
        "CJK node should have char width >= 4, got {}",
        layout.w
    );
}

// ═══ Edge Cases: Marks ═════════════════════════════════════

#[test]
fn test_rank_refresh_display() {
    let mut mm = MindMap::from_text("root\n\tTask");
    let task_id = find_node_by_title(&mm, "Task");
    mm.active_node = task_id;
    mm.modify_positive_rank(1);
    mm.build_canvas();
    assert!(
        mm.export_ascii().contains('+'),
        "Canvas should reflect rank marker"
    );
}

#[test]
fn test_star_refresh_display() {
    let mut mm = MindMap::from_text("root\n\tItem");
    let item_id = find_node_by_title(&mm, "Item");
    mm.active_node = item_id;
    mm.add_star();
    mm.build_canvas();
    assert!(
        mm.export_ascii().contains('★'),
        "Canvas should reflect star marker"
    );
}

#[test]
fn test_export_cjk_no_space_in_new() {
    // Regression: CJK text above should not cause spaces inside "NEW" below.
    let mm = MindMap::from_text("root\n\t中文测试\n\tNEW");
    let exported = mm.export_ascii();
    println!("=== EXPORT ===\n{exported}=== END ===");
    assert!(
        exported.contains("NEW"),
        "NEW should be contiguous, but export was:\n{}",
        exported
    );
}

#[test]
fn test_export_cjk_no_space_in_new_deep() {
    // Deeper nesting: CJK at depth 1, NEW at depth 1, plus NEW at depth 2
    let tree = "root\n\t中文测试\n\t\tNEW\n\tNEW";
    let mm = MindMap::from_text(tree);
    let exported = mm.export_ascii();
    println!("=== EXPORT DEEP ===\n{exported}=== END ===");
    // All "NEW" occurrences must be contiguous
    for line in exported.lines() {
        if line.contains('N') && line.contains('W') {
            assert!(
                line.contains("NEW"),
                "NEW should be contiguous in line: {:?}\nFull export:\n{}",
                line,
                exported
            );
        }
    }
}

#[test]
fn test_export_connector_cjk() {
    let tree = "根节点\n\t子节点\n\t\tNEW\n\t\tNEW\n\tNEW\n\tNEW";
    let mm = MindMap::from_text(tree);
    let exported = mm.export_ascii();
    println!("=== CONNECTOR TEST ===\n{exported}=== END ===");
    let lines: Vec<&str> = exported.lines().collect();
    // Depth-1 children (under root) should use ├── or ╰──, not │──
    // │── is only for continuing a deeper subtree
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        // If this line has │──NEW, it's a bug (orphan vertical bar at wrong depth)
        if trimmed.starts_with("│──") {
            panic!(
                "Line {} has orphan vertical connector: {:?}\nFull export:\n{}",
                i, line, exported
            );
        }
    }
}
