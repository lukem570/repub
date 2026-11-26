//! Document and page structures.

use repub_geometry::{Rect, Size, Unit};

use crate::id::{LayerId, MasterPageId, ObjectId, PageId};
use crate::layer::Layer;
use crate::shape::{Shape, ShapeKind};
use crate::styles::Fill;
use crate::{PageOrientation, PageSize};

/// Document metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentMetadata {
    /// Document title
    pub title: Option<String>,
    /// Author name
    pub author: Option<String>,
    /// Subject/description
    pub subject: Option<String>,
    /// Keywords for search
    pub keywords: Vec<String>,
    /// Creation timestamp (Unix epoch seconds)
    pub created: Option<i64>,
    /// Last modified timestamp
    pub modified: Option<i64>,
}

/// Document-level settings.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentSettings {
    /// Default unit for display
    pub display_unit: Unit,
    /// Whether to show rulers
    pub show_rulers: bool,
    /// Whether to show guides
    pub show_guides: bool,
    /// Whether to snap to grid
    pub snap_to_grid: bool,
    /// Grid spacing in points
    pub grid_spacing: f64,
    /// Whether to snap to guides
    pub snap_to_guides: bool,
    /// Whether to snap to objects
    pub snap_to_objects: bool,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            display_unit: Unit::Inches,
            show_rulers: true,
            show_guides: true,
            snap_to_grid: false,
            grid_spacing: 18.0, // 1/4 inch
            snap_to_guides: true,
            snap_to_objects: true,
        }
    }
}

/// Page margins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    /// Top margin in points
    pub top: f64,
    /// Right margin in points
    pub right: f64,
    /// Bottom margin in points
    pub bottom: f64,
    /// Left margin in points
    pub left: f64,
}

impl Default for Margins {
    fn default() -> Self {
        // Default 1 inch margins
        Self {
            top: 72.0,
            right: 72.0,
            bottom: 72.0,
            left: 72.0,
        }
    }
}

impl Margins {
    /// Create margins with equal values on all sides.
    pub fn all(value: f64) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Create margins with symmetric horizontal and vertical values.
    pub fn symmetric(vertical: f64, horizontal: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create custom margins.
    pub fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// A guide line on a page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    /// Position in points from the edge
    pub position: f64,
    /// Whether the guide is horizontal (true) or vertical (false)
    pub horizontal: bool,
}

impl Guide {
    /// Create a horizontal guide at the given position from the top.
    pub fn horizontal(position: f64) -> Self {
        Self {
            position,
            horizontal: true,
        }
    }

    /// Create a vertical guide at the given position from the left.
    pub fn vertical(position: f64) -> Self {
        Self {
            position,
            horizontal: false,
        }
    }
}

/// A single page in the document.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// Unique page identifier
    pub id: PageId,
    /// Page name (optional, for display in the pages panel)
    pub name: Option<String>,
    /// Page size
    pub size: Size,
    /// Page margins
    pub margins: Margins,
    /// Reference to master page (if any)
    pub master_page: Option<MasterPageId>,
    /// Layers on this page
    layers: Vec<Layer>,
    /// Guides on this page
    pub guides: Vec<Guide>,
    /// Background fill (None means white/default)
    pub background: Option<Fill>,
}

impl Page {
    /// Create a new page with the given size.
    pub fn new(size: PageSize, orientation: PageOrientation) -> Self {
        let base_size = size.to_points();
        let size = match orientation {
            PageOrientation::Portrait => base_size,
            PageOrientation::Landscape => Size::new(base_size.height, base_size.width),
        };

        Self {
            id: PageId::new(),
            name: None,
            size,
            margins: Margins::default(),
            master_page: None,
            layers: vec![Layer::default_layer()],
            guides: Vec::new(),
            background: None,
        }
    }

    /// Create a page with a specific ID (for deserialization).
    pub fn with_id(id: PageId, size: PageSize, orientation: PageOrientation) -> Self {
        let mut page = Self::new(size, orientation);
        page.id = id;
        page
    }

    /// A copy of this page with fresh page, layer and object IDs (including
    /// objects inside groups). Links between text frames on the page are
    /// pointed at the copies; links to frames elsewhere are dropped.
    pub fn duplicate(&self) -> Page {
        use std::collections::HashMap;

        fn renew(shape: &mut Shape, ids: &mut HashMap<ObjectId, ObjectId>) {
            let new_id = ObjectId::new();
            ids.insert(shape.id(), new_id);
            shape.common.id = new_id;
            if let ShapeKind::Group(group) = &mut shape.kind {
                for child in &mut group.children {
                    renew(child, ids);
                }
            }
        }
        fn relink(shape: &mut Shape, ids: &HashMap<ObjectId, ObjectId>) {
            match &mut shape.kind {
                ShapeKind::TextFrame(tf) => tf.linked_frame = tf.linked_frame.and_then(|old| ids.get(&old).copied()),
                ShapeKind::Group(group) => group.children.iter_mut().for_each(|c| relink(c, ids)),
                _ => {}
            }
        }

        let mut copy = self.clone();
        copy.id = PageId::new();
        let mut ids = HashMap::new();
        for layer in &mut copy.layers {
            layer.id = LayerId::new();
            for shape in layer.shapes_mut() {
                renew(shape, &mut ids);
            }
        }
        for layer in &mut copy.layers {
            for shape in layer.shapes_mut() {
                relink(shape, &ids);
            }
        }
        copy
    }

    /// Get the content area (page minus margins).
    pub fn content_rect(&self) -> Rect {
        Rect::new(
            self.margins.left,
            self.margins.top,
            self.size.width - self.margins.left - self.margins.right,
            self.size.height - self.margins.top - self.margins.bottom,
        )
    }

    /// Get all layers.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Get mutable access to all layers.
    pub fn layers_mut(&mut self) -> &mut [Layer] {
        &mut self.layers
    }

    /// Get a layer by ID.
    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Get a mutable layer by ID.
    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Get the default layer (first layer).
    pub fn default_layer(&self) -> &Layer {
        &self.layers[0]
    }

    /// Get the default layer mutably.
    pub fn default_layer_mut(&mut self) -> &mut Layer {
        &mut self.layers[0]
    }

    /// Add a new layer.
    pub fn add_layer(&mut self, name: impl Into<String>) -> LayerId {
        let layer = Layer::new(name);
        let id = layer.id;
        self.layers.push(layer);
        id
    }

    /// Remove a layer by ID. Cannot remove the last layer.
    pub fn remove_layer(&mut self, id: LayerId) -> Option<Layer> {
        if self.layers.len() <= 1 {
            return None;
        }
        if let Some(index) = self.layers.iter().position(|l| l.id == id) {
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Add a shape to the default layer.
    pub fn add_shape(&mut self, shape: Shape) -> ObjectId {
        self.default_layer_mut().add_shape(shape)
    }

    /// Insert a shape at a specific index in the default layer.
    pub fn insert_shape(&mut self, index: usize, shape: Shape) -> ObjectId {
        self.default_layer_mut().insert_shape(index, shape)
    }

    /// Add a shape to a specific layer.
    pub fn add_shape_to_layer(&mut self, layer_id: LayerId, shape: Shape) -> Option<ObjectId> {
        self.layer_mut(layer_id).map(|layer| layer.add_shape(shape))
    }

    /// Make every group's bounds enclose its members (after a member was
    /// moved or resized on its own).
    pub fn refresh_group_bounds(&mut self) {
        for layer in &mut self.layers {
            for shape in layer.shapes_mut() {
                shape.refresh_group_bounds();
            }
        }
    }

    /// Group the shapes `ids` (at least two on this page). The group takes the
    /// place of the top-most of them in the stacking order, and keeps them in
    /// their stacking order. Returns the group's ID.
    pub fn group_shapes(&mut self, ids: &[ObjectId]) -> Option<ObjectId> {
        // Members in stacking order, and where the top-most one sits
        let mut found: Vec<(usize, usize)> = Vec::new(); // (layer, index)
        for (li, layer) in self.layers.iter().enumerate() {
            for (si, shape) in layer.shapes().iter().enumerate() {
                if ids.contains(&shape.id()) {
                    found.push((li, si));
                }
            }
        }
        if found.len() < 2 {
            return None;
        }
        let (top_layer, top_index) = *found.last()?;
        // Position in the top layer once the members are taken out
        let insert_at = self.layers[top_layer].shapes()[..top_index].iter().filter(|s| !ids.contains(&s.id())).count();
        let members: Vec<ObjectId> = found.iter().map(|&(li, si)| self.layers[li].shapes()[si].id()).collect();
        let children: Vec<Shape> = members.iter().filter_map(|&id| self.remove_shape(id)).collect();
        let group = Shape::group_from(children);
        let id = group.id();
        let layer = &mut self.layers[top_layer];
        layer.add_shape(group);
        layer.reorder_shape(id, insert_at);
        Some(id)
    }

    /// Replace group `id` with its members, in its place. Returns their IDs
    /// (empty if `id` isn't a group on this page).
    pub fn ungroup(&mut self, id: ObjectId) -> Vec<ObjectId> {
        for layer in &mut self.layers {
            let Some(index) = layer.shapes().iter().position(|s| s.id() == id) else { continue };
            if !matches!(layer.shapes()[index].kind, ShapeKind::Group(_)) {
                return Vec::new();
            }
            let Some(group_shape) = layer.remove_shape(id) else { return Vec::new() };
            let ShapeKind::Group(group) = group_shape.kind else { return Vec::new() };
            let mut ids = Vec::new();
            for (k, child) in group.children.into_iter().enumerate() {
                let child_id = layer.add_shape(child);
                layer.reorder_shape(child_id, index + k);
                ids.push(child_id);
            }
            return ids;
        }
        Vec::new()
    }

    /// Remove a shape by ID (searches all layers).
    pub fn remove_shape(&mut self, id: ObjectId) -> Option<Shape> {
        for layer in &mut self.layers {
            if let Some(shape) = layer.remove_shape(id) {
                return Some(shape);
            }
        }
        None
    }

    /// Get a shape by ID (searches all layers).
    pub fn get_shape(&self, id: ObjectId) -> Option<&Shape> {
        for layer in &self.layers {
            if let Some(shape) = layer.get_shape(id) {
                return Some(shape);
            }
        }
        None
    }

    /// Get a mutable shape by ID (searches all layers).
    pub fn get_shape_mut(&mut self, id: ObjectId) -> Option<&mut Shape> {
        for layer in &mut self.layers {
            if let Some(shape) = layer.get_shape_mut(id) {
                return Some(shape);
            }
        }
        None
    }

    /// Get all shapes across all layers.
    pub fn all_shapes(&self) -> impl Iterator<Item = &Shape> {
        self.layers.iter().flat_map(|l| l.shapes().iter())
    }

    /// Get the total number of shapes across all layers.
    pub fn shape_count(&self) -> usize {
        self.layers.iter().map(|l| l.shape_count()).sum()
    }

    /// Add a horizontal guide.
    pub fn add_horizontal_guide(&mut self, position: f64) {
        self.guides.push(Guide::horizontal(position));
    }

    /// Add a vertical guide.
    pub fn add_vertical_guide(&mut self, position: f64) {
        self.guides.push(Guide::vertical(position));
    }

    /// Remove all guides.
    pub fn clear_guides(&mut self) {
        self.guides.clear();
    }
}

/// A master page (template for other pages).
#[derive(Debug, Clone, PartialEq)]
pub struct MasterPage {
    /// Unique identifier
    pub id: MasterPageId,
    /// Master page name
    pub name: String,
    /// Page size
    pub size: Size,
    /// Margins
    pub margins: Margins,
    /// Layers with objects
    pub layers: Vec<Layer>,
    /// Guides
    pub guides: Vec<Guide>,
}

impl MasterPage {
    /// Create a new master page.
    pub fn new(name: impl Into<String>, size: PageSize, orientation: PageOrientation) -> Self {
        let base_size = size.to_points();
        let size = match orientation {
            PageOrientation::Portrait => base_size,
            PageOrientation::Landscape => Size::new(base_size.height, base_size.width),
        };

        Self {
            id: MasterPageId::new(),
            name: name.into(),
            size,
            margins: Margins::default(),
            layers: vec![Layer::default_layer()],
            guides: Vec::new(),
        }
    }

    /// Add a shape to the master page's default layer.
    pub fn add_shape(&mut self, shape: Shape) -> ObjectId {
        self.layers[0].add_shape(shape)
    }

    /// A page holding this master's content, for editing it with the same tools
    /// as a page. Write the result back with [`Self::update_from_page`].
    pub fn to_page(&self) -> Page {
        Page {
            id: PageId::new(),
            name: Some(self.name.clone()),
            size: self.size,
            margins: self.margins,
            master_page: None,
            layers: self.layers.clone(),
            guides: self.guides.clone(),
            background: None,
        }
    }

    /// Take the content of a page made by [`Self::to_page`].
    pub fn update_from_page(&mut self, page: &Page) {
        self.layers = page.layers.clone();
        self.guides = page.guides.clone();
        self.margins = page.margins;
    }
}

/// A complete Repub document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// Document settings
    pub settings: DocumentSettings,
    /// Default page size for new pages
    pub default_page_size: PageSize,
    /// Default page orientation
    pub default_orientation: PageOrientation,
    /// All pages in the document
    pages: Vec<Page>,
    /// Master pages
    master_pages: Vec<MasterPage>,
}

impl Document {
    /// Create a new document with one page.
    pub fn new(page_size: PageSize, orientation: PageOrientation) -> Self {
        let mut doc = Self {
            metadata: DocumentMetadata::default(),
            settings: DocumentSettings::default(),
            default_page_size: page_size,
            default_orientation: orientation,
            pages: Vec::new(),
            master_pages: Vec::new(),
        };
        doc.add_page();
        doc
    }

    /// Create an empty document with no pages.
    pub fn empty(page_size: PageSize, orientation: PageOrientation) -> Self {
        Self {
            metadata: DocumentMetadata::default(),
            settings: DocumentSettings::default(),
            default_page_size: page_size,
            default_orientation: orientation,
            pages: Vec::new(),
            master_pages: Vec::new(),
        }
    }

    /// Get all pages.
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// Get mutable access to all pages.
    pub fn pages_mut(&mut self) -> &mut [Page] {
        &mut self.pages
    }

    /// Get a specific page by index.
    pub fn page(&self, index: usize) -> Option<&Page> {
        self.pages.get(index)
    }

    /// Get a mutable reference to a specific page.
    pub fn page_mut(&mut self, index: usize) -> Option<&mut Page> {
        self.pages.get_mut(index)
    }

    /// Get a page by ID.
    pub fn page_by_id(&self, id: PageId) -> Option<&Page> {
        self.pages.iter().find(|p| p.id == id)
    }

    /// Get a mutable page by ID.
    pub fn page_by_id_mut(&mut self, id: PageId) -> Option<&mut Page> {
        self.pages.iter_mut().find(|p| p.id == id)
    }

    /// Get the number of pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Add a new page with default settings.
    pub fn add_page(&mut self) -> &mut Page {
        let page = Page::new(self.default_page_size, self.default_orientation);
        self.pages.push(page);
        self.pages.last_mut().unwrap()
    }

    /// Insert a page at a specific index.
    pub fn insert_page(&mut self, index: usize) -> Option<&mut Page> {
        if index > self.pages.len() {
            return None;
        }

        let page = Page::new(self.default_page_size, self.default_orientation);
        self.pages.insert(index, page);
        Some(&mut self.pages[index])
    }

    /// Insert a copy of page `index` (objects included, with new IDs) right
    /// after it. Returns the new page's index.
    pub fn duplicate_page(&mut self, index: usize) -> Option<usize> {
        let copy = self.pages.get(index)?.duplicate();
        self.pages.insert(index + 1, copy);
        Some(index + 1)
    }

    /// Remove a page by index.
    pub fn remove_page(&mut self, index: usize) -> Option<Page> {
        if index < self.pages.len() {
            Some(self.pages.remove(index))
        } else {
            None
        }
    }

    /// Reorder a page.
    pub fn move_page(&mut self, from: usize, to: usize) -> bool {
        if from >= self.pages.len() || to >= self.pages.len() {
            return false;
        }
        let page = self.pages.remove(from);
        self.pages.insert(to, page);
        true
    }

    /// Rename a page.
    /// Pass `None` to clear the custom name and use default numbering.
    pub fn rename_page(&mut self, index: usize, name: Option<String>) -> bool {
        if let Some(page) = self.pages.get_mut(index) {
            page.name = name;
            true
        } else {
            false
        }
    }

    /// Get all master pages.
    pub fn master_pages(&self) -> &[MasterPage] {
        &self.master_pages
    }

    /// The layers drawn for `page`, bottom to top: its master page's layers
    /// (when it uses one), then the page's own.
    pub fn page_layers<'a>(&'a self, page: &'a Page) -> impl Iterator<Item = &'a Layer> + 'a {
        let master = page.master_page.and_then(|id| self.master_page(id));
        master.map(|m| m.layers.as_slice()).unwrap_or(&[]).iter().chain(page.layers())
    }

    /// Get a master page by ID.
    pub fn master_page(&self, id: MasterPageId) -> Option<&MasterPage> {
        self.master_pages.iter().find(|m| m.id == id)
    }

    /// Get a mutable master page by ID.
    pub fn master_page_mut(&mut self, id: MasterPageId) -> Option<&mut MasterPage> {
        self.master_pages.iter_mut().find(|m| m.id == id)
    }

    /// Add a new master page.
    pub fn add_master_page(&mut self, name: impl Into<String>) -> MasterPageId {
        let master = MasterPage::new(name, self.default_page_size, self.default_orientation);
        let id = master.id;
        self.master_pages.push(master);
        id
    }

    /// Apply a master page to a page.
    pub fn apply_master_page(&mut self, page_index: usize, master_id: MasterPageId) -> bool {
        if let Some(page) = self.pages.get_mut(page_index) {
            if self.master_pages.iter().any(|m| m.id == master_id) {
                page.master_page = Some(master_id);
                return true;
            }
        }
        false
    }

    /// Find a shape by ID across all pages.
    pub fn find_shape(&self, id: ObjectId) -> Option<(usize, &Shape)> {
        for (page_idx, page) in self.pages.iter().enumerate() {
            if let Some(shape) = page.get_shape(id) {
                return Some((page_idx, shape));
            }
        }
        None
    }

    /// Find a mutable shape by ID across all pages.
    pub fn find_shape_mut(&mut self, id: ObjectId) -> Option<(usize, &mut Shape)> {
        for (page_idx, page) in self.pages.iter_mut().enumerate() {
            if let Some(shape) = page.get_shape_mut(id) {
                return Some((page_idx, shape));
            }
        }
        None
    }

    /// Get total shape count across all pages.
    pub fn total_shape_count(&self) -> usize {
        self.pages.iter().map(|p| p.shape_count()).sum()
    }
}

#[cfg(test)]
mod group_tests {
    use super::*;
    use repub_geometry::{Point, Rect};
    use crate::shape::{Shape, ShapeKind};

    fn ids(page: &Page) -> Vec<ObjectId> {
        page.layers()[0].shapes().iter().map(|s| s.id()).collect()
    }

    #[test]
    fn grouping_keeps_stacking_order_and_ungrouping_restores_it() {
        let mut page = Page::new(PageSize::Letter, PageOrientation::Portrait);
        let a = page.add_shape(Shape::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0)));
        let b = page.add_shape(Shape::rectangle(Rect::new(50.0, 0.0, 10.0, 10.0)));
        let c = page.add_shape(Shape::rectangle(Rect::new(0.0, 50.0, 10.0, 10.0)));
        let d = page.add_shape(Shape::rectangle(Rect::new(90.0, 90.0, 10.0, 10.0)));

        let g = page.group_shapes(&[c, a]).unwrap();
        // The group takes c's place (above b, below d)
        assert_eq!(ids(&page), vec![b, g, d]);
        match page.get_shape(g).map(|s| &s.kind) {
            Some(ShapeKind::Group(group)) => {
                assert_eq!(group.children.iter().map(|s| s.id()).collect::<Vec<_>>(), vec![a, c]);
                assert_eq!(group.bounds, Rect::new(0.0, 0.0, 10.0, 60.0));
            }
            _ => panic!("group"),
        }
        assert_eq!(page.ungroup(g), vec![a, c]);
        assert_eq!(ids(&page), vec![b, a, c, d]);
        assert!(page.group_shapes(&[a]).is_none(), "one shape doesn't group");
    }

    #[test]
    fn moving_and_resizing_carries_geometry_along() {
        let mut line = Shape::line(Point::new(10.0, 10.0), Point::new(30.0, 10.0));
        line.set_position(Point::new(110.0, 60.0));
        match &line.kind {
            ShapeKind::Line(l) => {
                assert_eq!(l.start, Point::new(110.0, 60.0));
                assert_eq!(l.end, Point::new(130.0, 60.0));
            }
            _ => unreachable!(),
        }

        let mut page = Page::new(PageSize::Letter, PageOrientation::Portrait);
        let a = page.add_shape(Shape::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0)));
        let b = page.add_shape(Shape::rectangle(Rect::new(30.0, 30.0, 10.0, 10.0)));
        let g = page.group_shapes(&[a, b]).unwrap();
        let group = page.get_shape_mut(g).unwrap();
        group.set_bounds(Rect::new(100.0, 100.0, 80.0, 80.0)); // twice the size
        match &group.kind {
            ShapeKind::Group(group) => {
                assert_eq!(group.children[0].bounds(), Rect::new(100.0, 100.0, 20.0, 20.0));
                assert_eq!(group.children[1].bounds(), Rect::new(160.0, 160.0, 20.0, 20.0));
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod duplicate_tests {
    use super::*;
    use repub_geometry::Rect;
    use crate::shape::{Shape, ShapeKind};

    fn linked_id(shape: &Shape) -> Option<ObjectId> {
        match &shape.kind {
            ShapeKind::TextFrame(tf) => tf.linked_frame,
            _ => None,
        }
    }

    #[test]
    fn duplicate_page_copies_shapes_with_new_ids() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let mut a = Shape::text_frame(Rect::new(0.0, 0.0, 10.0, 10.0));
        let b = Shape::text_frame(Rect::new(20.0, 0.0, 10.0, 10.0));
        let (a_id, b_id) = (a.id(), b.id());
        if let ShapeKind::TextFrame(tf) = &mut a.kind {
            tf.linked_frame = Some(b_id);
        }
        let page = &mut doc.pages_mut()[0];
        page.add_shape(a);
        page.add_shape(b);
        page.add_shape(Shape::rectangle(Rect::new(0.0, 50.0, 10.0, 10.0)));

        assert_eq!(doc.duplicate_page(0), Some(1));
        assert_eq!(doc.pages().len(), 2);
        let (orig, copy) = (&doc.pages()[0], &doc.pages()[1]);
        assert_ne!(orig.id, copy.id);
        let ids = |p: &Page| p.layers().iter().flat_map(|l| l.shapes()).map(|s| s.id()).collect::<Vec<_>>();
        assert_eq!(ids(copy).len(), 3);
        assert!(ids(copy).iter().all(|id| !ids(orig).contains(id)), "copies get new ids");
        // The copied link points at the copied frame, not the original
        let shapes: Vec<&Shape> = copy.layers().iter().flat_map(|l| l.shapes()).collect();
        assert_eq!(linked_id(shapes[0]), Some(shapes[1].id()));
        assert_ne!(shapes[0].id(), a_id);
        assert_ne!(shapes[1].id(), b_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Rect;
    use crate::shape::Shape;

    #[test]
    fn test_new_document_has_one_page() {
        let doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        assert_eq!(doc.page_count(), 1);
    }

    #[test]
    fn test_add_page() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        doc.add_page();
        assert_eq!(doc.page_count(), 2);
    }

    #[test]
    fn test_remove_page() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        doc.add_page();
        doc.add_page();
        assert_eq!(doc.page_count(), 3);

        doc.remove_page(1);
        assert_eq!(doc.page_count(), 2);
    }

    #[test]
    fn test_landscape_orientation() {
        let doc = Document::new(PageSize::Letter, PageOrientation::Landscape);
        let page = doc.page(0).unwrap();
        assert_eq!(page.size.width, 792.0);
        assert_eq!(page.size.height, 612.0);
    }

    #[test]
    fn test_add_shape_to_page() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let shape = Shape::rectangle(Rect::new(100.0, 100.0, 200.0, 150.0));
        let page = doc.page_mut(0).unwrap();
        let id = page.add_shape(shape);

        assert_eq!(page.shape_count(), 1);
        assert!(page.get_shape(id).is_some());
    }

    #[test]
    fn test_layer_management() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let page = doc.page_mut(0).unwrap();

        // Should have one default layer
        assert_eq!(page.layers().len(), 1);

        // Add another layer
        let layer_id = page.add_layer("Background");
        assert_eq!(page.layers().len(), 2);

        // Add shape to specific layer
        let shape = Shape::rectangle(Rect::new(0.0, 0.0, 100.0, 100.0));
        page.add_shape_to_layer(layer_id, shape);

        assert_eq!(page.layer(layer_id).unwrap().shape_count(), 1);
    }

    #[test]
    fn test_master_page() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let master_id = doc.add_master_page("Header/Footer");

        // Add shape to master page
        let master = doc.master_page_mut(master_id).unwrap();
        let text = Shape::text_frame_with_text(Rect::new(72.0, 36.0, 468.0, 20.0), "Header");
        master.add_shape(text);

        // Apply to page
        assert!(doc.apply_master_page(0, master_id));
        assert_eq!(doc.page(0).unwrap().master_page, Some(master_id));
    }

    #[test]
    fn test_move_page() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        doc.add_page();
        doc.add_page();

        let page0_id = doc.page(0).unwrap().id;
        let page1_id = doc.page(1).unwrap().id;
        let page2_id = doc.page(2).unwrap().id;

        assert!(doc.move_page(0, 2));

        assert_eq!(doc.page(0).unwrap().id, page1_id);
        assert_eq!(doc.page(1).unwrap().id, page2_id);
        assert_eq!(doc.page(2).unwrap().id, page0_id);
    }

    #[test]
    fn test_move_page_invalid_indices() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        doc.add_page();

        assert!(!doc.move_page(5, 0));
        assert!(!doc.move_page(0, 5));
    }

    #[test]
    fn test_rename_page() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);

        assert!(doc.page(0).unwrap().name.is_none());

        assert!(doc.rename_page(0, Some("Cover".to_string())));
        assert_eq!(doc.page(0).unwrap().name, Some("Cover".to_string()));

        assert!(doc.rename_page(0, None));
        assert!(doc.page(0).unwrap().name.is_none());
    }

    #[test]
    fn test_rename_page_invalid_index() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        assert!(!doc.rename_page(5, Some("Invalid".to_string())));
    }
}
