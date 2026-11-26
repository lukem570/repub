//! Layers for organizing shapes on a page.

use crate::id::{LayerId, ObjectId};
use crate::shape::Shape;

/// A layer containing shapes on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// Unique layer identifier
    pub id: LayerId,
    /// Layer name
    pub name: String,
    /// Whether the layer is visible
    pub visible: bool,
    /// Whether the layer is locked (shapes can't be edited)
    pub locked: bool,
    /// Whether the layer prints
    pub printable: bool,
    /// Shapes on this layer
    shapes: Vec<Shape>,
}

impl Layer {
    /// Create a new empty layer.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            printable: true,
            shapes: Vec::new(),
        }
    }

    /// Create the default layer for a page.
    pub fn default_layer() -> Self {
        Self::new("Default")
    }

    /// Get all shapes on this layer.
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }

    /// Get mutable access to all shapes.
    pub fn shapes_mut(&mut self) -> &mut [Shape] {
        &mut self.shapes
    }

    /// Add a shape to the layer.
    pub fn add_shape(&mut self, shape: Shape) -> ObjectId {
        let id = shape.id();
        self.shapes.push(shape);
        id
    }

    /// Insert a shape at a specific index in the layer.
    pub fn insert_shape(&mut self, index: usize, shape: Shape) -> ObjectId {
        let id = shape.id();
        let index = index.min(self.shapes.len());
        self.shapes.insert(index, shape);
        id
    }

    /// Remove a shape by ID (also a member inside a group).
    pub fn remove_shape(&mut self, id: ObjectId) -> Option<Shape> {
        if let Some(index) = self.shapes.iter().position(|s| s.id() == id) {
            Some(self.shapes.remove(index))
        } else {
            self.shapes.iter_mut().find_map(|s| s.remove_member(id))
        }
    }

    /// Get a shape by ID (also a member inside a group).
    pub fn get_shape(&self, id: ObjectId) -> Option<&Shape> {
        self.shapes.iter().find_map(|s| s.find(id))
    }

    /// Get a mutable reference to a shape by ID.
    pub fn get_shape_mut(&mut self, id: ObjectId) -> Option<&mut Shape> {
        self.shapes.iter_mut().find_map(|s| s.find_mut(id))
    }

    /// Get the number of shapes on this layer.
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }

    /// Check if the layer is empty.
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Move a shape to a different z-index within the layer.
    pub fn reorder_shape(&mut self, id: ObjectId, new_index: usize) -> bool {
        if let Some(current_index) = self.shapes.iter().position(|s| s.id() == id) {
            let shape = self.shapes.remove(current_index);
            let insert_index = new_index.min(self.shapes.len());
            self.shapes.insert(insert_index, shape);
            true
        } else {
            false
        }
    }

    /// Bring a shape to the front (top of z-order).
    pub fn bring_to_front(&mut self, id: ObjectId) -> bool {
        self.reorder_shape(id, self.shapes.len())
    }

    /// Send a shape to the back (bottom of z-order).
    pub fn send_to_back(&mut self, id: ObjectId) -> bool {
        self.reorder_shape(id, 0)
    }

    /// Move a shape forward one position in z-order.
    pub fn bring_forward(&mut self, id: ObjectId) -> bool {
        if let Some(index) = self.shapes.iter().position(|s| s.id() == id) {
            if index < self.shapes.len() - 1 {
                self.shapes.swap(index, index + 1);
                return true;
            }
        }
        false
    }

    /// Move a shape backward one position in z-order.
    pub fn send_backward(&mut self, id: ObjectId) -> bool {
        if let Some(index) = self.shapes.iter().position(|s| s.id() == id) {
            if index > 0 {
                self.shapes.swap(index, index - 1);
                return true;
            }
        }
        false
    }

    /// Set layer visibility.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Set layer locked state.
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::default_layer()
    }
}
