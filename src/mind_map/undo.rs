use super::{MindMap, UndoSnapshot};

impl MindMap {
    pub(crate) fn push_undo(&mut self) {
        let snapshot = UndoSnapshot {
            nodes: self.nodes.clone(),
            root_id: self.root_id,
            active_node: self.active_node,
        };
        self.push_undo_snapshot(snapshot);
        self.redo_stack.clear();
    }

    /// Push onto the undo stack, enforcing the MAX_UNDO cap.
    fn push_undo_snapshot(&mut self, snapshot: UndoSnapshot) {
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > Self::MAX_UNDO {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            let redo_snapshot = UndoSnapshot {
                nodes: self.nodes.clone(),
                root_id: self.root_id,
                active_node: self.active_node,
            };
            self.redo_stack.push(redo_snapshot);

            self.nodes = snapshot.nodes;
            self.root_id = snapshot.root_id;
            self.active_node = snapshot.active_node;
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
            self.push_undo_snapshot(undo_snapshot);

            self.nodes = snapshot.nodes;
            self.root_id = snapshot.root_id;
            self.active_node = snapshot.active_node;
            self.refresh_display();
        }
    }
}
