//! Document session management.

use std::collections::HashMap;
use std::path::PathBuf;

use repub_model::id::ObjectId;
use repub_model::shape::Shape;
use repub_model::Document;
use uuid::Uuid;

use crate::error::{McpError, McpResult};

/// A document editing session.
#[derive(Debug)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// The document being edited
    pub document: Option<Document>,
    /// Path to the document file (if saved)
    pub path: Option<PathBuf>,
    /// Whether the document has unsaved changes
    pub dirty: bool,
    /// Object ID to string ID mapping
    object_ids: HashMap<String, ObjectId>,
    /// String ID to object ID mapping
    id_to_object: HashMap<ObjectId, String>,
    /// Counter for generating string IDs
    id_counter: u64,
}

impl Session {
    /// Create a new session.
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            document: None,
            path: None,
            dirty: false,
            object_ids: HashMap::new(),
            id_to_object: HashMap::new(),
            id_counter: 0,
        }
    }

    /// Create a new document in this session.
    pub fn create_document(&mut self, doc: Document) {
        self.document = Some(doc);
        self.path = None;
        self.dirty = true;
        self.object_ids.clear();
        self.id_to_object.clear();
        self.id_counter = 0;
        
        // Index existing objects
        self.index_document();
    }

    /// Open a document from a file.
    pub fn open_document(&mut self, doc: Document, path: PathBuf) {
        self.document = Some(doc);
        self.path = Some(path);
        self.dirty = false;
        self.object_ids.clear();
        self.id_to_object.clear();
        self.id_counter = 0;
        
        self.index_document();
    }

    /// Mark the document as saved.
    pub fn mark_saved(&mut self, path: PathBuf) {
        self.path = Some(path);
        self.dirty = false;
    }

    /// Get the document, returning an error if none is open.
    pub fn document(&self) -> McpResult<&Document> {
        self.document.as_ref().ok_or(McpError::NoDocument)
    }

    /// Get mutable document, returning an error if none is open.
    pub fn document_mut(&mut self) -> McpResult<&mut Document> {
        if self.document.is_some() {
            self.dirty = true;
        }
        self.document.as_mut().ok_or(McpError::NoDocument)
    }

    /// Close the current document.
    pub fn close_document(&mut self) {
        self.document = None;
        self.path = None;
        self.dirty = false;
        self.object_ids.clear();
        self.id_to_object.clear();
    }

    /// Generate a string ID for an object.
    pub fn register_object(&mut self, object_id: ObjectId) -> String {
        if let Some(str_id) = self.id_to_object.get(&object_id) {
            return str_id.clone();
        }

        self.id_counter += 1;
        let str_id = format!("obj_{}", self.id_counter);
        self.object_ids.insert(str_id.clone(), object_id);
        self.id_to_object.insert(object_id, str_id.clone());
        str_id
    }

    /// Look up an object ID from a string ID.
    pub fn get_object_id(&self, str_id: &str) -> McpResult<ObjectId> {
        self.object_ids
            .get(str_id)
            .copied()
            .ok_or_else(|| McpError::ObjectNotFound(str_id.to_string()))
    }

    /// Find an object by string ID.
    pub fn find_object(&self, str_id: &str) -> McpResult<&Shape> {
        let doc = self.document()?;
        let _obj_id = self.get_object_id(str_id)?;
        
        // Search through all pages and layers
        for page in doc.pages() {
            for layer in page.layers() {
                for obj in layer.shapes() {
                    if let Some(registered_id) = self.id_to_object.get(&obj.id()) {
                        if registered_id == str_id {
                            return Ok(obj);
                        }
                    }
                }
            }
        }

        Err(McpError::ObjectNotFound(str_id.to_string()))
    }

    /// Index all objects in the document.
    fn index_document(&mut self) {
        // Collect object IDs first to avoid borrow conflicts
        let object_ids: Vec<_> = if let Some(ref doc) = self.document {
            doc.pages()
                .iter()
                .flat_map(|page| page.layers())
                .flat_map(|layer| layer.shapes())
                .map(|obj| obj.id())
                .collect()
        } else {
            vec![]
        };

        // Now register them
        for obj_id in object_ids {
            self.register_object(obj_id);
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::{PageSize, PageOrientation};

    #[test]
    fn test_session_creation() {
        let session = Session::new();
        assert!(session.document.is_none());
        assert!(!session.dirty);
    }

    #[test]
    fn test_create_document() {
        let mut session = Session::new();
        let doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        session.create_document(doc);
        
        assert!(session.document.is_some());
        assert!(session.dirty);
        assert!(session.path.is_none());
    }

    #[test]
    fn test_no_document_error() {
        let session = Session::new();
        let result = session.document();
        assert!(matches!(result, Err(McpError::NoDocument)));
    }
}
