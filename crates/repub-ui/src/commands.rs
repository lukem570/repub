//! Command pattern for undo/redo.

use repub_geometry::Point;
use repub_model::shape::Shape;
use repub_model::Document;
use repub_model::ObjectId;

/// A command that can be executed, undone, and redone.
pub trait Command: std::fmt::Debug + Send {
    /// Execute the command.
    fn execute(&mut self, doc: &mut Document);
    /// Undo the command.
    fn undo(&mut self, doc: &mut Document);
    /// Get a description of the command.
    fn description(&self) -> &str;
}

/// Command history for undo/redo.
#[derive(Debug, Default)]
pub struct CommandHistory {
    /// Commands that have been executed
    undo_stack: Vec<Box<dyn Command>>,
    /// Commands that have been undone
    redo_stack: Vec<Box<dyn Command>>,
    /// Maximum history size
    max_size: usize,
}

impl CommandHistory {
    /// Create a new command history.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size: 100,
        }
    }

    /// Execute a command and add it to history.
    pub fn execute(&mut self, mut command: Box<dyn Command>, doc: &mut Document) {
        command.execute(doc);
        self.undo_stack.push(command);
        self.redo_stack.clear();

        // Trim history if too large
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last command.
    pub fn undo(&mut self, doc: &mut Document) -> bool {
        if let Some(mut command) = self.undo_stack.pop() {
            command.undo(doc);
            self.redo_stack.push(command);
            true
        } else {
            false
        }
    }

    /// Redo the last undone command.
    pub fn redo(&mut self, doc: &mut Document) -> bool {
        if let Some(mut command) = self.redo_stack.pop() {
            command.execute(doc);
            self.undo_stack.push(command);
            true
        } else {
            false
        }
    }

    /// Check if undo is available.
    /// Number of steps that can be undone.
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the description of the next undo command.
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.description())
    }

    /// Get the description of the next redo command.
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.description())
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Record a change that the caller is about to make (or has made) directly on
    /// the document. `before` is the document as it was before the change.
    pub fn record(&mut self, label: &str, before: Document) {
        self.undo_stack.push(Box::new(SnapshotCommand {
            label: label.to_owned(),
            other: before,
        }));
        self.redo_stack.clear();
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Drop the most recent undo entry (used when a recorded change turned out to be a no-op).
    pub fn discard_last(&mut self) {
        self.undo_stack.pop();
    }
}

// === Concrete Commands ===

/// Whole-document snapshot: undo and redo swap the current document with the
/// stored one. Lets any direct edit (drag, resize, typing, ...) be undone
/// without a dedicated command type.
#[derive(Debug)]
pub struct SnapshotCommand {
    label: String,
    /// The document state on the other side of this change.
    other: Document,
}

impl Command for SnapshotCommand {
    fn execute(&mut self, doc: &mut Document) {
        std::mem::swap(doc, &mut self.other);
    }

    fn undo(&mut self, doc: &mut Document) {
        std::mem::swap(doc, &mut self.other);
    }

    fn description(&self) -> &str {
        &self.label
    }
}

/// Command to add an object.
#[derive(Debug)]
pub struct AddObjectCommand {
    page_index: usize,
    object: Option<Shape>,
    object_id: Option<ObjectId>,
}

impl AddObjectCommand {
    /// Create a new add object command.
    pub fn new(page_index: usize, object: Shape) -> Self {
        Self {
            page_index,
            object: Some(object),
            object_id: None,
        }
    }
}

impl Command for AddObjectCommand {
    fn execute(&mut self, doc: &mut Document) {
        if let Some(object) = self.object.take() {
            self.object_id = Some(object.id());
            if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
                page.add_shape(object);
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(id) = self.object_id {
            if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
                // Find and remove the object by ID
                for layer in page.layers_mut() {
                    if let Some(obj) = layer.remove_shape(id) {
                        self.object = Some(obj);
                        break;
                    }
                }
            }
        }
    }

    fn description(&self) -> &str {
        "Add Object"
    }
}

/// Command to delete objects.
#[derive(Debug)]
pub struct DeleteObjectsCommand {
    page_index: usize,
    object_ids: Vec<ObjectId>,
    objects: Vec<Shape>,
}

impl DeleteObjectsCommand {
    /// Create a new delete objects command.
    pub fn new(page_index: usize, object_ids: Vec<ObjectId>) -> Self {
        Self {
            page_index,
            object_ids,
            objects: Vec::new(),
        }
    }
}

impl Command for DeleteObjectsCommand {
    fn execute(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            for layer in page.layers_mut() {
                for id in &self.object_ids {
                    if let Some(obj) = layer.remove_shape(*id) {
                        self.objects.push(obj);
                    }
                }
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            for obj in self.objects.drain(..) {
                page.add_shape(obj);
            }
        }
    }

    fn description(&self) -> &str {
        if self.object_ids.len() == 1 {
            "Delete Object"
        } else {
            "Delete Objects"
        }
    }
}

/// Command to move objects.
#[derive(Debug)]
pub struct MoveObjectsCommand {
    page_index: usize,
    object_ids: Vec<ObjectId>,
    delta: Point,
}

impl MoveObjectsCommand {
    /// Create a new move objects command.
    pub fn new(page_index: usize, object_ids: Vec<ObjectId>, delta: Point) -> Self {
        Self {
            page_index,
            object_ids,
            delta,
        }
    }
}

impl Command for MoveObjectsCommand {
    fn execute(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            for layer in page.layers_mut() {
                for obj in layer.shapes_mut() {
                    if self.object_ids.contains(&obj.id()) {
                        let bounds = obj.bounds();
                        obj.set_position_xy(
                            bounds.origin.x + self.delta.x,
                            bounds.origin.y + self.delta.y,
                        );
                    }
                }
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            for layer in page.layers_mut() {
                for obj in layer.shapes_mut() {
                    if self.object_ids.contains(&obj.id()) {
                        let bounds = obj.bounds();
                        obj.set_position_xy(
                            bounds.origin.x - self.delta.x,
                            bounds.origin.y - self.delta.y,
                        );
                    }
                }
            }
        }
    }

    fn description(&self) -> &str {
        "Move"
    }
}

/// Command to add a page.
#[derive(Debug)]
pub struct AddPageCommand {
    position: Option<usize>,
    page_index: Option<usize>,
}

impl AddPageCommand {
    /// Create a new add page command.
    pub fn new(position: Option<usize>) -> Self {
        Self {
            position,
            page_index: None,
        }
    }
}

impl Command for AddPageCommand {
    fn execute(&mut self, doc: &mut Document) {
        if let Some(pos) = self.position {
            doc.insert_page(pos);
            self.page_index = Some(pos);
        } else {
            doc.add_page();
            self.page_index = Some(doc.pages().len() - 1);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(idx) = self.page_index {
            doc.remove_page(idx);
        }
    }

    fn description(&self) -> &str {
        "Add Page"
    }
}

/// Command to delete a page.
#[derive(Debug)]
pub struct DeletePageCommand {
    page_index: usize,
    page: Option<repub_model::Page>,
}

impl DeletePageCommand {
    /// Create a new delete page command.
    pub fn new(page_index: usize) -> Self {
        Self {
            page_index,
            page: None,
        }
    }
}

impl Command for DeletePageCommand {
    fn execute(&mut self, doc: &mut Document) {
        if self.page_index < doc.pages().len() && doc.pages().len() > 1 {
            self.page = doc.remove_page(self.page_index);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(page) = self.page.take() {
            // Re-insert the page - for now just add at end
            // TODO: Implement proper insert_page_at
            doc.add_page();
        }
    }

    fn description(&self) -> &str {
        "Delete Page"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Rect;
    use repub_model::shape::Shape;
    use repub_model::{PageOrientation, PageSize};

    #[test]
    fn test_command_history() {
        let mut history = CommandHistory::new();
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);

        assert!(!history.can_undo());
        assert!(!history.can_redo());

        let shape = Shape::rectangle(Rect::new(100.0, 100.0, 200.0, 150.0));
        let cmd = AddObjectCommand::new(0, shape);
        history.execute(Box::new(cmd), &mut doc);

        assert!(history.can_undo());
        assert!(!history.can_redo());

        history.undo(&mut doc);

        assert!(!history.can_undo());
        assert!(history.can_redo());

        history.redo(&mut doc);

        assert!(history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_add_page_command() {
        let mut history = CommandHistory::new();
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);

        assert_eq!(doc.pages().len(), 1);

        let cmd = AddPageCommand::new(None);
        history.execute(Box::new(cmd), &mut doc);

        assert_eq!(doc.pages().len(), 2);

        history.undo(&mut doc);

        assert_eq!(doc.pages().len(), 1);
    }
}
