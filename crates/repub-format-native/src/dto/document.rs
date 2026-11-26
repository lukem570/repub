//! Serde DTOs for the document, pages, layers and master pages, plus the
//! round-trip reconstruction of a [`Document`] from its DTO.
//!
//! The model keeps `Document::pages`, `Page::layers` and `Layer::shapes`
//! private, so loading rebuilds them through the public builder API
//! ([`Document::empty`], [`Document::add_page`], [`Page::add_layer`],
//! [`Layer::add_shape`], and friends).

use repub_geometry::Unit;
use repub_model::document::{DocumentMetadata, DocumentSettings, Guide, Margins, MasterPage, Page};
use repub_model::id::{LayerId, MasterPageId, PageId};
use repub_model::layer::Layer;
use repub_model::{Document, PageOrientation, PageSize};
use serde::{Deserialize, Serialize};

use super::geometry::SizeDto;
use super::shape::ShapeDto;
use super::styles::FillDto;

/// DTO for [`PageSize`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum PageSizeDto {
    Letter,
    Legal,
    Tabloid,
    A3,
    A4,
    A5,
    A6,
    B5,
    Executive,
    BusinessCard,
    IndexCard4x6,
    IndexCard5x7,
    Custom { width: f64, height: f64 },
}

impl From<PageSize> for PageSizeDto {
    fn from(p: PageSize) -> Self {
        match p {
            PageSize::Letter => PageSizeDto::Letter,
            PageSize::Legal => PageSizeDto::Legal,
            PageSize::Tabloid => PageSizeDto::Tabloid,
            PageSize::A3 => PageSizeDto::A3,
            PageSize::A4 => PageSizeDto::A4,
            PageSize::A5 => PageSizeDto::A5,
            PageSize::A6 => PageSizeDto::A6,
            PageSize::B5 => PageSizeDto::B5,
            PageSize::Executive => PageSizeDto::Executive,
            PageSize::BusinessCard => PageSizeDto::BusinessCard,
            PageSize::IndexCard4x6 => PageSizeDto::IndexCard4x6,
            PageSize::IndexCard5x7 => PageSizeDto::IndexCard5x7,
            PageSize::Custom { width, height } => PageSizeDto::Custom { width, height },
        }
    }
}

impl From<PageSizeDto> for PageSize {
    fn from(p: PageSizeDto) -> Self {
        match p {
            PageSizeDto::Letter => PageSize::Letter,
            PageSizeDto::Legal => PageSize::Legal,
            PageSizeDto::Tabloid => PageSize::Tabloid,
            PageSizeDto::A3 => PageSize::A3,
            PageSizeDto::A4 => PageSize::A4,
            PageSizeDto::A5 => PageSize::A5,
            PageSizeDto::A6 => PageSize::A6,
            PageSizeDto::B5 => PageSize::B5,
            PageSizeDto::Executive => PageSize::Executive,
            PageSizeDto::BusinessCard => PageSize::BusinessCard,
            PageSizeDto::IndexCard4x6 => PageSize::IndexCard4x6,
            PageSizeDto::IndexCard5x7 => PageSize::IndexCard5x7,
            PageSizeDto::Custom { width, height } => PageSize::Custom { width, height },
        }
    }
}

/// DTO for [`PageOrientation`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum PageOrientationDto {
    Portrait,
    Landscape,
}

impl From<PageOrientation> for PageOrientationDto {
    fn from(o: PageOrientation) -> Self {
        match o {
            PageOrientation::Portrait => PageOrientationDto::Portrait,
            PageOrientation::Landscape => PageOrientationDto::Landscape,
        }
    }
}

impl From<PageOrientationDto> for PageOrientation {
    fn from(o: PageOrientationDto) -> Self {
        match o {
            PageOrientationDto::Portrait => PageOrientation::Portrait,
            PageOrientationDto::Landscape => PageOrientation::Landscape,
        }
    }
}

/// DTO for [`Unit`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum UnitDto {
    Points,
    Inches,
    Millimeters,
    Centimeters,
    Pixels,
    Picas,
}

impl From<Unit> for UnitDto {
    fn from(u: Unit) -> Self {
        match u {
            Unit::Points => UnitDto::Points,
            Unit::Inches => UnitDto::Inches,
            Unit::Millimeters => UnitDto::Millimeters,
            Unit::Centimeters => UnitDto::Centimeters,
            Unit::Pixels => UnitDto::Pixels,
            Unit::Picas => UnitDto::Picas,
        }
    }
}

impl From<UnitDto> for Unit {
    fn from(u: UnitDto) -> Self {
        match u {
            UnitDto::Points => Unit::Points,
            UnitDto::Inches => Unit::Inches,
            UnitDto::Millimeters => Unit::Millimeters,
            UnitDto::Centimeters => Unit::Centimeters,
            UnitDto::Pixels => Unit::Pixels,
            UnitDto::Picas => Unit::Picas,
        }
    }
}

/// DTO for [`DocumentMetadata`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadataDto {
    /// Title.
    pub title: Option<String>,
    /// Author.
    pub author: Option<String>,
    /// Subject.
    pub subject: Option<String>,
    /// Keywords.
    pub keywords: Vec<String>,
    /// Created timestamp.
    pub created: Option<i64>,
    /// Modified timestamp.
    pub modified: Option<i64>,
}

impl From<DocumentMetadata> for DocumentMetadataDto {
    fn from(m: DocumentMetadata) -> Self {
        Self {
            title: m.title,
            author: m.author,
            subject: m.subject,
            keywords: m.keywords,
            created: m.created,
            modified: m.modified,
        }
    }
}

impl From<DocumentMetadataDto> for DocumentMetadata {
    fn from(m: DocumentMetadataDto) -> Self {
        DocumentMetadata {
            title: m.title,
            author: m.author,
            subject: m.subject,
            keywords: m.keywords,
            created: m.created,
            modified: m.modified,
        }
    }
}

/// DTO for [`DocumentSettings`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSettingsDto {
    /// Display unit.
    pub display_unit: UnitDto,
    /// Show rulers.
    pub show_rulers: bool,
    /// Show guides.
    pub show_guides: bool,
    /// Snap to grid.
    pub snap_to_grid: bool,
    /// Grid spacing.
    pub grid_spacing: f64,
    /// Snap to guides.
    pub snap_to_guides: bool,
    /// Snap to objects.
    pub snap_to_objects: bool,
}

impl From<DocumentSettings> for DocumentSettingsDto {
    fn from(s: DocumentSettings) -> Self {
        Self {
            display_unit: s.display_unit.into(),
            show_rulers: s.show_rulers,
            show_guides: s.show_guides,
            snap_to_grid: s.snap_to_grid,
            grid_spacing: s.grid_spacing,
            snap_to_guides: s.snap_to_guides,
            snap_to_objects: s.snap_to_objects,
        }
    }
}

impl From<DocumentSettingsDto> for DocumentSettings {
    fn from(s: DocumentSettingsDto) -> Self {
        DocumentSettings {
            display_unit: s.display_unit.into(),
            show_rulers: s.show_rulers,
            show_guides: s.show_guides,
            snap_to_grid: s.snap_to_grid,
            grid_spacing: s.grid_spacing,
            snap_to_guides: s.snap_to_guides,
            snap_to_objects: s.snap_to_objects,
        }
    }
}

/// DTO for page [`Margins`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageMarginsDto {
    /// Top.
    pub top: f64,
    /// Right.
    pub right: f64,
    /// Bottom.
    pub bottom: f64,
    /// Left.
    pub left: f64,
}

impl From<Margins> for PageMarginsDto {
    fn from(m: Margins) -> Self {
        Self {
            top: m.top,
            right: m.right,
            bottom: m.bottom,
            left: m.left,
        }
    }
}

impl From<PageMarginsDto> for Margins {
    fn from(m: PageMarginsDto) -> Self {
        Margins {
            top: m.top,
            right: m.right,
            bottom: m.bottom,
            left: m.left,
        }
    }
}

/// DTO for [`Guide`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GuideDto {
    /// Position.
    pub position: f64,
    /// Horizontal flag.
    pub horizontal: bool,
}

impl From<Guide> for GuideDto {
    fn from(g: Guide) -> Self {
        Self {
            position: g.position,
            horizontal: g.horizontal,
        }
    }
}

impl From<GuideDto> for Guide {
    fn from(g: GuideDto) -> Self {
        Guide {
            position: g.position,
            horizontal: g.horizontal,
        }
    }
}

/// DTO for [`Layer`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDto {
    /// Raw layer id.
    pub id: u64,
    /// Name.
    pub name: String,
    /// Visible flag.
    pub visible: bool,
    /// Locked flag.
    pub locked: bool,
    /// Printable flag.
    pub printable: bool,
    /// Shapes.
    pub shapes: Vec<ShapeDto>,
}

impl From<&Layer> for LayerDto {
    fn from(l: &Layer) -> Self {
        Self {
            id: l.id.raw(),
            name: l.name.clone(),
            visible: l.visible,
            locked: l.locked,
            printable: l.printable,
            shapes: l.shapes().iter().cloned().map(Into::into).collect(),
        }
    }
}

impl LayerDto {
    /// Rebuild a [`Layer`] from this DTO via the public builder API.
    fn into_layer(self) -> Layer {
        let mut layer = Layer::new(self.name);
        layer.id = LayerId::from_raw(self.id);
        layer.visible = self.visible;
        layer.locked = self.locked;
        layer.printable = self.printable;
        for shape in self.shapes {
            layer.add_shape(shape.into());
        }
        layer
    }
}

/// DTO for [`Page`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDto {
    /// Raw page id.
    pub id: u64,
    /// Name.
    pub name: Option<String>,
    /// Size.
    pub size: SizeDto,
    /// Margins.
    pub margins: PageMarginsDto,
    /// Master page (raw id).
    pub master_page: Option<u64>,
    /// Layers.
    pub layers: Vec<LayerDto>,
    /// Guides.
    pub guides: Vec<GuideDto>,
    /// Background fill.
    pub background: Option<FillDto>,
}

impl From<&Page> for PageDto {
    fn from(p: &Page) -> Self {
        Self {
            id: p.id.raw(),
            name: p.name.clone(),
            size: p.size.into(),
            margins: p.margins.into(),
            master_page: p.master_page.map(|id| id.raw()),
            layers: p.layers().iter().map(LayerDto::from).collect(),
            guides: p.guides.iter().copied().map(Into::into).collect(),
            background: p.background.clone().map(Into::into),
        }
    }
}

/// DTO for [`MasterPage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPageDto {
    /// Raw master-page id.
    pub id: u64,
    /// Name.
    pub name: String,
    /// Size.
    pub size: SizeDto,
    /// Margins.
    pub margins: PageMarginsDto,
    /// Layers.
    pub layers: Vec<LayerDto>,
    /// Guides.
    pub guides: Vec<GuideDto>,
}

impl From<&MasterPage> for MasterPageDto {
    fn from(m: &MasterPage) -> Self {
        Self {
            id: m.id.raw(),
            name: m.name.clone(),
            size: m.size.into(),
            margins: m.margins.into(),
            layers: m.layers.iter().map(LayerDto::from).collect(),
            guides: m.guides.iter().copied().map(Into::into).collect(),
        }
    }
}

/// DTO for [`Document`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDto {
    /// Metadata.
    pub metadata: DocumentMetadataDto,
    /// Settings.
    pub settings: DocumentSettingsDto,
    /// Default page size.
    pub default_page_size: PageSizeDto,
    /// Default orientation.
    pub default_orientation: PageOrientationDto,
    /// Pages.
    pub pages: Vec<PageDto>,
    /// Master pages.
    pub master_pages: Vec<MasterPageDto>,
}

impl From<&Document> for DocumentDto {
    fn from(d: &Document) -> Self {
        Self {
            metadata: d.metadata.clone().into(),
            settings: d.settings.clone().into(),
            default_page_size: d.default_page_size.into(),
            default_orientation: d.default_orientation.into(),
            pages: d.pages().iter().map(PageDto::from).collect(),
            master_pages: d.master_pages().iter().map(MasterPageDto::from).collect(),
        }
    }
}

impl DocumentDto {
    /// Rebuild a [`Document`] from this DTO.
    ///
    /// Pages, layers and master pages are reconstructed through the model's
    /// public builder API, since their internal collections are private. A
    /// master page is created and then rebuilt by copying content in via a
    /// scratch [`Page`].
    pub fn into_document(self) -> Document {
        let default_size: PageSize = self.default_page_size.into();
        let default_orientation: PageOrientation = self.default_orientation.into();
        let mut doc = Document::empty(default_size, default_orientation);
        doc.metadata = self.metadata.into();
        doc.settings = self.settings.into();

        // Master pages first, so pages can reference them.
        for mp in self.master_pages {
            let id = doc.add_master_page(mp.name.clone());
            let layers: Vec<Layer> = mp.layers.into_iter().map(LayerDto::into_layer).collect();
            let guides: Vec<Guide> = mp.guides.into_iter().map(Into::into).collect();
            if let Some(master) = doc.master_page_mut(id) {
                master.id = MasterPageId::from_raw(mp.id);
                master.name = mp.name;
                master.size = mp.size.into();
                master.margins = mp.margins.into();
                let mut scratch = master.to_page();
                set_page_layers(&mut scratch, layers);
                scratch.guides = guides;
                master.update_from_page(&scratch);
            }
        }

        for pg in self.pages {
            let page = doc.add_page();
            page.id = PageId::from_raw(pg.id);
            page.name = pg.name;
            page.size = pg.size.into();
            page.margins = pg.margins.into();
            page.master_page = pg.master_page.map(MasterPageId::from_raw);
            page.guides = pg.guides.into_iter().map(Into::into).collect();
            page.background = pg.background.map(Into::into);
            let layers: Vec<Layer> = pg.layers.into_iter().map(LayerDto::into_layer).collect();
            set_page_layers(page, layers);
        }

        doc
    }
}

/// Replace a page's layers with `layers` through its public mutable slice and
/// [`Page::add_layer`], since the layer collection itself is private.
fn set_page_layers(page: &mut Page, layers: Vec<Layer>) {
    // A freshly created page/master page has exactly one default layer.
    // Overwrite it with the first loaded layer, then append the rest.
    let mut iter = layers.into_iter();
    if let Some(first) = iter.next() {
        page.layers_mut()[0] = first;
    }
    for layer in iter {
        let new_id = page.add_layer(layer.name.clone());
        if let Some(slot) = page.layer_mut(new_id) {
            *slot = layer;
        }
    }
}
