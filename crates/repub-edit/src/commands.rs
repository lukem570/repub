//! Command pattern for undo/redo.

use repub_geometry::Point;
use repub_model::id::ObjectId;
use repub_model::shape::Shape;
use repub_model::Document;

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

    /// Number of steps that can be undone.
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Check if undo is available.
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

/// Command to add a shape.
#[derive(Debug)]
pub struct AddShapeCommand {
    page_index: usize,
    shape: Option<Shape>,
    shape_id: Option<ObjectId>,
}

impl AddShapeCommand {
    /// Create a new add shape command.
    pub fn new(page_index: usize, shape: Shape) -> Self {
        Self {
            page_index,
            shape: Some(shape),
            shape_id: None,
        }
    }
}

impl Command for AddShapeCommand {
    fn execute(&mut self, doc: &mut Document) {
        if let Some(shape) = self.shape.take() {
            self.shape_id = Some(shape.id());
            if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
                page.add_shape(shape);
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(id) = self.shape_id {
            if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
                if let Some(shape) = page.remove_shape(id) {
                    self.shape = Some(shape);
                }
            }
        }
    }

    fn description(&self) -> &str {
        "Add Shape"
    }
}

/// Command to delete shapes.
#[derive(Debug)]
pub struct DeleteShapesCommand {
    page_index: usize,
    shape_ids: Vec<ObjectId>,
    shapes: Vec<Shape>,
}

impl DeleteShapesCommand {
    /// Create a new delete shapes command.
    pub fn new(page_index: usize, shape_ids: Vec<ObjectId>) -> Self {
        Self {
            page_index,
            shape_ids,
            shapes: Vec::new(),
        }
    }
}

impl Command for DeleteShapesCommand {
    fn execute(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            for id in &self.shape_ids {
                if let Some(shape) = page.remove_shape(*id) {
                    self.shapes.push(shape);
                }
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            for shape in self.shapes.drain(..) {
                page.add_shape(shape);
            }
        }
    }

    fn description(&self) -> &str {
        if self.shape_ids.len() == 1 {
            "Delete Shape"
        } else {
            "Delete Shapes"
        }
    }
}

/// Command to move shapes.
#[derive(Debug)]
pub struct MoveShapesCommand {
    page_index: usize,
    shape_ids: Vec<ObjectId>,
    delta: Point,
}

impl MoveShapesCommand {
    /// Create a new move shapes command.
    pub fn new(page_index: usize, shape_ids: Vec<ObjectId>, delta: Point) -> Self {
        Self {
            page_index,
            shape_ids,
            delta,
        }
    }
}

impl Command for MoveShapesCommand {
    fn execute(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            for id in &self.shape_ids {
                if let Some(shape) = page.get_shape_mut(*id) {
                    shape.translate(self.delta.x, self.delta.y);
                }
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            for id in &self.shape_ids {
                if let Some(shape) = page.get_shape_mut(*id) {
                    shape.translate(-self.delta.x, -self.delta.y);
                }
            }
        }
    }

    fn description(&self) -> &str {
        "Move"
    }
}

/// Command to resize a shape by setting its bounds.
#[derive(Debug)]
pub struct ResizeShapeCommand {
    page_index: usize,
    shape_id: ObjectId,
    new_bounds: repub_geometry::Rect,
    old_bounds: Option<repub_geometry::Rect>,
}

impl ResizeShapeCommand {
    /// Create a new resize command.
    pub fn new(page_index: usize, shape_id: ObjectId, new_bounds: repub_geometry::Rect) -> Self {
        Self {
            page_index,
            shape_id,
            new_bounds,
            old_bounds: None,
        }
    }
}

impl Command for ResizeShapeCommand {
    fn execute(&mut self, doc: &mut Document) {
        if let Some(page) = doc.pages_mut().get_mut(self.page_index) {
            if let Some(shape) = page.get_shape_mut(self.shape_id) {
                self.old_bounds = Some(shape.bounds());
                shape.set_bounds(self.new_bounds);
            }
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let (Some(old), Some(page)) = (self.old_bounds, doc.pages_mut().get_mut(self.page_index))
        {
            if let Some(shape) = page.get_shape_mut(self.shape_id) {
                shape.set_bounds(old);
            }
        }
    }

    fn description(&self) -> &str {
        "Resize"
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
        if self.page.take().is_some() {
            // Re-insert a page at the original index. The model constructs a
            // fresh blank page rather than restoring the exact contents.
            doc.insert_page(self.page_index.min(doc.pages().len()));
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
    use repub_model::{PageOrientation, PageSize};

    #[test]
    fn test_command_history() {
        let mut history = CommandHistory::new();
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);

        assert!(!history.can_undo());
        assert!(!history.can_redo());

        let shape = Shape::rectangle(Rect::new(100.0, 100.0, 200.0, 150.0));
        let cmd = AddShapeCommand::new(0, shape);
        history.execute(Box::new(cmd), &mut doc);

        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(doc.page(0).unwrap().shape_count(), 1);

        history.undo(&mut doc);

        assert!(!history.can_undo());
        assert!(history.can_redo());
        assert_eq!(doc.page(0).unwrap().shape_count(), 0);

        history.redo(&mut doc);

        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(doc.page(0).unwrap().shape_count(), 1);
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

    #[test]
    fn move_shapes_round_trip() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let id = doc
            .page_mut(0)
            .unwrap()
            .add_shape(Shape::rectangle(Rect::new(10.0, 10.0, 20.0, 20.0)));
        let mut cmd = MoveShapesCommand::new(0, vec![id], Point::new(5.0, 7.0));
        cmd.execute(&mut doc);
        assert_eq!(
            doc.page(0).unwrap().get_shape(id).unwrap().bounds().origin,
            Point::new(15.0, 17.0)
        );
        cmd.undo(&mut doc);
        assert_eq!(
            doc.page(0).unwrap().get_shape(id).unwrap().bounds().origin,
            Point::new(10.0, 10.0)
        );
    }
}
