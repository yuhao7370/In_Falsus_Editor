#[derive(Debug, Clone)]
struct EditorSnapshot {
    notes: Vec<GroundNote>,
    next_note_id: u64,
    timeline_events: Vec<TimelineEvent>,
    next_event_id: u64,
    bpm_source: BpmSourceData,
    track_source: TrackSourceData,
}

#[derive(Debug)]
struct UndoHistory {
    stack: Vec<EditorSnapshot>,
    index: usize,
    max_size: usize,
}

impl UndoHistory {
    fn new(max_size: usize) -> Self {
        Self {
            stack: Vec::new(),
            index: 0,
            max_size,
        }
    }

    fn push(&mut self, snapshot: EditorSnapshot) {
        if self.index + 1 < self.stack.len() {
            self.stack.truncate(self.index + 1);
        }
        self.stack.push(snapshot);
        if self.stack.len() > self.max_size {
            self.stack.remove(0);
        }
        self.index = self.stack.len().saturating_sub(1);
    }

    fn is_at_top(&self) -> bool {
        !self.stack.is_empty() && self.index == self.stack.len() - 1
    }

    fn can_undo(&self) -> bool {
        self.index > 0
    }

    fn can_redo(&self) -> bool {
        self.index + 1 < self.stack.len()
    }

    fn undo(&mut self) -> Option<&EditorSnapshot> {
        if self.can_undo() {
            self.index -= 1;
            Some(&self.stack[self.index])
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<&EditorSnapshot> {
        if self.can_redo() {
            self.index += 1;
            Some(&self.stack[self.index])
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct UndoManager {
    history: UndoHistory,
}

impl UndoManager {
    fn new(max_size: usize) -> Self {
        Self {
            history: UndoHistory::new(max_size),
        }
    }

    fn push(&mut self, snapshot: EditorSnapshot) {
        self.history.push(snapshot);
    }

    fn is_at_top(&self) -> bool {
        self.history.is_at_top()
    }

    fn undo(&mut self) -> Option<&EditorSnapshot> {
        self.history.undo()
    }

    fn redo(&mut self) -> Option<&EditorSnapshot> {
        self.history.redo()
    }

    fn capture(&mut self, state: &EditorState) {
        self.push(state.snapshot());
    }

    fn capture_if_at_top(&mut self, state: &EditorState) {
        if self.is_at_top() {
            self.capture(state);
        }
    }

    fn undo_snapshot(&mut self) -> Option<EditorSnapshot> {
        self.undo().cloned()
    }

    fn redo_snapshot(&mut self) -> Option<EditorSnapshot> {
        self.redo().cloned()
    }
}
